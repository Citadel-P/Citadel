use super::*;
impl ScriptService {
    pub async fn queue_webhook(
        &self,
        id: Uuid,
        expected_webhook: &WebhookConfig,
        args: &Value,
    ) -> Result<(), ScriptError> {
        let script = self.store.get(id).await?;
        self.options.validate_execution(script.timeout_seconds)?;
        self.ensure_automated_operations().await?;
        self.store
            .enqueue_webhook(id, expected_webhook, args)
            .await?;
        self.changed(Some(id));
        Ok(())
    }
}

impl ScriptService {
    pub async fn accept_occurrence(
        &self,
        id: Uuid,
        minute: DateTime<Utc>,
    ) -> Result<bool, ScriptError> {
        if !self.options.enabled {
            return Ok(false);
        }
        self.ensure_automated_operations().await?;
        let accepted = self
            .store
            .enqueue_scheduled(id, minute, self.options.max_timeout_seconds)
            .await?
            .is_some();
        if accepted {
            self.changed(Some(id));
        }
        Ok(accepted)
    }
}
