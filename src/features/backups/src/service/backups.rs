use super::*;

impl BackupService {
    pub async fn process_backup(&self, shutdown: &CancellationToken) -> Result<bool, BackupError> {
        let Some(work) = self.claim_backup(shutdown).await? else {
            return Ok(false);
        };
        self.execute_claim(work, shutdown).await
    }

    pub async fn claim_backup(
        &self,
        shutdown: &CancellationToken,
    ) -> Result<Option<ClaimedBackupWork>, BackupError> {
        let Some(tracking) = self.tracking_slot(shutdown).await else {
            return Ok(None);
        };
        Ok(self
            .store
            .claim_backup(Utc::now() - self.stale_after)
            .await?
            .map(|claim| ClaimedBackupWork {
                claim: BackupWorkClaim::Backup(Box::new(claim)),
                _tracking: tracking,
            }))
    }

    pub(super) async fn execute_backup(
        &self,
        claim: BackupClaim,
        shutdown: &CancellationToken,
    ) -> Result<bool, BackupError> {
        self.changed();
        let token = shutdown.child_token();
        self.progress(
            claim.run.id,
            "Running",
            "Checking Backup permissions and resolving source storage...",
        );
        self.active_backups
            .lock()
            .map_err(poison)?
            .insert(claim.run.id, token.clone());
        let execution = async {
            let authorization = if matches!(claim.run.trigger.as_str(), "Schedule" | "Webhook") {
                self.ensure_automated_operations().await
            } else {
                Ok(())
            };
            let authorization = match authorization {
                Ok(()) => self.authorizer.authorize_backup(&claim).await,
                Err(error) => Err(error),
            };
            match authorization {
                Ok(()) => match async {
                    // Persisted policies must obey the same safety contract as new
                    // requests; an obsolete consistency option cannot silently run live.
                    claim.policy.source.validate_execution()?;
                    if token.is_cancelled() {
                        return Err(BackupError::Conflict("Backup was cancelled.".into()));
                    }
                    match &claim.source_plan {
                        Some(plan) => Ok(plan.clone()),
                        None => self.planner.plan(&claim, &token).await,
                    }
                }
                .await
                {
                    Ok(plan) => match async {
                        self.authorizer
                            .authorize_plan(&claim, &plan)
                            .await
                            .map_err(|error| ("AuthorizationRevoked", error))?;
                        self.store
                            .prepare_backup_items(&claim, &plan)
                            .await
                            .map_err(|error| {
                                (
                                    if matches!(error, BackupError::CleanupPending) {
                                        "CleanupPending"
                                    } else if matches!(error, BackupError::Conflict(_)) {
                                        "SourceBusy"
                                    } else {
                                        "PreparationFailed"
                                    },
                                    error,
                                )
                            })
                    }
                    .await
                    {
                        Ok(()) => {
                            self.progress(
                                claim.run.id,
                                "Running",
                                "Source storage reserved. Running Backup and retention...",
                            );
                            if token.is_cancelled() {
                                return failed_before_execution(
                                    "Cancelled",
                                    "Backup was cancelled.".into(),
                                );
                            }
                            self.executor
                                .backup_with_progress(
                                    &claim,
                                    &plan,
                                    &token,
                                    self.log_progress(claim.run.id),
                                )
                                .await
                        }
                        Err((code, error)) => {
                            let mut message = error.to_string();
                            if code != "SourceBusy"
                                && let Err(cleanup) = self.planner.cleanup_staging(&plan).await
                            {
                                message.push_str(&format!(
                                    "; recovery staging cleanup also failed: {cleanup}"
                                ));
                            }
                            failed_before_execution(code, message)
                        }
                    },
                    Err(error) => failed_before_execution("SourceUnavailable", error.to_string()),
                },
                Err(error) => rejected_backup(error.to_string()),
            }
        };
        let (mut result, owned) = self
            .with_lease(
                claim.repository.id,
                claim.run.id,
                claim.attempt_id,
                &token,
                execution,
            )
            .await;
        if !owned {
            result.error_code = Some("LeaseLost".into());
            result.error_message = Some(
                "Backup ownership was lost. Storage remains reserved until cleanup is verified."
                    .into(),
            );
            result.status = BackupRunStatus::Interrupted;
        }
        if let Ok(mut active) = self.active_backups.lock() {
            active.remove(&claim.run.id);
        }
        if owned
            && matches!(
                result.error_code.as_deref(),
                Some("SourceBusy" | "HostBusy")
            )
            && self.store.defer_backup(&claim).await?
        {
            self.changed();
            return Ok(true);
        }
        let confirmed = completion::persist(claim.run.id, shutdown, || async {
            // Keep ownership alive while retrying persistence of this same
            // result. Never re-run the external operation.
            if owned {
                let _ = self
                    .store
                    .renew_leases(claim.repository.id, claim.run.id, claim.attempt_id)
                    .await;
            }

            if self.store.finish_backup(&claim, &result).await? {
                return Ok(Some((result.status, result.error_message.clone())));
            }
            match self.store.get_run(claim.run.id).await {
                Ok(run) if run.status.is_terminal() => Ok(Some((run.status, run.error_message))),
                Ok(_) | Err(BackupError::NotFound) => Ok(None),
                Err(error) => Err(error),
            }
        })
        .await?;
        if let Some((status, message)) = confirmed {
            self.progress(
                claim.run.id,
                status.as_str(),
                message.as_deref().unwrap_or("Backup run completed."),
            );
            self.changed();
        }
        Ok(true)
    }
}
