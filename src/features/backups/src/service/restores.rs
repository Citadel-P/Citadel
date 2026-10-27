use super::*;

impl BackupService {
    pub async fn process_restore(&self, shutdown: &CancellationToken) -> Result<bool, BackupError> {
        let Some(work) = self.claim_restore(shutdown).await? else {
            return Ok(false);
        };
        self.execute_claim(work, shutdown).await
    }

    pub async fn claim_restore(
        &self,
        shutdown: &CancellationToken,
    ) -> Result<Option<ClaimedBackupWork>, BackupError> {
        let Some(tracking) = self.tracking_slot(shutdown).await else {
            return Ok(None);
        };
        Ok(self
            .store
            .claim_restore(Utc::now() - self.stale_after)
            .await?
            .map(|claim| ClaimedBackupWork {
                claim: BackupWorkClaim::Restore(Box::new(claim)),
                _tracking: tracking,
            }))
    }

    pub(super) async fn execute_restore(
        &self,
        claim: RestoreClaim,
        shutdown: &CancellationToken,
    ) -> Result<bool, BackupError> {
        self.changed();
        self.progress(
            claim.run.id,
            "Running",
            "Checking restore permissions and preparing the target volume...",
        );
        let token = shutdown.child_token();
        self.active_restores
            .lock()
            .map_err(poison)?
            .insert(claim.run.id, token.clone());
        let execution = async {
            match self.authorizer.authorize_restore(&claim).await {
                Ok(()) => {
                    self.executor
                        .restore_with_progress(&claim, &token, self.log_progress(claim.run.id))
                        .await
                }
                Err(error) => rejected_restore(error.to_string()),
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
                "Restore ownership was lost. Storage remains reserved until cleanup is verified."
                    .into(),
            );
            result.status = BackupRestoreStatus::Interrupted;
        }
        if let Ok(mut active) = self.active_restores.lock() {
            active.remove(&claim.run.id);
        }
        if owned
            && result.error_code.as_deref() == Some("HostBusy")
            && self.store.defer_restore(&claim).await?
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

            if self.store.finish_restore(&claim, &result).await? {
                return Ok(Some((result.status, result.error_message.clone())));
            }
            match self.store.get_restore(claim.run.id).await {
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
                message.as_deref().unwrap_or("Restore run completed."),
            );
            self.changed();
        }
        Ok(true)
    }
}
