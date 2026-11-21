use std::sync::Arc;
use std::time::Duration;

use citadel_git::GitRepositoryExecutionService;
use tokio_util::sync::CancellationToken;

const IDLE_DELAY: Duration = Duration::from_secs(30);
const FAILURE_DELAY: Duration = Duration::from_secs(5);

pub(crate) async fn git_repository_sync(
    admission: citadel_runtime::admission::AdmissionGate,
    cancellation: CancellationToken,
    service: Arc<GitRepositoryExecutionService>,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), String> {
    loop {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        // Keep a claimed execution alive across scheduling ticks. Dropping it
        // here used to interrupt clones taking longer than a minute and leave
        // their durable claim awaiting stale-run recovery.
        let Some(permit) = admission.enter("Git synchronization", &cancellation).await else {
            return Ok(());
        };
        let iteration = citadel_runtime::runtime_metrics::RuntimeWork::GitSync.start();
        let execution =
            citadel_runtime::jobs::observe_work(service.process_one(&cancellation), |worked| {
                Some((u64::from(*worked), "operations"))
            });
        let result = execution.await;
        drop(iteration);
        drop(permit);
        match result {
            Ok(true) => {}
            Ok(false) => {
                super::super::notifications::wait(&mut wake, &cancellation, IDLE_DELAY).await;
            }
            Err(error) => {
                tracing::error!(%error, "Git repository synchronization worker failed");
                if wait_or_cancel(&cancellation, FAILURE_DELAY).await {
                    return Ok(());
                }
            }
        }
    }
}

async fn wait_or_cancel(cancellation: &CancellationToken, duration: Duration) -> bool {
    tokio::select! {
        () = cancellation.cancelled() => true,
        () = tokio::time::sleep(duration) => false,
    }
}
