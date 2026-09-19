//! Noncritical recurring work starts after one period plus a stable subsecond offset.
use std::time::Duration;

pub(super) fn interval(name: &str, period: Duration) -> tokio::time::Interval {
    let offset = name.bytes().fold(0_u64, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(u64::from(byte))
    }) % 997;
    let mut tick = tokio::time::interval_at(
        tokio::time::Instant::now() + period + Duration::from_millis(offset),
        period,
    );
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tick
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
