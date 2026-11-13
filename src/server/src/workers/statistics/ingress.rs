//! Sampling and realtime ingress are independent of the two persistence writers.
use crate::realtime::RealtimeHub;
use citadel_platforms::stats_ingestion::StatsBatchStore;
use citadel_platforms::stats_ingestion::{ContainerStatsSample, PlatformStatsSample, StatsScope};
use citadel_platforms::{RuntimeContainerStat, RuntimePlatformStats};
use citadel_runtime::{TaskSupervisor, runtime_metrics::RuntimeWork};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

use super::writer;

#[derive(Clone)]
pub struct StatsIngress {
    pub(super) containers: writer::Ingress<ContainerStatsSample>,
    pub(super) platforms: writer::Ingress<PlatformStatsSample>,
    pub(super) budget: super::budget::Budget,
    pub(super) realtime: Option<RealtimeHub>,
}

pub fn register(
    supervisor: &mut TaskSupervisor,
    cancel: &CancellationToken,
    stores: (
        Arc<dyn StatsBatchStore<ContainerStatsSample>>,
        Arc<dyn StatsBatchStore<PlatformStatsSample>>,
    ),
    realtime: Option<RealtimeHub>,
    capacity: usize,
    batch_size: usize,
    flush_interval: Duration,
) -> StatsIngress {
    let (containers, container_queue) =
        writer::channel(capacity, RuntimeWork::ContainerStatsIngress);
    let (platforms, platform_queue) = writer::channel(capacity, RuntimeWork::PlatformStatsIngress);
    let settings = writer::WriterSettings {
        batch_size,
        flush_interval,
        shutdown_timeout: Duration::from_secs(5),
    };
    supervisor.spawn(
        "container-stats-writer",
        writer::run(
            container_queue,
            stores.0,
            cancel.clone(),
            settings,
            RuntimeWork::ContainerStatsIngress,
            RuntimeWork::ContainerStatsFlush,
            RuntimeWork::ContainerStatsStale,
        ),
    );
    supervisor.spawn(
        "platform-stats-writer",
        writer::run(
            platform_queue,
            stores.1,
            cancel.clone(),
            settings,
            RuntimeWork::PlatformStatsIngress,
            RuntimeWork::PlatformStatsFlush,
            RuntimeWork::PlatformStatsStale,
        ),
    );
    StatsIngress {
        budget: Default::default(),
        containers,
        platforms,
        realtime,
    }
}

impl StatsIngress {
    /// Scope and capture timestamps travel with observations. Node captures retain
    /// their own I/O history, separate from manager gauges. Realtime is observational
    /// and may precede a commit; the hub skips payload creation without subscribers.
    pub async fn submit(
        &self,
        scope: StatsScope,
        stats: Vec<RuntimeContainerStat>,
        metadata: Option<RuntimePlatformStats>,
        cancel: &CancellationToken,
    ) -> bool {
        self.submit_inner(scope, stats, metadata, None, cancel)
            .await
    }

    /// One discovered cycle produces one host total; individual capture times remain intact.
    pub async fn submit_cycle(
        &self,
        scope: StatsScope,
        stats: Vec<RuntimeContainerStat>,
        metadata: Option<RuntimePlatformStats>,
        captured_at: i64,
        cancel: &CancellationToken,
    ) -> bool {
        self.submit_inner(scope, stats, metadata, Some(captured_at), cancel)
            .await
    }

