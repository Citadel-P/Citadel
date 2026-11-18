//! CPU baselines are owned by one sampler and replaced on each successful cycle.
use super::{
    DockerClient, inventory::map_container_stat, projection::ContainerStats,
    runtime::cancelled_error,
};
use chrono::{DateTime, Utc};
use citadel_platforms::{ContainerStatsPort, RuntimeCapabilityError, RuntimeContainerStat};
use futures_util::{StreamExt, future::BoxFuture, stream};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

pub(super) type CpuBaselines = HashMap<String, CpuReading>;

#[derive(Clone, Copy)]
pub(super) struct CpuReading {
    requested: Instant,
    read: DateTime<Utc>,
    total: u64,
    system: u64,
}

impl CpuReading {
    fn from_stats(id: &str, stats: &ContainerStats, requested: Instant) -> Option<Self> {
        let read: DateTime<Utc> = stats.read.parse().ok()?;
        (stats.id == id && stats.cpu_stats.system_cpu_usage > 0 && read.timestamp() > 0).then_some(
            Self {
                requested,
                read,
                total: stats.cpu_stats.cpu_usage.total_usage,
                system: stats.cpu_stats.system_cpu_usage,
            },
        )
    }
}

pub(super) struct WarmSample {
    pub stat: RuntimeContainerStat,
    pub reading: CpuReading,
    previous: CpuReading,
}

impl WarmSample {
    pub(super) fn continuous(&self, status: Option<&str>) -> bool {
        let Some(uptime) = status.and_then(minimum_uptime) else {
            return false;
        };
        let daemon_elapsed = (self.reading.read - self.previous.read)
            .to_std()
            .unwrap_or(Duration::MAX);
        // Discovery follows both readings. A lower bound on uptime must cover the
        // entire interval, including the time spent waiting for the discovery list.
        uptime > self.previous.requested.elapsed().max(daemon_elapsed)
    }
}

// Docker's "Up ..." status is rounded for display. Use only a conservative lower
// bound. Unknown formats take the ordinary two-reading path.
fn minimum_uptime(status: &str) -> Option<Duration> {
    let age = status.strip_prefix("Up ")?.split(" (").next()?;
    let seconds = match age {
        "About a minute" => 60,
        "About an hour" => 3600,
        _ => {
            let (count, unit) = age.split_once(' ')?;
            let count = count.parse::<u64>().ok()?;
            match unit {
                "second" | "seconds" => count,
                "minute" | "minutes" => count.checked_mul(60)?,
                // Docker rounds hours before converting them to days/weeks/months.
                "hour" | "hours" | "day" | "days" | "week" | "weeks" | "month" | "months"
                | "year" | "years" => {
                    let hours = match unit {
                        "hour" | "hours" => 1,
                        "day" | "days" => 24,
                        "week" | "weeks" => 168,
                        "month" | "months" => 720,
                        _ => 8760,
                    };
                    count
                        .checked_mul(hours)?
                        .checked_sub(1)?
                        .checked_mul(3600)?
                }
                _ => return None,
            }
        }
    };
    Some(Duration::from_secs(seconds))
}

pub(super) async fn warm_samples(
    docker: &DockerClient,
    previous: CpuBaselines,
    concurrency: usize,
    cancel: &CancellationToken,
) -> Result<HashMap<String, Result<WarmSample, RuntimeCapabilityError>>, RuntimeCapabilityError> {
    let reads = stream::iter(previous)
        .map(|(id, prev)| async move {
            let requested = Instant::now();
            let mut stats = match docker.read_sample(&id, true, cancel).await {
                Ok(stats) => stats,
                Err(error) => return Some((id, Err(error))),
            };
            let current = CpuReading::from_stats(&id, &stats, requested)?;
            if current.read <= prev.read
                || current.system <= prev.system
                || current.total < prev.total
            {
                return None;
            }
            stats.precpu_stats.system_cpu_usage = prev.system;
            stats.precpu_stats.cpu_usage.total_usage = prev.total;
            let stat = map_container_stat(&id, stats);
            Some((
                id,
                Ok(WarmSample {
                    stat,
                    reading: current,
                    previous: prev,
                }),
            ))
        })
        .buffer_unordered(concurrency)
        .filter_map(std::future::ready)
        .collect();
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(cancelled_error()),
        samples = reads => Ok(samples),
    }
}

pub(super) struct ColdSamples<'a> {
    pub docker: &'a DockerClient,
    pub next: Mutex<CpuBaselines>,
}

impl ContainerStatsPort for ColdSamples<'_> {
    fn sample_container_stats<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeContainerStat, RuntimeCapabilityError>> {
        Box::pin(async move {
            let requested = Instant::now();
            let stats = self.docker.read_sample(id, false, cancel).await?;
            if let Some(reading) = CpuReading::from_stats(id, &stats, requested) {
                self.next
                    .lock()
                    .expect("CPU baselines poisoned")
                    .insert(id.to_owned(), reading);
            }
            Ok(map_container_stat(id, stats))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_proof_rounds_down_and_rejects_unknown_status() {
        for (status, minimum) in [
            ("Up Less than a second", None),
            ("Up 2 seconds (health: starting)", Some(2)),
            ("Up About a minute", Some(60)),
            ("Up 2 hours (healthy)", Some(3600)),
            ("Up 2 days", Some(47 * 3600)),
            ("Up 3 epochs", None),
            ("Up 18446744073709551615 years", None),
            ("Exited (0) 10 days ago", None),
        ] {
            assert_eq!(
                minimum_uptime(status).map(|d| d.as_secs()),
                minimum,
                "{status}"
            );
        }
    }
}
