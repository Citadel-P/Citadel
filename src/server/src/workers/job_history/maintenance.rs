use citadel_instance::{JobDiagnosticSource, JobHistoryMaintenance};
use citadel_runtime::jobs::{JobHandle, JobOutcome};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

/// Owned by Core's existing database lease. Like statistics compaction, use
/// short committed batches with a full rest interval after exhausting a budget.
pub async fn run_maintenance(
    store: Arc<dyn JobHistoryMaintenance>,
    source: Arc<dyn JobDiagnosticSource>,
    cancel: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    let mut tick = super::super::schedule::phased_interval(
        "job-history-maintenance",
        Duration::from_secs(300),
    );
    let budget = Duration::from_secs(5);
    if let Some(job) = JobHandle::current() {
        job.policy(
            "Every five minutes; retry unfinished maintenance after one minute",
            Some(budget),
        );
    }
    loop {
        tokio::select! { () = cancel.cancelled() => break, _ = tick.tick() => {} }
        let iteration = JobHandle::iteration();
        let mut total = 0;
        let result = tokio::select! {
            () = cancel.cancelled() => break,
            result = tokio::time::timeout(budget, async {
                for _ in 0..128 {
                    let changed = store.maintain().await?;
                    if changed == 0 { return Ok(false); }
                    total += changed;
                    // Notify only after the adapter has committed its transaction.
                    source.committed();
                    if let Some(job) = JobHandle::current() { job.progress(changed, "history rows processed"); }
                    tokio::task::yield_now().await;
                }
                Ok::<_, citadel_instance::InstanceError>(true)
            }) => result,
        };
        let (outcome, delay) = match result {
            Ok(Ok(false)) => (
                if total == 0 {
                    JobOutcome::NoWork
                } else {
                    JobOutcome::Productive
                },
                300,
            ),
            Ok(Ok(true)) | Err(_) => (JobOutcome::BudgetExhausted, 60),
            Ok(Err(error)) => {
                tracing::warn!(%error, "Job history maintenance failed");
                (JobOutcome::Failed, 300)
            }
        };
        if let Some(iteration) = iteration {
            iteration.finish(outcome, None);
        }
        tick.reset_after(Duration::from_secs(delay));
    }
    Ok(())
}
