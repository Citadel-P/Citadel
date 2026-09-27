//! Sampling and realtime ingress are independent of the two persistence writers.
use crate::realtime::RealtimeHub;
use citadel_adapters::persistence::postgres::platforms::statistics::batch::PostgresStatsBatchStore;
use citadel_platforms::stats_ingestion::{ContainerStatsSample, PlatformStatsSample, StatsScope};
use citadel_platforms::{RuntimeContainerStat, RuntimePlatformStats};
use citadel_runtime::{TaskSupervisor, runtime_metrics::RuntimeWork};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

mod writer;

#[derive(Clone)]
pub struct StatsIngress {
    containers: writer::Ingress<ContainerStatsSample>,
    platforms: writer::Ingress<PlatformStatsSample>,
    realtime: Option<RealtimeHub>,
}

pub fn register(
    supervisor: &mut TaskSupervisor,
    cancel: &CancellationToken,
    pool: sqlx::PgPool,
    realtime: Option<RealtimeHub>,
    capacity: usize,
    batch_size: usize,
    flush_interval: Duration,
) -> StatsIngress {
    let (containers, container_queue) =
        writer::channel(capacity, RuntimeWork::ContainerStatsIngress);
    let (platforms, platform_queue) = writer::channel(capacity, RuntimeWork::PlatformStatsIngress);
    let store = Arc::new(PostgresStatsBatchStore::new(pool));
    let settings = writer::WriterSettings {
        batch_size,
        flush_interval,
        shutdown_timeout: Duration::from_secs(5),
    };
    supervisor.spawn(
        "container-stats-writer",
        writer::run(
            container_queue,
            store.clone(),
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
            store,
            cancel.clone(),
            settings,
            RuntimeWork::PlatformStatsIngress,
            RuntimeWork::PlatformStatsFlush,
            RuntimeWork::PlatformStatsStale,
        ),
    );
    StatsIngress {
        containers,
        platforms,
        realtime,
    }
}

impl StatsIngress {
    /// Scope and capture timestamps travel with the observations. Node samples
    /// never produce manager Platform totals. Realtime is observational and may
    /// precede a commit; the hub skips payload creation without subscribers.
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
        stats: Vec<RuntimeContainerStat>,
        metadata: Option<RuntimePlatformStats>,
        captured_at: Option<i64>,
        cancel: &CancellationToken,
    ) -> bool {
        if cancel.is_cancelled() {
            return false;
        }
        if let Some(hub) = &self.realtime {
            hub.publish_scoped_container_stats(scope.platform_id, scope.node_id.as_deref(), &stats);
        }
        let mut totals = BTreeMap::<i64, PlatformStatsSample>::new();
        if scope.node_id.is_none() {
            for stat in &stats {
                let sample = totals
                    .entry(captured_at.unwrap_or(stat.created))
                    .or_insert_with(|| PlatformStatsSample {
                        scope: scope.clone(),
                        created: captured_at.unwrap_or(stat.created),
                        memory_active: 0.0,
                        cpu_usage: 0.0,
                        rx_bytes: 0.0,
                        tx_bytes: 0.0,
                        metadata: metadata.clone(),
                    });
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
        for stat in stats {
            if !self
                .containers
                .send(
                    ContainerStatsSample {
                        scope: scope.clone(),
                        sample: stat,
                    },
                    cancel,
                )
                .await
            {
                return false;
            }
        }
        for sample in totals.into_values() {
            if !self.platforms.send(sample, cancel).await {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests;
