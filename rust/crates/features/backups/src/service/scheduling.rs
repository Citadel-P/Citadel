use super::*;

impl BackupService {
    pub async fn queue_webhook(
        &self,
        id: Uuid,
        expected_webhook: &Value,
    ) -> Result<(), BackupError> {
        self.ensure_automated_operations().await?;
        self.store.enqueue_webhook(id, expected_webhook).await?;
        self.changed();
        Ok(())
    }

    pub async fn queue_due_scheduled(&self, now: DateTime<Utc>) -> Result<usize, BackupError> {
        match self.ensure_automated_operations().await {
            Ok(()) => {}
            Err(BackupError::LicenseRequired) => return Ok(0),
            Err(error) => return Err(error),
        }
        let minute = now
            .with_second(0)
            .and_then(|value| value.with_nanosecond(0))
            .ok_or_else(|| BackupError::Storage("Could not normalize scheduler time.".into()))?;
        let mut queued = 0;
        for policy in self.store.list_scheduled_policies().await? {
            if schedule_is_due(
                policy.cron.as_deref(),
                policy.time_zone.as_deref().unwrap_or("UTC"),
                minute,
            ) && self
                .store
                .enqueue_scheduled_backup(policy.id, minute)
                .await?
            {
                queued += 1;
                self.changed();
            }
        }
        Ok(queued)
    }
}
