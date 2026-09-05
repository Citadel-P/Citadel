use std::sync::Arc;
use std::time::Duration;

use citadel_automation::{AutomationError, AutomationService};
use tokio_util::sync::CancellationToken;

pub async fn automation_runs(
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
            () = tokio::time::sleep(Duration::from_secs(2)) => {}
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
            () = tokio::time::sleep(Duration::from_secs(15)) => {}
        }
    }
    Ok(())
}
