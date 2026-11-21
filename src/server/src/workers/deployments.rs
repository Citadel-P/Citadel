use std::sync::Arc;
use std::time::Duration;

use citadel_deployments::DeploymentService;
use tokio_util::sync::CancellationToken;

const STALE_AFTER: Duration = Duration::from_secs(11 * 60);
const MAXIMUM_BATCH: i64 = 10;

pub(super) fn report_update_problem(id: uuid::Uuid) {
    if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
        job.report_problem(
            "A deployment update check or automatic apply reported a problem. Open the deployment for its recorded outcome.",
            citadel_runtime::jobs::JobTarget {
                resource_type: "Deployment".into(),
                resource_id: id,
                operation_id: None,
                container_id: None,
            },
        );
    }
}

pub(crate) async fn deployment_apply_reconciliation(
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
        if let Err(error) =
            citadel_runtime::jobs::observe(deployments.recover_update_checks()).await
        {
            tracing::warn!(%error, "Deployment update check recovery failed");
        }
        match citadel_runtime::jobs::observe_work(
            deployments.reconcile_stale_applies(started_before, MAXIMUM_BATCH),
            |count| Some((*count as u64, "operations")),
        )
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
