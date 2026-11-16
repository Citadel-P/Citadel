use citadel_platforms::containers::ContainerMutationService;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub async fn reconcile(
    cancellation: CancellationToken,
    service: Arc<ContainerMutationService>,
    signals: tokio::sync::watch::Receiver<()>,
) -> Result<(), citadel_platforms::RuntimeCapabilityError> {
    let mut wake =
        super::recovery::RecoveryWake::new("container-recovery", signals, Duration::from_secs(65));
    while wake.next(&cancellation).await {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::ContainerRecovery.start();
        if let Err(error) =
            citadel_runtime::jobs::observe(service.reconcile(&cancellation, report_unconfirmed))
                .await
        {
            tracing::warn!(%error, "Container operation reconciliation failed");
        }
    }
    Ok(())
}

fn report_unconfirmed(claim: &citadel_platforms::containers::ContainerClaim) {
    if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
        for target in &claim.targets {
            job.report_problem(
                "Recovery could not confirm the whole container operation. This container was part of that operation; its current state may differ.",
                citadel_runtime::jobs::JobTarget {
                    resource_type: "Platform".into(),
                    resource_id: target.platform_id,
                    operation_id: Some(claim.operation_id),
                    container_id: Some(target.id),
                },
            );
        }
    }
}
