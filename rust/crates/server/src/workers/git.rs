use std::sync::Arc;
use std::time::Duration;

use citadel_git::GitRepositoryExecutionService;
use tokio_util::sync::CancellationToken;

const IDLE_DELAY: Duration = Duration::from_secs(2);
const FAILURE_DELAY: Duration = Duration::from_secs(5);
const SCHEDULE_INTERVAL: Duration = Duration::from_secs(60);

pub(super) async fn git_repository_sync(
    cancellation: CancellationToken,
    service: Arc<GitRepositoryExecutionService>,
) -> Result<(), String> {
    let mut schedule = super::schedule::interval("git-sync", SCHEDULE_INTERVAL);
    schedule.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        // Keep a claimed execution alive across scheduling ticks. Dropping it
        // here used to interrupt clones taking longer than a minute and leave
        // their durable claim awaiting stale-run recovery.
        let iteration = citadel_runtime::runtime_metrics::RuntimeWork::GitSync.start();
        let execution = service.process_one(&cancellation);
        tokio::pin!(execution);
        let result = loop {
            tokio::select! {
                result = &mut execution => break result,
                _ = schedule.tick(), if !cancellation.is_cancelled() => {
                    if let Err(error) = service.enqueue_due(100).await {
                        tracing::warn!(%error, "failed to enqueue due Git repository synchronizations");
                    }
                }
            }
        };
        drop(iteration);
        match result {
            Ok(true) => {}
            Ok(false) => {
                if wait_or_cancel(&cancellation, IDLE_DELAY).await {
                    return Ok(());
                }
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

#[cfg(test)]
mod tests;
