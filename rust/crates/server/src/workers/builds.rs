use std::sync::Arc;
use std::time::Duration;

use citadel_builds::{BuildError, BuildService};
use tokio_util::sync::CancellationToken;

pub async fn build_consumers(
    cancellation: CancellationToken,
    service: citadel_builds::BuildCompletionService,
) -> Result<(), BuildError> {
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! { ()=cancellation.cancelled()=>return Ok(()), _=ticker.tick()=>{} }
        if let Err(error) = service.process_batch(&cancellation).await {
            tracing::warn!(%error,"Build consumer propagation failed");
        }
    }
}

pub async fn pool_health(
    cancellation: CancellationToken,
    service: Arc<BuildService>,
) -> Result<(), BuildError> {
    while !cancellation.is_cancelled() {
        if let Err(error) = service.monitor_pool_health(&cancellation).await {
            tracing::error!(%error,"Build Pool health monitoring failed");
        }
        tokio::select! {()=cancellation.cancelled()=>break,()=tokio::time::sleep(Duration::from_secs(30))=>{}}
    }
    Ok(())
}

pub async fn build_runs(
    cancellation: CancellationToken,
    service: Arc<BuildService>,
) -> Result<(), BuildError> {
    while !cancellation.is_cancelled() {
        match service.process_one(&cancellation).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(error) => tracing::error!(%error,"Build worker iteration failed"),
        }
        tokio::select! {()=cancellation.cancelled()=>break,()=tokio::time::sleep(Duration::from_secs(2))=>{}}
    }
    Ok(())
}
