use citadel_adapters::connectors::routing::volumes::content::VolumeContentAdapter;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(super) async fn reconcile(
    cancellation: CancellationToken,
    volumes: Arc<VolumeContentAdapter>,
) -> Result<(), std::convert::Infallible> {
    let mut tick = tokio::time::interval(Duration::from_secs(60));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut after = Uuid::nil();
    loop {
        tokio::select! { ()=cancellation.cancelled()=>break,_=tick.tick()=>{} }
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::VolumeRecovery.start();
        match volumes.reap_expired(after, &cancellation).await {
            Ok(next) => after = next,
            Err(error) => tracing::warn!(%error,"Volume helper recovery failed"),
        }
    }
    Ok(())
}
