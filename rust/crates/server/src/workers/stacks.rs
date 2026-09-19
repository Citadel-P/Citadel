use std::sync::Arc;
use std::time::Duration;

use citadel_stacks::StackService;
use tokio_util::sync::CancellationToken;

const RECONCILIATION_INTERVAL: Duration = Duration::from_secs(10);
// Apply is bounded to fifteen minutes. A shorter age lets the recovery worker
// race an operation that is still legitimately running and complete its claim
// from a partially observed runtime. Only recover claims that outlived that
// bound (plus a small scheduling margin).
const OBSERVABLE_AFTER: Duration = Duration::from_secs(16 * 60);
const MAXIMUM_BATCH: i64 = 25;
const DRIFT_MONITOR_INTERVAL: Duration = Duration::from_secs(5 * 60);
const DRIFT_MONITOR_BATCH: i64 = 100;

pub(super) async fn stack_updates(
    cancellation: CancellationToken,
    stacks: Arc<StackService>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker = super::schedule::interval("stack-updates", Duration::from_secs(30));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_images = Some(tokio::time::Instant::now());
    loop {
        tokio::select! { ()=cancellation.cancelled()=>return Ok(()), _=ticker.tick()=>{} }
        let _iteration = citadel_application::runtime_metrics::RuntimeWork::StackUpdates.start();
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

pub(super) async fn stack_webhooks(
    cancellation: CancellationToken,
    stacks: Arc<StackService>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker = super::schedule::interval("stack-webhooks", Duration::from_secs(5));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {}
        }
        let _iteration = citadel_application::runtime_metrics::RuntimeWork::StackWebhooks.start();
        if let Err(error) = stacks.process_webhooks().await {
            tracing::warn!(%error,"Stack webhook dispatch failed");
        }
    }
}

pub(super) async fn stack_operation_reconciliation(
    cancellation: CancellationToken,
    stacks: Arc<StackService>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker = tokio::time::interval(RECONCILIATION_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {}
        }
        let _iteration = citadel_application::runtime_metrics::RuntimeWork::StackRecovery.start();
        match stacks
            .reconcile_stale_operations(OBSERVABLE_AFTER, MAXIMUM_BATCH)
            .await
        {
            Ok(count) if count > 0 => tracing::info!(count, "reconciled Stack operations"),
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "Stack operation reconciliation failed"),
        }
    }
}

pub(super) async fn stack_drift_monitor(
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
        let _iteration = citadel_application::runtime_metrics::RuntimeWork::StackDrift.start();
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

/// The .NET drift monitor also consumes stop/pause events. Notifications are
/// transactional and originate from the shared Local/Direct/Edge event store.
pub(super) async fn event_drift(
    cancellation: CancellationToken,
    stacks: Arc<StackService>,
    mut listener: sqlx::postgres::PgListener,
) -> Result<(), std::convert::Infallible> {
    loop {
        let result = async {
            loop {
                let notification = tokio::select! { ()=cancellation.cancelled()=>return Ok::<(),sqlx::Error>(()), notification=listener.recv()=>notification? };
                let Ok(id) = uuid::Uuid::parse_str(notification.payload()) else { continue; };
                if let Err(error) = stacks.monitor_container_event(id).await {
                    tracing::warn!(%error, %id, "Event-triggered Stack drift repair failed");
                }
            }
        }.await;
        if cancellation.is_cancelled() {
            return Ok(());
        }
        if let Err(error) = result {
            tracing::warn!(%error, "Stack drift event listener failed");
        }
        tokio::select! { ()=cancellation.cancelled()=>return Ok(()), _=tokio::time::sleep(Duration::from_secs(2))=>{} }
    }
}
