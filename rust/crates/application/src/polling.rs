use std::time::Duration;

/// .NET WorkerPollingDelay: reset after work, back off idle polling to ten seconds.
pub fn worker_poll_delay(current: Duration, minimum: Duration, dispatched: bool) -> Duration {
    if dispatched {
        minimum
    } else {
        current
            .saturating_mul(2)
            .min(Duration::from_secs(10))
            .max(minimum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // HotPathCoordinationTests.WorkerPollingDelay_BacksOffWhenIdleAndResetsAfterDispatch.
    #[test]
    fn idle_backoff_is_bounded_and_dispatch_resets_it() {
        let minimum = Duration::from_secs(2);
        let mut delay = minimum;
        for expected in [4, 8, 10, 10] {
            delay = worker_poll_delay(delay, minimum, false);
            assert_eq!(delay, Duration::from_secs(expected));
        }
        assert_eq!(worker_poll_delay(delay, minimum, true), minimum);
        assert_eq!(
            worker_poll_delay(delay, Duration::from_secs(30), false),
            Duration::from_secs(30)
        );
    }
}
