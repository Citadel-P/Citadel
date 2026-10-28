use super::*;

impl BackupService {
    /// Poll execution and renewal together. Losing ownership cancels execution,
    /// but still awaits its cleanup; no detached renewal or execution task.
    pub(super) async fn with_lease<T>(
        &self,
        repository: Uuid,
        owner: Uuid,
        attempt: Uuid,
        cancellation: &CancellationToken,
        execution: impl std::future::Future<Output = T>,
    ) -> (T, bool) {
        with_renewal(cancellation, execution, || {
            self.store.renew_leases(repository, owner, attempt)
        })
        .await
    }
}

async fn with_renewal<T, F, Fut>(
    cancellation: &CancellationToken,
    execution: impl std::future::Future<Output = T>,
    mut renew: F,
) -> (T, bool)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<bool, BackupError>>,
{
    // Preserve the initial ownership gate before any execution is polled.
    // Subsequent renewals must not stall an operation already in progress.
    if !matches!(
        tokio::time::timeout(std::time::Duration::from_secs(10), renew()).await,
        Ok(Ok(true))
    ) {
        cancellation.cancel();
        return (execution.await, false);
    }
    let renewal = async {
        let period = std::time::Duration::from_secs(30);
        let mut heartbeat = tokio::time::interval_at(tokio::time::Instant::now() + period, period);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            heartbeat.tick().await;
            if !matches!(
                tokio::time::timeout(std::time::Duration::from_secs(10), renew()).await,
                Ok(Ok(true))
            ) {
                break;
            }
        }
    };
    tokio::pin!(execution);
    tokio::select! {
        biased;
        () = renewal => {
            cancellation.cancel();
            (execution.await, false)
        }
        result = &mut execution => (result, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, future::pending, time::Duration};

    #[tokio::test(start_paused = true)]
    async fn minimum_ttl_survives_delayed_heartbeats_for_long_running_work() {
        let cancellation = CancellationToken::new();
        let expires = Cell::new(tokio::time::Instant::now() + Duration::from_secs(90));
        let calls = Cell::new(0);
        let result = with_renewal(
            &cancellation,
            async {
                tokio::time::sleep(Duration::from_secs(600)).await;
                42
            },
            || async {
                // Both lease types have the same enforced minimum; model the shorter one.
                tokio::time::sleep(Duration::from_secs(9)).await;
                let now = tokio::time::Instant::now();
                if now >= expires.get() {
                    return Ok(false);
                }
                calls.set(calls.get() + 1);
                expires.set(now + Duration::from_secs(90));
                Ok(true)
            },
        )
        .await;
        assert_eq!(result, (42, true));
        assert!(calls.get() > 10);
        assert!(!cancellation.is_cancelled());
    }

    #[tokio::test(start_paused = true)]
    async fn stalled_renewal_does_not_stop_execution_or_completion() {
        let calls = Cell::new(0);
        let cancellation = CancellationToken::new();
        let execution = async {
            tokio::time::sleep(Duration::from_secs(31)).await;
            assert_eq!(calls.get(), 2, "renewal must be pending during execution");
            42
        };
        let result = with_renewal(&cancellation, execution, || {
            calls.set(calls.get() + 1);
            let initial = calls.get() == 1;
            async move { if initial { Ok(true) } else { pending().await } }
        })
        .await;
        assert_eq!(result, (42, true));
        assert!(!cancellation.is_cancelled());
    }

    #[tokio::test(start_paused = true)]
    async fn ownership_loss_cancels_execution_and_awaits_cleanup() {
        let cancellation = CancellationToken::new();
        let calls = Cell::new(0);
        let cleaned = Cell::new(false);
        let execution = async {
            cancellation.cancelled().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
            cleaned.set(true);
        };
        let (_, owned) = with_renewal(&cancellation, execution, || {
            calls.set(calls.get() + 1);
            let initial = calls.get() == 1;
            async move { Ok(initial) }
        })
        .await;
        assert!(!owned);
        assert!(cleaned.get());
    }

    #[tokio::test(start_paused = true)]
    async fn renewal_timeout_keeps_polling_execution_until_cancellation_cleanup() {
        let cancellation = CancellationToken::new();
        let progressed = Cell::new(false);
        let calls = Cell::new(0);
        let start = tokio::time::Instant::now();
        let execution = async {
            tokio::time::sleep(Duration::from_secs(31)).await;
            progressed.set(true);
            cancellation.cancelled().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        };
        let (_, owned) = with_renewal(&cancellation, execution, || {
            calls.set(calls.get() + 1);
            let initial = calls.get() == 1;
            async move { if initial { Ok(true) } else { pending().await } }
        })
        .await;
        assert!(!owned);
        assert!(progressed.get());
        assert_eq!(start.elapsed(), Duration::from_secs(41));
    }

    #[tokio::test(start_paused = true)]
    async fn initial_ownership_check_cancels_before_execution_can_start() {
        let cancellation = CancellationToken::new();
        let execution = async {
            assert!(
                cancellation.is_cancelled(),
                "lost ownership must prevent starting work"
            );
        };
        let (_, owned) = with_renewal(&cancellation, execution, || async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok(false)
        })
        .await;
        assert!(!owned);
    }
}
