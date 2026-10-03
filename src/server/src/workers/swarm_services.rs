use std::sync::Arc;
use std::time::Duration;

use citadel_swarm_services::SwarmServiceService;
use tokio_util::sync::CancellationToken;

// Dispatch is bounded to ten minutes; crash recovery must not race it.
const OBSERVABLE_AFTER: Duration = Duration::from_secs(11 * 60);
const MAXIMUM_BATCH: i64 = 25;

pub(super) async fn swarm_service_image_updates(
    cancellation: CancellationToken,
    services: Arc<SwarmServiceService>,
) -> Result<(), std::convert::Infallible> {
    // Matches DeploymentAutoUpdateJob's two-hour Service check cadence. Keep it
    // separate from active rollout observation.
    let mut ticker =
        super::schedule::interval("service-image-updates", Duration::from_secs(2 * 60 * 60));
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
    services: Arc<SwarmServiceService>,
    signals: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    let mut wake =
        super::recovery::RecoveryWake::new("service-recovery", signals, Duration::from_secs(65));
    while wake.next(&cancellation).await {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::ServiceRecovery.start();
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
    Ok(())
}

/// Accepted rollouts get short follow-ups; empty deployments do no periodic
/// five-second work. The fixed notification is a hint, SQL remains authoritative.
pub(super) async fn observe_active_operations(
    cancellation: CancellationToken,
    services: Arc<SwarmServiceService>,
    signals: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    let mut wake = super::recovery::ActiveWake::new(signals);
    let mut cursor = None;
    let mut cycle_has_work = false;
    while wake.next(&cancellation).await {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::ServiceObservation.start();
        match services
            .observe_active_operations(cursor, MAXIMUM_BATCH)
            .await
        {
            Ok((next, count)) => {
                cycle_has_work |= count > 0;
                cursor = next;
                wake.active = cycle_has_work;
                if cursor.is_none() {
                    cycle_has_work = false;
                }
            }
            Err(error) => {
                tracing::warn!(%error, "Active Service observation failed; retrying at recovery fallback");
                cursor = None;
                cycle_has_work = false;
                wake.active = false;
            }
        }
    }
    Ok(())
}
