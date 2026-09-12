//! Keep one recent platform statistics sample alongside each container statistics stream.
use citadel_platforms::{PlatformRuntimePort, RuntimePlatformStats};
use futures_util::{Stream, StreamExt};
use std::{pin::Pin, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

pub(super) struct LatestPlatformStats {
    sample: Option<(Instant, RuntimePlatformStats)>,
    max_age: Duration,
}
impl LatestPlatformStats {
    pub fn new(interval: Duration) -> Self {
        Self {
            sample: None,
            max_age: interval.saturating_mul(3),
        }
    }
    pub fn record(&mut self, sample: Option<RuntimePlatformStats>) {
        self.sample = sample.map(|s| (Instant::now(), s));
    }
    pub fn get(&self) -> Option<&RuntimePlatformStats> {
        self.sample
            .as_ref()
            .filter(|(at, _)| at.elapsed() <= self.max_age)
            .map(|(_, s)| s)
    }
}

/// Failure of optional disk telemetry must not interrupt CPU/container telemetry.
pub(super) fn samples<R: PlatformRuntimePort + Send + Sync + 'static>(
    runtime: R,
    interval: Duration,
    cancel: CancellationToken,
) -> Pin<Box<dyn Stream<Item = Option<RuntimePlatformStats>> + Send>> {
    Box::pin(async_stream::stream! {
        loop {
            let stream = tokio::select! {
                () = cancel.cancelled() => break,
                stream = runtime.stream_stats(interval, &cancel) => stream,
            };
            if let Ok(mut stream) = stream {
                loop {
                    let next = tokio::select! { () = cancel.cancelled() => return, next = stream.next() => next };
                    match next {
                        Some(Ok(stat)) => yield Some(stat),
                        _ => break,
                    }
                }
            }
            yield None;
            tokio::select! { () = cancel.cancelled() => break, () = tokio::time::sleep(interval) => {} }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn unavailable_and_stale_samples_do_not_reuse_last_disk_value() {
        let mut latest = LatestPlatformStats::new(Duration::from_secs(10));
        let sample = RuntimePlatformStats {
            disk_used_bytes: Some(95),
            disk_total_bytes: Some(100),
            disk_usage: Some(95.0),
            image_used_bytes: Some(2048),
            volume_used_bytes: Some(4096),
            ..Default::default()
        };
        latest.record(Some(sample.clone()));
        assert_eq!(latest.get(), Some(&sample));
        tokio::time::advance(Duration::from_secs(31)).await;
        assert_eq!(latest.get(), None);
        latest.record(Some(sample));
        latest.record(None);
        assert_eq!(latest.get(), None);
    }
}
