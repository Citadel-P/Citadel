use std::sync::Arc;
use std::time::Duration;

use citadel_swarm_services::ManagedSwarmServiceService;
use tokio_util::sync::CancellationToken;

const RECONCILIATION_INTERVAL: Duration = Duration::from_secs(5);
const OBSERVABLE_AFTER: Duration = Duration::from_secs(2);
const MAXIMUM_BATCH: i64 = 25;

pub(super) async fn swarm_service_image_updates(
    cancellation: CancellationToken,
    services: Arc<ManagedSwarmServiceService>,
) -> Result<(), std::convert::Infallible> {
    // Matches DeploymentAutoUpdateJob's two-hour Service check cadence. Keep it
    // separate from the five-second operation recovery path.
    let mut ticker = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_secs(2 * 60 * 60),
        Duration::from_secs(2 * 60 * 60),
    );
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! { biased; () = cancellation.cancelled() => return Ok(()), _ = ticker.tick() => {} }
        if let Err(error) = services.run_scheduled_update_checks(&cancellation).await {
            if cancellation.is_cancelled() {
                return Ok(());
            }
            tracing::warn!(%error, "Service image update sweep failed");
        }
    }
}

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
        let _iteration = citadel_application::runtime_metrics::RuntimeWork::ServiceRecovery.start();
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
