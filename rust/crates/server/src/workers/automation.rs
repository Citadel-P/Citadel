use std::sync::Arc;

use citadel_automation::{AutomationError, AutomationService};
use tokio_util::sync::CancellationToken;

pub async fn automation_runs(
    cancellation: CancellationToken,
    service: Arc<AutomationService>,
) -> Result<(), AutomationError> {
    let workers = (0..service.options().max_parallel_runs)
        .map(|_| run_worker(cancellation.clone(), service.clone()));
    // Each loop awaits its process cleanup; shutdown never drops active runs.
    for result in futures_util::future::join_all(workers).await {
        result?;
    }
    Ok(())
}

async fn run_worker(
    cancellation: CancellationToken,
    service: Arc<AutomationService>,
) -> Result<(), AutomationError> {
    while !cancellation.is_cancelled() {
        match service.process_one(&cancellation).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(error) => tracing::error!(%error, "automation worker iteration failed"),
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            () = tokio::time::sleep(service.options().poll_interval) => {}
        }
    }
    Ok(())
}

pub async fn automation_scheduler(
    cancellation: CancellationToken,
    service: Arc<AutomationService>,
) -> Result<(), AutomationError> {
    while !cancellation.is_cancelled() {
        if let Err(error) = service.queue_due_scheduled(chrono::Utc::now()).await {
            tracing::warn!(%error, "automation scheduler tick failed");
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            () = tokio::time::sleep(service.options().schedule_poll_interval) => {}
        }
    }
    Ok(())
}
