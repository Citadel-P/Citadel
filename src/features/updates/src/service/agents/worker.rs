use super::AgentUpdateService;
use crate::UpdateError;
use chrono::{Timelike, Utc};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

impl AgentUpdateService {
    pub fn work_changes(&self) -> tokio::sync::watch::Receiver<()> {
        self.work_changed.subscribe()
    }

    /// Probe only scheduling/ownership columns until durable work needs full state.
    pub async fn poll(&self, cancellation: &CancellationToken) -> Result<Duration, UpdateError> {
        let work = self.repository.pending_work().await?;
        let now = Utc::now();
        let minute = now
            .with_second(0)
            .and_then(|value| value.with_nanosecond(0))
            .ok_or_else(|| UpdateError::Storage("Invalid scheduler clock.".into()))?;
        let due = self.updates_allowed
            && work.schedule_enabled
            && work.last_scheduled_minute.is_none_or(|last| last < minute)
            && citadel_primitives::schedule::schedule_is_due(
                Some(&work.cron),
                &work.time_zone,
                minute,
            );
        if work.active || (due && self.entitlements.automated_operations().await?) {
            self.tick(cancellation).await?;
            return Ok(Duration::from_secs(5));
        }
        // Check at minute boundaries even when a wakeup was missed. Persisted
        // occurrence uniqueness, not this hint, remains the scheduling authority.
        Ok(
            Duration::from_millis(60_000 - now.timestamp_millis().rem_euclid(60_000) as u64)
                .min(Duration::from_secs(30)),
        )
    }
}
