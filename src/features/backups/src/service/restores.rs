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
        let mut delay = std::time::Duration::from_millis(250);
        loop {
            let completion = async {
                if self.store.finish_restore(&claim, &result).await? {
                    return Ok(Some((result.status, result.error_message.clone())));
                }
                match self.store.get_restore(claim.run.id).await {
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
                            message.as_deref().unwrap_or("Restore run completed."),
                        );
                        self.changed();
                    }
                    break;
                }
                Err(BackupError::Storage(error)) => {
                    tracing::warn!(%error, run_id=%claim.run.id, "Restore completion pending");
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
