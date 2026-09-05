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
    let mut schedule = tokio::time::interval(SCHEDULE_INTERVAL);
    schedule.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            _ = schedule.tick() => {
                if let Err(error) = service.enqueue_due(100).await {
                    tracing::warn!(%error, "failed to enqueue due Git repository synchronizations");
                }
            }
            result = service.process_one(&cancellation) => {
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
    }
}

async fn wait_or_cancel(cancellation: &CancellationToken, duration: Duration) -> bool {
    tokio::select! {
        () = cancellation.cancelled() => true,
        () = tokio::time::sleep(duration) => false,
    }
}
