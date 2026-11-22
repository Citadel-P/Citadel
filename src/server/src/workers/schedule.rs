//! Noncritical recurring work starts after one period plus a stable subsecond offset.
use std::time::Duration;

pub(crate) fn startup_offset(name: &str) -> Duration {
    let offset = name.bytes().fold(0_u64, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(u64::from(byte))
    }) % 997;
    Duration::from_millis(offset)
}

pub(crate) fn interval(name: &str, period: Duration) -> ObservedInterval {
    let first = tokio::time::Instant::now() + period + startup_offset(name);
    let mut tick = tokio::time::interval_at(first, period);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let job = citadel_runtime::jobs::JobHandle::current();
    if let Some(job) = &job {
        job.policy(format!("Every {} seconds", period.as_secs_f64()), None);
    }
    ObservedInterval {
        inner: tick,
        job,
        next: Some(first),
    }
}

pub(crate) struct ObservedInterval {
    inner: tokio::time::Interval,
    next: Option<tokio::time::Instant>,
    job: Option<citadel_runtime::jobs::JobHandle>,
}
impl ObservedInterval {
    pub fn without_observation(mut self) -> Self {
        self.job = None;
        self
    }
    pub fn next_deadline(&self) -> Option<tokio::time::Instant> {
        self.next
    }

    pub async fn tick(&mut self) -> tokio::time::Instant {
        if let Some(job) = &self.job {
            job.waiting("Scheduled check", self.next);
        }
        let tick = self.inner.tick().await;
        let next = tick + self.inner.period();
        self.next = (next >= tokio::time::Instant::now()).then_some(next);
        tick
    }
    pub fn set_missed_tick_behavior(&mut self, behavior: tokio::time::MissedTickBehavior) {
        self.inner.set_missed_tick_behavior(behavior);
    }
    pub fn reset_after(&mut self, delay: Duration) {
        let next = tokio::time::Instant::now() + delay;
        self.inner.reset_at(next);
        self.next = Some(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn first_expensive_iterations_are_delayed_and_staggered() {
        let now = tokio::time::Instant::now();
        let mut recovery = interval("recovery", Duration::from_secs(60));
        let mut retention = interval("retention", Duration::from_secs(900));
        let mut stats = interval("local-stats", Duration::from_secs(10));
        let first_stats = stats.tick().await;
        let first_recovery = recovery.tick().await;
        let first_retention = retention.tick().await;
        assert!(first_stats >= now + Duration::from_secs(10));
        assert!(first_recovery >= now + Duration::from_secs(60));
        assert!(first_retention >= now + Duration::from_secs(900));
        assert_ne!(
            (first_stats - now).subsec_millis(),
            (first_recovery - now).subsec_millis()
        );
    }
}
