use super::*;

impl BackupService {
    pub async fn queue_webhook(
        &self,
        id: Uuid,
        expected_webhook: &citadel_primitives::WebhookConfig,
    ) -> Result<(), BackupError> {
        self.ensure_automated_operations().await?;
        self.store.enqueue_webhook(id, expected_webhook).await?;
        self.changed(BackupChange::Policy(id));
        Ok(())
    }

    pub async fn accept_occurrence(
        &self,
        id: Uuid,
        minute: DateTime<Utc>,
    ) -> Result<bool, BackupError> {
        if !self.scheduling_available().await? {
            return Ok(false);
        }
        let accepted = self.store.enqueue_scheduled_backup(id, minute).await?;
        if accepted {
            self.changed(BackupChange::Policy(id));
        }
        Ok(accepted)
    }
}
