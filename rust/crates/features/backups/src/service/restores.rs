use super::*;

impl BackupService {
    pub async fn process_restore(&self, shutdown: &CancellationToken) -> Result<bool, BackupError> {
        let Some(claim) = self
            .store
            .claim_restore(Utc::now() - self.stale_after)
            .await?
        else {
            return Ok(false);
        };
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
        let result = match self.authorizer.authorize_restore(&claim).await {
            Ok(()) => {
                self.executor
                    .restore_with_progress(&claim, &token, self.log_progress(claim.run.id))
                    .await
            }
            Err(error) => rejected_restore(error.to_string()),
        };
        if let Ok(mut active) = self.active_restores.lock() {
            active.remove(&claim.run.id);
        }
        self.store.finish_restore(&claim, &result).await?;
        self.progress(
            claim.run.id,
            result.status,
            result
                .error_message
                .as_deref()
                .unwrap_or("Restore run completed."),
        );
        self.changed();
        Ok(true)
    }
}
