use std::sync::Arc;
use std::time::Duration;

use citadel_alerts::{AlertDeliveryService, AlertError};
use tokio_util::sync::CancellationToken;

pub async fn deliveries(
    admission: citadel_runtime::admission::AdmissionGate,
    cancellation: CancellationToken,
    service: Arc<AlertDeliveryService>,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), AlertError> {
    loop {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        let result = {
            let Some(_admission) = admission.enter("Alert deliveries", &cancellation).await else {
                return Ok(());
            };
            service.process_one(&cancellation).await
        };
        let failed = match result {
            Ok(true) => continue,
            Ok(false) => false,
            Err(error) => {
                tracing::error!(%error, "Alert delivery worker iteration failed");
                true
            }
        };
        let fallback = if cancellation.is_cancelled() {
            return Ok(());
        } else {
            Duration::from_secs(60)
        };
        let sleep = match service.next_delivery_deadline().await {
            Ok(Some(deadline)) => idle_delay(
                deadline
                    .signed_duration_since(chrono::Utc::now())
                    .to_std()
                    .unwrap_or(Duration::ZERO),
                fallback,
            ),
            Ok(None) => fallback,
            Err(error) => {
                tracing::warn!(%error, "Could not read alert delivery deadline; using recovery fallback");
                Duration::from_secs(5)
            }
        };
        if failed {
            // Failure backoff cannot be bypassed by an overdue row or repeated notifications.
            tokio::select! { () = cancellation.cancelled() => return Ok(()), _ = tokio::time::sleep(Duration::from_secs(5)) => {} }
        } else if sleep <= Duration::from_millis(250) {
            // Repeated notifications for locked work cannot bypass the idle floor.
            tokio::select! { () = cancellation.cancelled() => return Ok(()), _ = tokio::time::sleep(sleep) => {} }
        } else {
            wait_for_work(&mut wake, &cancellation, sleep).await;
        }
    }
}

fn idle_delay(until_due: Duration, fallback: Duration) -> Duration {
    until_due.min(fallback).max(Duration::from_millis(250))
}

async fn wait_for_work(
    wake: &mut tokio::sync::watch::Receiver<()>,
    cancellation: &CancellationToken,
    timeout: Duration,
) {
    tokio::select! {
        () = cancellation.cancelled() => {},
        _ = tokio::time::sleep(timeout) => {},
        changed = wake.changed() => if changed.is_err() {
            tokio::select! { ()=cancellation.cancelled()=>{}, _=tokio::time::sleep(timeout)=>{} }
        },
    }
}

/// Notifications wake a durable queue; startup and fallback recover missed wakes.
pub(crate) async fn observations(
    cancel: CancellationToken,
    alerts: Arc<dyn citadel_alerts::PendingAlertObservations>,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    let owner = uuid::Uuid::now_v7();
    loop {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let result = tokio::select! { () = cancel.cancelled() => return Ok(()), result = alerts.process_pending_observation(owner) => result };
        match result {
            Ok(true) => continue,
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(%error, "Job alert evaluation failed; observation retained");
                tokio::select! { () = cancel.cancelled() => return Ok(()), _ = tokio::time::sleep(Duration::from_secs(5)) => {} }
            }
        }
        super::notifications::wait(&mut wake, &cancel, Duration::from_secs(30)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overdue_locked_work_keeps_a_minimum_idle_delay() {
        assert_eq!(
            idle_delay(Duration::ZERO, Duration::from_secs(60)),
            Duration::from_millis(250)
        );
        assert_eq!(
            idle_delay(Duration::from_secs(8), Duration::from_secs(60)),
            Duration::from_secs(8)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn idle_delivery_waits_for_signal_or_deadline() {
        let (_sender, mut wake) = tokio::sync::watch::channel(());
        let cancellation = CancellationToken::new();
        let started = tokio::time::Instant::now();
        let waiter = tokio::spawn({
            let cancellation = cancellation.clone();
            async move { wait_for_work(&mut wake, &cancellation, Duration::from_secs(30)).await }
        });
        tokio::task::yield_now().await;
        assert!(
            !waiter.is_finished(),
            "empty queue must wait instead of polling"
        );
        tokio::time::advance(Duration::from_secs(29)).await;
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());
        tokio::time::advance(Duration::from_secs(1)).await;
        waiter.await.unwrap();
        assert_eq!(
            tokio::time::Instant::now() - started,
            Duration::from_secs(30)
        );

        let (sender, mut wake) = tokio::sync::watch::channel(());
        let waiter = tokio::spawn({
            let cancellation = cancellation.clone();
            async move { wait_for_work(&mut wake, &cancellation, Duration::from_secs(60)).await }
        });
        tokio::task::yield_now().await;
        sender.send_modify(|_| {});
        waiter.await.unwrap();
        assert_eq!(
            tokio::time::Instant::now() - started,
            Duration::from_secs(30)
        );
    }
    struct PendingObservations {
        remaining: std::sync::atomic::AtomicUsize,
        attempts: tokio::sync::mpsc::UnboundedSender<bool>,
    }

    impl citadel_alerts::PendingAlertObservations for PendingObservations {
        fn process_pending_observation(
            &self,
            _: uuid::Uuid,
        ) -> futures_util::future::BoxFuture<'_, Result<bool, AlertError>> {
            Box::pin(async move {
                use std::sync::atomic::Ordering;
                let found = self
                    .remaining
                    .try_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                        remaining.checked_sub(1)
                    })
                    .is_ok();
                self.attempts.send(found).unwrap();
                Ok(found)
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn observations_drain_on_startup_and_coalesced_wake_then_recover_without_hub() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (attempts, mut received) = tokio::sync::mpsc::unbounded_channel();
        let store = Arc::new(PendingObservations {
            remaining: AtomicUsize::new(2),
            attempts,
        });
        let (sender, wake) = tokio::sync::watch::channel(());
        let token = CancellationToken::new();
        let worker = tokio::spawn(observations(token.clone(), store.clone(), wake));
        let start = tokio::time::Instant::now();
        for expected in [true, true, false] {
            assert_eq!(received.recv().await, Some(expected));
        }
        assert_eq!(tokio::time::Instant::now(), start);
        store.remaining.store(2, Ordering::SeqCst);
        for _ in 0..1000 {
            sender.send_modify(|_| {});
        }
        for expected in [true, true, false] {
            assert_eq!(received.recv().await, Some(expected));
        }
        assert_eq!(tokio::time::Instant::now(), start);
        // Losing the shared listener must neither lose durable work nor spin.
        drop(sender);
        store.remaining.store(1, Ordering::SeqCst);
        assert_eq!(received.recv().await, Some(true));
        assert_eq!(received.recv().await, Some(false));
        assert_eq!(tokio::time::Instant::now() - start, Duration::from_secs(30));
        token.cancel();
        worker.await.unwrap().unwrap();
        assert!(
            received.try_recv().is_err(),
            "shutdown must not start another queue scan"
        );
    }
}
