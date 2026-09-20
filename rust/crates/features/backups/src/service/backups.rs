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
            "Checking Backup permissions and resolving source volumes...",
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
            Ok(()) => match self.planner.plan(&claim, &token).await {
                Ok(plan) => match self.store.prepare_backup_items(&claim, &plan).await {
                    Ok(()) => {
                        self.progress(
                            claim.run.id,
                            "Running",
                            "Source volumes reserved. Running Backup and retention...",
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
                    Err(error) => {
                        let mut message = error.to_string();
                        if let Some(directory) = plan.local_directory.as_deref()
                            && let Err(cleanup) = tokio::fs::remove_dir_all(directory).await
                            && cleanup.kind() != std::io::ErrorKind::NotFound
                        {
                            message.push_str(&format!(
                                "; recovery staging cleanup also failed: {cleanup}"
                            ));
                        }
                        failed_before_execution("SourceBusy", message)
                    }
                },
                Err(error) => failed_before_execution("SourceUnavailable", error.to_string()),
            },
            Err(error) => rejected_backup(error.to_string()),
        };
        if let Ok(mut active) = self.active_backups.lock() {
            active.remove(&claim.run.id);
        }
        self.store.finish_backup(&claim, &result).await?;
        self.progress(
            claim.run.id,
            result.status,
            result
                .error_message
                .as_deref()
                .unwrap_or("Backup run completed."),
        );
        self.changed();
        Ok(true)
    }
}
