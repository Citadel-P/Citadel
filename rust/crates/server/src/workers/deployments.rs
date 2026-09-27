use std::sync::Arc;
use std::time::Duration;

use citadel_deployments::DeploymentService;
use tokio_util::sync::CancellationToken;

const STALE_AFTER: Duration = Duration::from_secs(11 * 60);
const MAXIMUM_BATCH: i64 = 10;

pub(super) async fn image_updates(
    cancellation: CancellationToken,
    deployments: Arc<DeploymentService>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker =
        super::schedule::interval("deployment-image-updates", Duration::from_secs(2 * 60 * 60));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! { biased; () = cancellation.cancelled() => return Ok(()), _ = ticker.tick() => {} }
        if let Err(error) = deployments.run_scheduled_update_checks(&cancellation).await {
            tracing::warn!(%error, "Deployment image update sweep failed");
        }
    }
}

pub(super) async fn deployment_apply_reconciliation(
    cancellation: CancellationToken,
    deployments: Arc<DeploymentService>,
    signals: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    let mut wake =
        super::recovery::RecoveryWake::new("deployment-recovery", signals, Duration::from_secs(65));
    while wake.next(&cancellation).await {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DeploymentRecovery.start();
        let started_before = chrono::Utc::now().timestamp()
            - i64::try_from(STALE_AFTER.as_secs()).unwrap_or(i64::MAX);
        if let Err(error) = deployments.recover_update_checks().await {
            tracing::warn!(%error, "Deployment update check recovery failed");
        }
        match deployments
            .reconcile_stale_applies(started_before, MAXIMUM_BATCH)
            .await
        {
            Ok(count) if count > 0 => {
                tracing::info!(count, "reconciled stale Deployment Apply operations");
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(%error, "Deployment Apply reconciliation failed");
            }
        }
    }
    Ok(())
}
