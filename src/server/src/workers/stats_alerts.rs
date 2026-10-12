//! Evaluate one smoothed observation per committed Platform writer batch.
//! PostgreSQL is the buffer: idle input cannot strand samples or grow an in-memory queue.
use citadel_alerts::AlertEventSink;
use citadel_alerts::platform_metrics::PlatformAlertBatchStore;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub(crate) async fn run(
    cancel: CancellationToken,
    store: Arc<dyn PlatformAlertBatchStore>,
    alerts: Arc<dyn AlertEventSink>,
    interval: Duration,
    batch_size: i64,
    mut signals: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    // Commit notifications avoid adding another full flush interval of alert
    // latency. PostgreSQL retains pending rows across missed signals/restarts.
    loop {
        super::notifications::wait(&mut signals, &cancel, interval).await;
        if cancel.is_cancelled() {
            return Ok(());
        }
        let drain = async {
            for _ in 0..10 {
                let count = store.flush_pending(alerts.as_ref(), batch_size).await?;
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
                Ok(Err(error)) => tracing::warn!(%error, "Platform threshold batch failed; retaining samples"),
                Err(_) => tracing::warn!("Platform threshold flush exhausted its time budget; retaining samples"),
            }
        }
    }
}

#[cfg(test)]
pub(crate) use super::test_support::flush_pending;
