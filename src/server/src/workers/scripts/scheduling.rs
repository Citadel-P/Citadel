use citadel_scripts::{ScriptError, ScriptService};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub async fn script_scheduler(
    admission: citadel_runtime::admission::AdmissionGate,
    cancellation: CancellationToken,
    service: Arc<ScriptService>,
) -> Result<(), ScriptError> {
    while !cancellation.is_cancelled() {
        {
            let Some(_admission) = admission.enter("Script schedules", &cancellation).await else {
                return Ok(());
            };
            if let Err(error) = service.queue_due_scheduled(chrono::Utc::now()).await {
                tracing::warn!(%error, "scripts scheduler tick failed");
            }
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
