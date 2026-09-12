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
    parallel_runs: usize,
) -> Result<(), BuildError> {
    for result in futures_util::future::join_all(
        (0..parallel_runs).map(|_| run_worker(cancellation.clone(), service.clone())),
    )
    .await
    {
        result?;
    }
    Ok(())
}

async fn run_worker(
    cancellation: CancellationToken,
    service: Arc<BuildService>,
) -> Result<(), BuildError> {
    let minimum = Duration::from_secs(2);
    let mut delay = minimum;
    while !cancellation.is_cancelled() {
        match service.process_one(&cancellation).await {
            Ok(true) => {
                delay = citadel_application::worker_poll_delay(delay, minimum, true);
                continue;
            }
            Ok(false) => {}
            Err(error) => tracing::error!(%error,"Build worker iteration failed"),
        }
        delay = citadel_application::worker_poll_delay(delay, minimum, false);
        tokio::select! {()=cancellation.cancelled()=>break,()=tokio::time::sleep(delay)=>{}}
    }
    Ok(())
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
