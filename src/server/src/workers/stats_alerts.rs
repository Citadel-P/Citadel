//! Evaluate one smoothed observation per committed Platform writer batch.
//! PostgreSQL is the buffer: idle input cannot strand samples or grow an in-memory queue.
use citadel_alerts::platform_metrics::PlatformAlertBatchStore;
use citadel_runtime::runtime_metrics::RuntimeWork;
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

pub(crate) async fn run(
    cancel: CancellationToken,
    store: Arc<dyn PlatformAlertBatchStore>,
    interval: Duration,
    batch_size: i64,
    mut signals: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    let mut diagnostics_at = Instant::now();
    let mut last_progress = None::<Instant>;
    let mut last_failure_report = None::<Instant>;
    // Commit notifications avoid adding another full flush interval of alert
    // latency. PostgreSQL retains pending rows across missed signals/restarts.
    loop {
        super::notifications::wait(&mut signals, &cancel, interval).await;
        if cancel.is_cancelled() {
            return Ok(());
        }
        if Instant::now() >= diagnostics_at {
            diagnostics_at = Instant::now() + Duration::from_secs(60);
            let backlog = tokio::select! {
                () = cancel.cancelled() => return Ok(()),
                result = tokio::time::timeout(Duration::from_secs(2), store.backlog()) => result,
            };
            match backlog {
                Ok(Ok(backlog)) => {
                    RuntimeWork::AlertBacklog.queued(backlog.pending);
                    RuntimeWork::AlertBacklog.buffered(usize::from(backlog.blocks_compaction));
                    let age = backlog
                        .oldest_created
                        .map(|created| {
                            chrono::Utc::now()
                                .timestamp()
                                .saturating_sub(created)
                                .max(0) as u64
                        })
                        .unwrap_or(0);
                    RuntimeWork::AlertBacklog.age(Duration::from_secs(age));
                    // u64::MAX means no successful progress observed by this worker.
                    RuntimeWork::AlertProgress.age(
                        last_progress
                            .map(|at| at.elapsed())
                            .unwrap_or(Duration::MAX),
                    );
                    if backlog.blocks_compaction {
                        tracing::warn!(pending=backlog.pending, count_is_lower_bound=backlog.capped, oldest_age_seconds=age, last_progress_seconds=?last_progress.map(|at| at.elapsed().as_secs()), "Pending platform alerts block raw-history compaction");
                    }
                }
                _ => tracing::warn!(
                    "Platform alert backlog diagnostics unavailable; retaining last known metrics"
                ),
            }
        }
        let drain = async {
            for _ in 0..10 {
                let count = store.flush_pending(batch_size).await?;
                if count > 0 {
                    last_progress = Some(Instant::now());
                    RuntimeWork::AlertProgress.success();
                    RuntimeWork::AlertProgress.units(count as u64);
                    RuntimeWork::AlertProgress.age(Duration::ZERO);
                }
                if count < batch_size.max(1) as usize {
                    break;
                }
            }
            Ok::<_, citadel_alerts::AlertError>(())
        };
        tokio::select! {
            () = cancel.cancelled() => return Ok(()),
            result = tokio::time::timeout(Duration::from_secs(5), drain) => match result {
                Ok(Ok(())) => {},
                result => {
                    RuntimeWork::AlertFlush.failures(1);
                    if last_failure_report.is_none_or(|at| at.elapsed() >= Duration::from_secs(60)) {
                        last_failure_report = Some(Instant::now());
                        match result {
                            Ok(Err(error)) => tracing::warn!(%error, "Platform threshold batch failed; retaining samples"),
                            Err(_) => tracing::warn!("Platform threshold flush exhausted its time budget; retaining samples"),
                            _ => unreachable!(),
                        }
                    }
                },
            }
        }
    }
}

#[cfg(test)]
pub(crate) use super::test_support::flush_pending;
