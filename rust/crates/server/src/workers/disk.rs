//! Keep one recent host disk sample alongside each container statistics stream.
use citadel_platforms::{HostDiskUsage, PlatformRuntimePort};
use futures_util::{Stream, StreamExt};
use std::{pin::Pin, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

pub(super) struct LatestDisk {
    sample: Option<(Instant, HostDiskUsage)>,
    max_age: Duration,
}
impl LatestDisk {
    pub fn new(interval: Duration) -> Self {
        Self {
            sample: None,
            max_age: interval.saturating_mul(3),
        }
    }
    pub fn record(&mut self, sample: Option<HostDiskUsage>) {
        self.sample = sample.map(|s| (Instant::now(), s));
    }
    pub fn get(&self) -> Option<HostDiskUsage> {
        self.sample
            .filter(|(at, _)| at.elapsed() <= self.max_age)
            .map(|(_, s)| s)
    }
}

/// Failure of optional disk telemetry must not interrupt CPU/container telemetry.
pub(super) fn samples<R: PlatformRuntimePort + Send + Sync + 'static>(
    runtime: R,
    interval: Duration,
    cancel: CancellationToken,
) -> Pin<Box<dyn Stream<Item = Option<HostDiskUsage>> + Send>> {
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
                        Some(Ok(stat)) => yield stat.disk(),
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
        let mut latest = LatestDisk::new(Duration::from_secs(10));
        let disk = HostDiskUsage::new(95, 100, 95.0).unwrap();
        latest.record(Some(disk));
        assert_eq!(latest.get(), Some(disk));
        tokio::time::advance(Duration::from_secs(31)).await;
        assert_eq!(latest.get(), None);
        latest.record(Some(disk));
        latest.record(None);
        assert_eq!(latest.get(), None);
    }
}
