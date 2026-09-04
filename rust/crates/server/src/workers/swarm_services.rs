use std::sync::Arc;
use std::time::Duration;

use citadel_swarm_services::ManagedSwarmServiceService;
use tokio_util::sync::CancellationToken;

const RECONCILIATION_INTERVAL: Duration = Duration::from_secs(5);
const OBSERVABLE_AFTER: Duration = Duration::from_secs(2);
const MAXIMUM_BATCH: i64 = 25;

pub(super) async fn swarm_service_operation_reconciliation(
    cancellation: CancellationToken,
    services: Arc<ManagedSwarmServiceService>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker = tokio::time::interval(RECONCILIATION_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {}
        }
        match services
            .reconcile_stale_operations(OBSERVABLE_AFTER, MAXIMUM_BATCH)
            .await
        {
            Ok(count) if count > 0 => {
                tracing::info!(count, "reconciled managed Swarm Service operations");
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(%error, "managed Swarm Service reconciliation failed");
            }
        }
    }
}
