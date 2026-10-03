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
        if let Err(error) = service.reconcile(&cancellation).await {
            tracing::warn!(%error, "Container operation reconciliation failed");
        }
    }
    Ok(())
}
