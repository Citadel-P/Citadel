use citadel_runtime::notifications::{NotificationError, NotificationSource, RuntimeNotification};
use std::sync::Arc;
use std::time::Duration;

use citadel_stacks::StackService;
use tokio_util::sync::CancellationToken;

// Apply is bounded to fifteen minutes. A shorter age lets the recovery worker
// race an operation that is still legitimately running and complete its claim
// from a partially observed runtime. Only recover claims that outlived that
// bound (plus a small scheduling margin).
const OBSERVABLE_AFTER: Duration = Duration::from_secs(16 * 60);
const MAXIMUM_BATCH: i64 = 25;
const DRIFT_MONITOR_INTERVAL: Duration = Duration::from_secs(5 * 60);
const DRIFT_MONITOR_BATCH: i64 = 100;
const DRIFT_EVENT_COALESCE_WINDOW: Duration = Duration::from_millis(50);
const DRIFT_EVENT_MAX_PENDING: usize = 256;

pub(crate) async fn stack_updates(
    cancellation: CancellationToken,
    stacks: Arc<StackService>,
    mut completed: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker = super::schedule::interval("stack-updates", Duration::from_secs(5 * 60));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_images = Some(tokio::time::Instant::now());
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            result = completed.changed() => if result.is_err() { return Ok(()); },
            _ = ticker.tick() => {},
        }
        // Coalesce completed syncs before scanning; changes during the scan
        // remain pending. The timer only reconciles stored observations.
        completed.borrow_and_update();
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::StackUpdates.start();
        let images = last_images.is_none_or(|last: tokio::time::Instant| {
            last.elapsed() >= Duration::from_secs(2 * 60 * 60)
        });
        if images {
            last_images = Some(tokio::time::Instant::now());
        }
        // Git checks consume newly synchronized refs; image scans run every two
        // hours. Both query bounded keyset pages, without retaining an inventory.
        if let Err(error) = stacks.run_update_checks(images, &cancellation).await {
            tracing::warn!(%error,"Stack update monitoring failed");
        }
    }
}

pub(crate) async fn stack_webhooks(
    cancellation: CancellationToken,
    stacks: Arc<StackService>,
    mut signals: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    loop {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        let processed = {
            let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::StackWebhooks.start();
            stacks.process_webhooks().await
        };
        match processed {
            Ok(count) if count > 0 => {
                tokio::task::yield_now().await;
            }
            result => {
                if let Err(error) = result {
                    tracing::warn!(%error,"Stack webhook dispatch failed");
                }
                // Durable retries have availableat deadlines. A lost wakeup or
                // a retry becoming ready is recovered within thirty seconds.
                super::notifications::wait(&mut signals, &cancellation, Duration::from_secs(30))
                    .await;
            }
        }
    }
}

pub(crate) async fn stack_operation_reconciliation(
    cancellation: CancellationToken,
    stacks: Arc<StackService>,
    signals: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    let mut wake = super::recovery::RecoveryWake::new("stack-recovery", signals, OBSERVABLE_AFTER);
    while wake.next(&cancellation).await {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::StackRecovery.start();
        match stacks
            .reconcile_stale_operations(OBSERVABLE_AFTER, MAXIMUM_BATCH)
            .await
        {
            Ok(count) if count > 0 => tracing::info!(count, "reconciled Stack operations"),
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "Stack operation reconciliation failed"),
        }
    }
    Ok(())
}

pub(crate) async fn stack_drift_monitor(
    cancellation: CancellationToken,
    stacks: Arc<StackService>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker = super::schedule::interval("stack-drift", DRIFT_MONITOR_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {}
        }
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::StackDrift.start();
        let mut cursor = None;
        loop {
            match stacks.monitor_drift(cursor, DRIFT_MONITOR_BATCH).await {
                Ok(result) => {
                    cursor = result.next_cursor;
                    if result.reconciled > 0 {
                        tracing::info!(
                            checked = result.checked,
                            reconciled = result.reconciled,
                            "reconciled Stack drift"
                        );
                    }
                    let last_page = result.checked < DRIFT_MONITOR_BATCH as usize;
                    for failure in result.failures {
                        tracing::warn!(
                            stack_id = %failure.stack_id,
                            error = %failure.message,
                            "Stack drift monitoring failed"
                        );
                    }
                    if last_page || cancellation.is_cancelled() {
                        break;
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "Stack drift monitor query failed");
                    break;
                }
            }
        }
    }
}

