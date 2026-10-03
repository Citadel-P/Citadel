use super::*;
impl AutomationService {
    pub async fn queue_webhook(
        &self,
        id: Uuid,
        expected_webhook: &WebhookConfig,
        args: &Value,
    ) -> Result<(), AutomationError> {
        let action = self.store.get(id).await?;
        self.options.validate_execution(action.timeout_seconds)?;
        self.ensure_paid_trigger().await?;
        self.store
            .enqueue_webhook(id, expected_webhook, args)
            .await?;
        self.changed();
        Ok(())
    }
    pub async fn process_one(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<bool, AutomationError> {
        if !self.options.enabled {
            return Ok(false);
        }
        let Ok(_permit) = self.slots.clone().try_acquire_owned() else {
            return Ok(false);
        };
        let stale_before = Utc::now()
            - chrono::Duration::from_std(self.stale_after)
                .map_err(|error| AutomationError::Storage(error.to_string()))?;
        let Some(claim) = self.store.claim_next(stale_before).await? else {
            return Ok(false);
        };
        self.execute_claim(&claim, cancellation, None).await?;
        Ok(true)
    }
}

impl AutomationService {
    pub async fn queue_due_scheduled(&self, now: DateTime<Utc>) -> Result<usize, AutomationError> {
        if !self.options.enabled {
            return Ok(0);
        }
        match self.ensure_paid_trigger().await {
            Ok(()) => {}
            Err(AutomationError::LicenseRequired) => return Ok(0),
            Err(error) => return Err(error),
        }
        let minute = now
            .with_second(0)
            .and_then(|value| value.with_nanosecond(0))
            .ok_or_else(|| {
                AutomationError::Storage("Could not normalize scheduler time.".to_owned())
            })?;
        let mut queued = 0;
        let mut after = None;
        loop {
            let actions = self.store.list_scheduled(after, 64).await?;
            if actions.is_empty() {
                break;
            }
            for action in actions {
                after = Some(action.id);
                if self
                    .options
                    .validate_timeout(action.timeout_seconds)
                    .is_err()
                {
                    tracing::warn!(action_id=%action.id, "Scheduled Action exceeds the configured timeout limit");
                    continue;
                }
                if !schedule_is_due(
                    action.schedule_cron.as_deref(),
                    &action.schedule_time_zone,
                    minute,
                ) {
                    continue;
                }
                match self.store.enqueue_scheduled(action.id, minute).await {
                    Ok(Some(_)) => {
                        queued += 1;
                        self.changed();
                    }
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(%error, action_id=%action.id, "Scheduled Action could not be queued")
                    }
                }
            }
        }
        Ok(queued)
    }
}
