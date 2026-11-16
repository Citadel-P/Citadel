use std::{sync::Arc, time::Duration};

use citadel_scripts::{ScriptError, ScriptService};
use tokio_util::sync::CancellationToken;

pub async fn script_runs(
    admission: citadel_runtime::admission::AdmissionGate,
    cancellation: CancellationToken,
    service: Arc<ScriptService>,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), ScriptError> {
    let cancellation = cancellation.child_token();
    let mut controllers = tokio::task::JoinSet::new();
    let capacity = service.options().max_parallel_runs + service.options().max_remote_runs;
    let fallback = service.options().poll_interval.max(Duration::from_secs(30));
    let mut remote_first = false;
    let result = async {
        while !cancellation.is_cancelled() {
            while let Some(result) = controllers.try_join_next() {
                controller_result(result)?;
            }
            if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
                job.active_operations(controllers.len());
            }
            if controllers.len() < capacity {
                let Some(permit) = admission.enter("Scripts", &cancellation).await else {
                    break;
                };
                let claim = {
                    let _iteration =
                        citadel_runtime::runtime_metrics::RuntimeWork::ScriptClaim.start();
                    // Finish the durable claim even if shutdown arrives. Never drop
                    // a committed run between claiming and assigning its controller.
                    citadel_runtime::jobs::observe_work(
                        service.claim_execution(remote_first),
                        |v| Some((u64::from(v.is_some()), "claims")),
                    )
                    .await
                };
                match claim {
                    Ok(Some(execution)) => {
                        // Fast local completions must not starve the remote queue.
                        remote_first = !remote_first;
                        citadel_runtime::runtime_metrics::RuntimeWork::ScriptClaim.units(1);
                        let service = service.clone();
                        let cancellation = cancellation.clone();
                        controllers.spawn(async move {
                            let _admission = permit;
                            service.execute_claimed(execution, &cancellation).await
                        });
                        continue;
                    }
                    Ok(None) => {}
                    Err(error) => tracing::error!(%error, "scripts claim failed"),
                }
            }
            tokio::select! {
                () = cancellation.cancelled() => break,
                result = controllers.join_next(), if !controllers.is_empty() => {
                    if let Some(result) = result { controller_result(result)?; }
                }
                () = super::super::notifications::wait(&mut wake, &cancellation, fallback) => {}
            }
        }
        Ok(())
    }
    .await;
    if result.is_err() {
        cancellation.cancel();
    }
    // Cancellation requests cleanup; it must not abort execution or finalization.
    while let Some(completed) = controllers.join_next().await {
        if let Err(error) = controller_result(completed) {
            tracing::error!(%error, "Script controller failed while draining");
        }
    }
    result
}

fn controller_result(
    result: Result<Result<(), ScriptError>, tokio::task::JoinError>,
) -> Result<(), ScriptError> {
    // An execution failure must not stop unrelated queued work. A task panic
    // cancels and drains the other owned controllers before returning the failure to supervision.
    if let Err(error) = result
        .map_err(|error| ScriptError::Storage(format!("Script controller failed: {error}")))?
    {
        tracing::error!(%error, "scripts execution failed");
    }
    Ok(())
}
