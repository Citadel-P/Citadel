use citadel_platforms::containers::ContainerMutationService;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub async fn reconcile(
    cancellation: CancellationToken,
    service: Arc<ContainerMutationService>,
) -> Result<(), citadel_platforms::RuntimeCapabilityError> {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! { biased; ()=cancellation.cancelled()=>break, _=interval.tick()=>{} }
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::ContainerRecovery.start();
        if let Err(error) = service.reconcile(&cancellation).await {
            tracing::warn!(%error, "Container operation reconciliation failed");
        }
    }
    Ok(())
}
