use super::*;

impl BackupService {
    pub async fn process_backup(&self, shutdown: &CancellationToken) -> Result<bool, BackupError> {
        let Some(claim) = self
            .store
            .claim_backup(Utc::now() - self.stale_after)
            .await?
        else {
            return Ok(false);
        };
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
        let authorization = if matches!(claim.run.trigger.as_str(), "Schedule" | "Webhook") {
            self.ensure_automated_operations().await
        } else {
            Ok(())
        };
        let authorization = match authorization {
            Ok(()) => self.authorizer.authorize_backup(&claim).await,
            Err(error) => Err(error),
        };
        let result = match authorization {
            Ok(()) => match async {
                // Persisted policies must obey the same safety contract as new
                // requests; an obsolete consistency option cannot silently run live.
                claim.policy.source.validate_execution()?;
                self.planner.plan(&claim, &token).await
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
                        .map_err(|error| ("SourceBusy", error))
                }
                .await
                {
                    Ok(()) => {
                        self.progress(
                            claim.run.id,
                            "Running",
                            "Source storage reserved. Running Backup and retention...",
                        );
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
                        if let Err(cleanup) = self.planner.cleanup_staging(&plan).await {
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
        };
        if let Ok(mut active) = self.active_backups.lock() {
            active.remove(&claim.run.id);
        }
        let mut delay = std::time::Duration::from_millis(250);
        loop {
            let completion = async {
                if self.store.finish_backup(&claim, &result).await? {
                    return Ok(Some((result.status, result.error_message.clone())));
                }
                match self.store.get_run(claim.run.id).await {
                    Ok(run) if run.status.is_terminal() => {
                        Ok(Some((run.status, run.error_message)))
                    }
                    Ok(_) | Err(BackupError::NotFound) => Ok(None),
                    Err(error) => Err(error),
                }
            };
            let outcome = tokio::select! {
                biased;
                () = shutdown.cancelled() => return Ok(true),
                outcome = completion => outcome,
            };
            match outcome {
                Ok(confirmed) => {
                    if let Some((status, message)) = confirmed {
                        self.progress(
                            claim.run.id,
                            status.as_str(),
                            message.as_deref().unwrap_or("Backup run completed."),
                        );
                        self.changed();
                    }
                    break;
                }
                Err(BackupError::Storage(error)) => {
                    tracing::warn!(%error, run_id=%claim.run.id, "Backup completion pending");
                }
                Err(error) => return Err(error),
            }
            tokio::select! {
                () = shutdown.cancelled() => return Ok(true),
                () = tokio::time::sleep(delay) => {}
            }
            delay = (delay * 2).min(std::time::Duration::from_secs(5));
        }
        Ok(true)
    }
}
