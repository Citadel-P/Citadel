use citadel_adapters::connectors::routing::volumes::content::VolumeContentAdapter;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(super) async fn reconcile(
    cancellation: CancellationToken,
    volumes: Arc<VolumeContentAdapter>,
) -> Result<(), std::convert::Infallible> {
    let mut tick = super::schedule::interval("volume_helpers", super::recovery::FALLBACK);
    let mut startup = true;
    let mut after = Uuid::nil();
    loop {
        if cancellation.is_cancelled() {
            break;
        }
        if !std::mem::take(&mut startup) {
            tokio::select! { ()=cancellation.cancelled()=>break,_=tick.tick()=>{} }
        }
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::VolumeRecovery.start();
        match volumes.reap_expired(after, &cancellation).await {
            Ok(next) => after = next,
            Err(error) => tracing::warn!(%error,"Volume helper recovery failed"),
        }
    }
    Ok(())
}
