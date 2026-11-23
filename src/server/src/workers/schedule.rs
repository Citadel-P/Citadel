//! Observed timers for fixed-period work and independent recovery deadlines.
use std::time::Duration;

pub(crate) fn startup_offset(name: &str) -> Duration {
    let offset = name.bytes().fold(0_u64, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(u64::from(byte))
    }) % 997;
    Duration::from_millis(offset)
}

pub(crate) fn interval(name: &str, period: Duration) -> ObservedInterval {
    observed_interval(period, period + startup_offset(name))
}

/// Spread independent sweeps across their first period. Subsequent ticks and
/// explicit completion-relative rests retain the worker's normal cadence.
pub(crate) fn phased_interval(name: &str, period: Duration) -> ObservedInterval {
    observed_interval(period, first_phase(name, period))
}

pub(super) fn first_phase(name: &str, period: Duration) -> Duration {
    let grace = period / 10;
    let hash = name.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    let window_ms = (period - grace).as_millis();
    let offset_ms = u128::from(hash) % window_ms.max(1);
    grace + Duration::from_millis(offset_ms as u64)
}

fn observed_interval(period: Duration, first_delay: Duration) -> ObservedInterval {
    let first = tokio::time::Instant::now() + first_delay;
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
    async fn heavy_sweeps_have_distinct_first_deadlines_and_keep_their_cadence() {
        let start = tokio::time::Instant::now();
        let period = Duration::from_secs(300);
        let mut phases = [
            "statistics-compaction",
            "stack-drift",
            "job-history-maintenance",
            "lifecycle-deployments",
        ]
        .map(|name| phased_interval(name, period).next_deadline().unwrap() - start);
        phases.sort();
        assert!(phases[0] >= Duration::from_secs(30) && phases[3] <= period);
        assert!(
            phases
                .windows(2)
                .all(|w| w[1] - w[0] >= Duration::from_secs(45))
        );
        let mut timer = phased_interval("statistics-compaction", period);
        let first = timer.tick().await;
        assert_eq!(timer.tick().await - first, period);
        for delay in [60, 300] {
            tokio::time::advance(Duration::from_secs(7)).await;
            let completed = tokio::time::Instant::now();
            timer.reset_after(Duration::from_secs(delay));
            assert_eq!(timer.tick().await - completed, Duration::from_secs(delay));
        }
    }

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