/// The drift monitor consumes stop/pause events. Notifications are
/// transactional and originate from the shared Local/Direct/Edge event store.
pub(crate) async fn event_drift(
    cancellation: CancellationToken,
    stacks: Arc<StackService>,
    mut listener: Box<dyn NotificationSource>,
) -> Result<(), std::convert::Infallible> {
    loop {
        let result = async {
            loop {
                let Some(pending) = receive_drift_batch(listener.as_mut(), &cancellation).await?
                else {
                    return Ok::<(), NotificationError>(());
                };
                for id in pending {
                    if let Err(error) = stacks.monitor_container_event(id).await {
                        tracing::warn!(%error, %id, "Event-triggered Stack drift repair failed");
                    }
                }
            }
        }
        .await;
        if cancellation.is_cancelled() {
            return Ok(());
        }
        if let Err(error) = result {
            tracing::warn!(%error, "Stack drift event listener failed");
        }
        tokio::select! { ()=cancellation.cancelled()=>return Ok(()), _=tokio::time::sleep(Duration::from_secs(2))=>{} }
    }
}

async fn receive_drift_batch(
    listener: &mut dyn NotificationSource,
    cancellation: &CancellationToken,
) -> Result<Option<std::collections::BTreeSet<uuid::Uuid>>, NotificationError> {
    let first = tokio::select! {
        () = cancellation.cancelled() => return Ok(None),
        notification = listener.recv() => notification?,
    };
    let mut pending = std::collections::BTreeSet::new();
    if let RuntimeNotification::StackDrift(id) = first {
        pending.insert(id);
    }
    let deadline = tokio::time::Instant::now() + DRIFT_EVENT_COALESCE_WINDOW;
    while pending.len() < DRIFT_EVENT_MAX_PENDING {
        tokio::select! {
            () = cancellation.cancelled() => return Ok(None),
            _ = tokio::time::sleep_until(deadline) => break,
            notification = listener.recv() => {
                let notification = notification?;
                if let RuntimeNotification::StackDrift(id) = notification { pending.insert(id); }
            }
        }
    }
    Ok(Some(pending))
}

#[cfg(test)]
mod drift_event_tests {
    use super::*;
    struct Events(std::collections::VecDeque<RuntimeNotification>);
    impl NotificationSource for Events {
        fn recv(
            &mut self,
        ) -> futures_util::future::BoxFuture<'_, Result<RuntimeNotification, NotificationError>>
        {
            Box::pin(async move {
                match self.0.pop_front() {
                    Some(event) => Ok(event),
                    None => std::future::pending().await,
                }
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn drift_notifications_are_deduplicated_and_pending_work_is_bounded() {
        let first = uuid::Uuid::now_v7();
        let mut events = Events(std::collections::VecDeque::from([
            RuntimeNotification::StackDrift(first),
            RuntimeNotification::StackDrift(first),
            RuntimeNotification::Resync,
        ]));
        for _ in 0..DRIFT_EVENT_MAX_PENDING {
            events
                .0
                .push_back(RuntimeNotification::StackDrift(uuid::Uuid::now_v7()));
        }
        let cancel = CancellationToken::new();
        let batch = receive_drift_batch(&mut events, &cancel)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(batch.len(), DRIFT_EVENT_MAX_PENDING);
        assert!(batch.contains(&first));
        assert_eq!(
            events.0.len(),
            1,
            "overflow remains available for the next batch"
        );
        let start = tokio::time::Instant::now();
        assert_eq!(
            receive_drift_batch(&mut events, &cancel)
                .await
                .unwrap()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            tokio::time::Instant::now() - start,
            DRIFT_EVENT_COALESCE_WINDOW
        );
        cancel.cancel();
        assert!(
            receive_drift_batch(&mut events, &cancel)
                .await
                .unwrap()
                .is_none()
        );
    }
}
