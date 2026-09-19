use std::sync::Arc;

use citadel_automation::{AutomationError, AutomationService};
use tokio_util::sync::CancellationToken;

pub async fn automation_runs(
    cancellation: CancellationToken,
    service: Arc<AutomationService>,
    wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), AutomationError> {
    let workers = (0..service.options().max_parallel_runs)
        .map(|_| run_worker(cancellation.clone(), service.clone(), wake.clone()));
    // Each loop awaits its process cleanup; shutdown never drops active runs.
    for result in futures_util::future::join_all(workers).await {
        result?;
    }
    Ok(())
}

async fn run_worker(
    cancellation: CancellationToken,
    service: Arc<AutomationService>,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), AutomationError> {
    let minimum = service.options().poll_interval;
    let mut delay = minimum;
    while !cancellation.is_cancelled() {
        let result = {
            let _iteration =
                citadel_application::runtime_metrics::RuntimeWork::AutomationClaim.start();
            service.process_one(&cancellation).await
        };
        if matches!(result, Ok(true)) {
            citadel_application::runtime_metrics::RuntimeWork::AutomationClaim.units(1);
        }
        match result {
            Ok(true) => {
                delay = citadel_application::worker_poll_delay(delay, minimum, true);
                continue;
            }
            Ok(false) => {}
            Err(error) => tracing::error!(%error, "automation worker iteration failed"),
        }
        delay = citadel_application::worker_poll_delay(delay, minimum, false);
        super::notifications::wait(
            &mut wake,
            &cancellation,
            delay.max(std::time::Duration::from_secs(30)),
        )
        .await;
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