    async fn submit_inner(
        &self,
        scope: StatsScope,
        mut stats: Vec<RuntimeContainerStat>,
        metadata: Option<RuntimePlatformStats>,
        captured_at: Option<i64>,
        cancel: &CancellationToken,
    ) -> bool {
        if cancel.is_cancelled() {
            return false;
        }
        // Bound preflight's timestamp map before constructing it. Even the
        // scalar representations alone cannot fit if these lower bounds exceed
        // the budget; no payload serialization is needed to reject that case.
        let scalar_bytes = stats.len().saturating_mul(
            2 * std::mem::size_of::<ContainerStatsSample>()
                + 2 * std::mem::size_of::<citadel_platforms::ContainerIoSnapshot>(),
        );
        if scalar_bytes > super::budget::RETAINED_BYTES
            || stats
                .capacity()
                .saturating_mul(std::mem::size_of::<RuntimeContainerStat>())
                > super::budget::RETAINED_BYTES
        {
            self.budget.reject(true);
            return true;
        }
        // Reserve before constructing totals or cloned scope/IO representations.
        // Admission is immediate: waiting collectors cannot accumulate owned input.
        let scope_bytes = super::budget::scope_bytes(&scope);
        // The capture and narrow metadata statements each carry scope data.
        let capture_base = scope_bytes.saturating_mul(2).saturating_add(1024);
        let mut capture_sizes = BTreeMap::<i64, usize>::new();
        let mut estimates = Vec::with_capacity(stats.len());
        let oversized = stats.iter().any(|stat| {
            let bytes = capture_sizes
                .entry(captured_at.unwrap_or(stat.created))
                .or_insert(capture_base);
            *bytes = bytes.saturating_add(super::budget::io_bytes(stat));
            if *bytes > super::budget::BATCH_BYTES {
                return true;
            }
            let encoded = super::budget::container_bytes(scope_bytes, stat);
            estimates.push(encoded);
            encoded > super::budget::BATCH_BYTES
        }) || capture_base > super::budget::BATCH_BYTES;
        if oversized {
            self.budget.reject(true);
            // False stops the owning sampler; rejected telemetry must not stop collection.
            return true;
        }
        let (bytes, container_bytes, platform_bytes) = super::budget::cycle_bytes(
            &scope,
            &stats,
            &estimates,
            capture_sizes.len(),
            capture_sizes
                .values()
                .copied()
                .fold(0usize, usize::saturating_add)
                .max(capture_base),
        );
        let Some(mut lease) = self.budget.reserve(bytes) else {
            self.budget.reject(bytes > super::budget::RETAINED_BYTES);
            return true;
        };
        let container_lease = super::budget::Lease::split(&mut lease, container_bytes);
        let platform_lease = super::budget::Lease::split(&mut lease, platform_bytes);
        if let Some(oldest) = stats.iter().map(|s| s.created).min() {
            RuntimeWork::ContainerStatsIngress.age(Duration::from_secs(
                chrono::Utc::now().timestamp().saturating_sub(oldest).max(0) as u64,
            ));
        }
        let original_count = stats.len();
        let mut index = 0;
        let mut retained = 0;
        stats.retain(|s| {
            let valid = s.created >= 0
                && !s.docker_container_id.is_empty()
                && [
                    s.memory_active,
                    s.memory_cache,
                    s.memory_limit,
                    s.cpu_usage,
                    s.rx_bytes,
                    s.tx_bytes,
                ]
                .into_iter()
                .all(|v| v.is_finite() && v >= 0.0)
                && s.memory_active_cache
                    .is_none_or(|v| v.is_finite() && v >= 0.0 && v <= s.memory_active)
                && [s.block_read_bytes, s.block_write_bytes]
                    .into_iter()
                    .flatten()
                    .all(|v| v.is_finite() && v >= 0.0);
            if !valid {
                RuntimeWork::StatsRejected.units(1);
            }
            if valid {
                estimates[retained] = estimates[index];
                retained += 1;
            }
            index += 1;
            valid
        });
        estimates.truncate(retained);
        let metadata = metadata.filter(|m| {
            let valid = m.mem_total >= 0
                && m.container_count >= 0
                && m.image_count >= 0
                && m.disk_usage
                    .is_none_or(|v| v.is_finite() && (0.0..=100.0).contains(&v));
            if !valid {
                RuntimeWork::StatsRejected.units(1);
            }
            valid
        });
        let mut totals = BTreeMap::<i64, PlatformStatsSample>::new();
        if stats.len() == original_count {
            for stat in &stats {
                let sample = totals
                    .entry(captured_at.unwrap_or(stat.created))
                    .or_insert_with(|| PlatformStatsSample {
                        scope: scope.clone(),
                        container_io: Vec::new(),
                        created: captured_at.unwrap_or(stat.created),
                        memory_active: 0.0,
                        cpu_usage: 0.0,
                        rx_bytes: 0.0,
                        tx_bytes: 0.0,
                        metadata: metadata.clone(),
                    });
                sample.container_io.push(stat.into());
                sample.memory_active += stat.memory_active;
                sample.cpu_usage += stat.cpu_usage;
                sample.rx_bytes += stat.rx_bytes;
                sample.tx_bytes += stat.tx_bytes;
            }
            if stats.is_empty() {
                let created = captured_at.unwrap_or_else(|| chrono::Utc::now().timestamp());
                totals.insert(
                    created,
                    PlatformStatsSample {
                        scope: scope.clone(),
                        container_io: Vec::new(),
                        created,
                        memory_active: 0.0,
                        cpu_usage: 0.0,
                        rx_bytes: 0.0,
                        tx_bytes: 0.0,
                        metadata,
                    },
                );
            }
        }
        totals.retain(|_, t| {
            let valid = [t.memory_active, t.cpu_usage, t.rx_bytes, t.tx_bytes]
                .into_iter()
                .all(f64::is_finite);
            if !valid {
                RuntimeWork::StatsRejected.units(1);
            }
            valid
        });
        if let Some(hub) = &self.realtime {
            hub.publish_scoped_container_stats_with_total(
                scope.platform_id,
                scope.node_id.as_deref(),
                &stats,
                totals.last_key_value().map(|(_, sample)| sample),
            );
        }
        for (stat, encoded) in stats.into_iter().zip(estimates) {
            if !self
                .containers
                .send_owned(
                    ContainerStatsSample {
                        scope: scope.clone(),
                        sample: stat,
                    },
                    Some(container_lease.clone()),
                    encoded,
                    cancel,
                )
                .await
            {
                return false;
            }
        }
        for sample in totals.into_values() {
            let encoded = capture_sizes
                .get(&sample.created)
                .copied()
                .unwrap_or(capture_base);
            if !self
                .platforms
                .send_owned(sample, Some(platform_lease.clone()), encoded, cancel)
                .await
            {
                return false;
            }
        }
        true
    }
}
