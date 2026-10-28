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
    async fn claim_next(
        &self,
        stale_before: DateTime<Utc>,
        remote: bool,
    ) -> Result<Option<ScriptRunClaim>, ScriptError> {
        let result = self.store.claim_next(stale_before, remote).await?;
        for script_id in result.recovered_scripts {
            self.changed(Some(script_id));
        }
        Ok(result.claim)
    }
    pub async fn process_one(&self, cancellation: &CancellationToken) -> Result<bool, ScriptError> {
        if !self.options.enabled {
            return Ok(false);
        }
        let stale_before = Utc::now()
            - chrono::Duration::from_std(self.stale_after)
                .map_err(|error| ScriptError::Storage(error.to_string()))?;
        if let Ok(_permit) = self.slots.clone().try_acquire_owned()
            && let Some(claim) = self.claim_next(stale_before, false).await?
        {
            self.execute_claim(&claim, cancellation, Some(&self.progress))
                .await?;
            return Ok(true);
        }
        let Ok(_permit) = self.remote_slots.clone().try_acquire_owned() else {
            return Ok(false);
        };
        let Some(claim) = self.claim_next(stale_before, true).await? else {
            return Ok(false);
        };
        self.execute_claim(&claim, cancellation, Some(&self.progress))
            .await?;
        Ok(true)
    }
}

impl ScriptService {
    pub async fn queue_due_scheduled(&self, now: DateTime<Utc>) -> Result<usize, ScriptError> {
        if !self.options.enabled {
            return Ok(0);
        }
        match self.ensure_automated_operations().await {
            Ok(()) => {}
            Err(ScriptError::LicenseRequired) => return Ok(0),
            Err(error) => return Err(error),
        }
        let minute = now
            .with_second(0)
            .and_then(|value| value.with_nanosecond(0))
            .ok_or_else(|| {
                ScriptError::Storage("Could not normalize scheduler time.".to_owned())
            })?;
        let mut queued = 0;
        let mut after = None;
        loop {
            let scripts = self.store.list_scheduled(after, 64).await?;
            if scripts.is_empty() {
                break;
            }
            for script in scripts {
                after = Some(script.id);
                if self
                    .options
                    .validate_timeout(script.timeout_seconds)
                    .is_err()
                {
                    tracing::warn!(script_id=%script.id, "Scheduled Script exceeds the configured timeout limit");
                    continue;
                }
                if !schedule_is_due(
                    script.schedule_cron.as_deref(),
                    &script.schedule_time_zone,
                    minute,
                ) {
                    continue;
                }
                match self.store.enqueue_scheduled(script.id, minute).await {
                    Ok(Some(_)) => {
                        queued += 1;
                        self.changed(Some(script.id));
                    }
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(%error, script_id=%script.id, "Scheduled Script could not be queued")
                    }
                }
            }
        }
        Ok(queued)
    }
}
