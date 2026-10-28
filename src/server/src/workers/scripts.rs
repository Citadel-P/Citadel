use std::sync::Arc;

use citadel_scripts::{ScriptError, ScriptService};
use tokio_util::sync::CancellationToken;

pub async fn script_runs(
    cancellation: CancellationToken,
    service: Arc<ScriptService>,
    wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), ScriptError> {
    let workers = (0..service.options().max_parallel_runs + service.options().max_remote_runs)
        .map(|_| run_worker(cancellation.clone(), service.clone(), wake.clone()));
    // Each loop awaits its process cleanup; shutdown never drops active runs.
    for result in futures_util::future::join_all(workers).await {
        result?;
    }
    Ok(())
}

async fn run_worker(
    cancellation: CancellationToken,
    service: Arc<ScriptService>,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), ScriptError> {
    let minimum = service.options().poll_interval;
    let mut delay = minimum;
    while !cancellation.is_cancelled() {
        let result = {
            let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::ScriptClaim.start();
            service.process_one(&cancellation).await
        };
        if matches!(result, Ok(true)) {
            citadel_runtime::runtime_metrics::RuntimeWork::ScriptClaim.units(1);
        }
        match result {
            Ok(true) => {
                delay = citadel_runtime::worker_poll_delay(delay, minimum, true);
                continue;
            }
            Ok(false) => {}
            Err(error) => tracing::error!(%error, "scripts worker iteration failed"),
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

pub async fn script_scheduler(
    cancellation: CancellationToken,
    service: Arc<ScriptService>,
) -> Result<(), ScriptError> {
    while !cancellation.is_cancelled() {
        if let Err(error) = service.queue_due_scheduled(chrono::Utc::now()).await {
            tracing::warn!(%error, "scripts scheduler tick failed");
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            () = tokio::time::sleep(service.options().schedule_poll_interval) => {}
        }
    }
    Ok(())
}

// Slow/offline executors must not delay cron scheduling.
pub async fn script_cleanup(
    cancellation: CancellationToken,
    service: Arc<ScriptService>,
) -> Result<(), ScriptError> {
    while !cancellation.is_cancelled() {
        if let Err(error) = service.reconcile_remote(&cancellation).await {
            tracing::warn!(%error, "Script executor cleanup reconciliation failed");
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            () = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
        }
    }
    Ok(())
}
