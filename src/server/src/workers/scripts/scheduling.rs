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
            let check = service.queue_due_scheduled(chrono::Utc::now());
            let result = if service.options().enabled {
                citadel_runtime::jobs::observe_work(check, |count| {
                    Some((*count as u64, "queued operations"))
                })
                .await
            } else {
                // Retain the worker's existing disabled check without claiming a successful iteration.
                check.await
            };
            if let Err(error) = result {
                tracing::warn!(%error, "scripts scheduler tick failed");
            }
        }
        if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
            let delay = service.options().schedule_poll_interval;
            job.policy(
                format!("{} seconds after completion", delay.as_secs_f64()),
                None,
            );
            if service.options().enabled {
                job.waiting(
                    "Next schedule check",
                    Some(tokio::time::Instant::now() + delay),
                );
            } else {
                job.disabled("Script scheduling disabled in Core configuration");
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
