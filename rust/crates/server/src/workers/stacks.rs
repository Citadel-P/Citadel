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
    let mut ticker = tokio::time::interval(DRIFT_MONITOR_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut cursor = None;
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {}
        }
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
                for failure in result.failures {
                    tracing::warn!(
                        stack_id = %failure.stack_id,
                        error = %failure.message,
                        "Stack drift monitoring failed"
                    );
                }
            }
            Err(error) => tracing::warn!(%error, "Stack drift monitor query failed"),
        }
    }
}
