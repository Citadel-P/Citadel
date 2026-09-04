use std::sync::Arc;
use std::time::Duration;

use citadel_deployments::DeploymentService;
use tokio_util::sync::CancellationToken;

const RECONCILIATION_INTERVAL: Duration = Duration::from_secs(60);
const STALE_AFTER: Duration = Duration::from_secs(11 * 60);
const MAXIMUM_BATCH: i64 = 10;

pub(super) async fn deployment_apply_reconciliation(
    cancellation: CancellationToken,
    deployments: Arc<DeploymentService>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker = tokio::time::interval(RECONCILIATION_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {}
        }
        let started_before = chrono::Utc::now().timestamp()
            - i64::try_from(STALE_AFTER.as_secs()).unwrap_or(i64::MAX);
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
}
