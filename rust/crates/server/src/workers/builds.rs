use std::sync::Arc;
use std::time::Duration;

use citadel_builds::{BuildError, BuildService};
use tokio_util::sync::CancellationToken;

pub async fn build_consumers(
    cancellation: CancellationToken,
    service: citadel_builds::BuildCompletionService,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), BuildError> {
    while !cancellation.is_cancelled() {
        let result = {
            let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::BuildCompletion.start();
            service.process_batch(&cancellation).await
        };
        match result {
            Ok(count) if count > 0 => continue,
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "Build consumer propagation failed"),
        }
        super::notifications::wait(&mut wake, &cancellation, Duration::from_secs(30)).await;
    }
    Ok(())
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
    wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), BuildError> {
    for result in futures_util::future::join_all(
        (0..parallel_runs).map(|_| run_worker(cancellation.clone(), service.clone(), wake.clone())),
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
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), BuildError> {
    let minimum = Duration::from_secs(2);
    let mut delay = minimum;
    while !cancellation.is_cancelled() {
        let result = {
            let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::BuildClaim.start();
            service.process_one(&cancellation).await
        };
        if matches!(result, Ok(true)) {
            citadel_runtime::runtime_metrics::RuntimeWork::BuildClaim.units(1);
        }
        match result {
            Ok(true) => {
                delay = citadel_runtime::worker_poll_delay(delay, minimum, true);
                continue;
            }
            Ok(false) => {}
            Err(error) => tracing::error!(%error,"Build worker iteration failed"),
        }
        delay = citadel_runtime::worker_poll_delay(delay, minimum, false);
        super::notifications::wait(
            &mut wake,
            &cancellation,
            delay.max(std::time::Duration::from_secs(30)),
        )
        .await;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
