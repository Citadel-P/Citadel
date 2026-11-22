use citadel_scripts::{ScriptError, ScriptService};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub async fn script_cleanup(
    cancellation: CancellationToken,
    service: Arc<ScriptService>,
) -> Result<(), ScriptError> {
    while !cancellation.is_cancelled() {
        if let Err(error) =
            citadel_runtime::jobs::observe(service.reconcile_remote(&cancellation)).await
        {
            tracing::warn!(%error, "Script executor cleanup reconciliation failed");
        }
        if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
            job.policy("30 seconds after completion", None);
            job.waiting(
                "Next cleanup check",
                Some(tokio::time::Instant::now() + std::time::Duration::from_secs(30)),
            );
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            () = tokio::time::sleep(std::time::Duration::from_secs(30)) => {}
        }
    }
    Ok(())
}
