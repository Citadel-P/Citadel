use citadel_platforms::{
    ContainerStatsStore, RuntimeCapabilityError, RuntimeContainerStat, RuntimeErrorKind,
};
use citadel_runtime::runtime_metrics::RuntimeWork;
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresContainerStatsStore {
    pool: PgPool,
}

impl PostgresContainerStatsStore {
    /// Persist host disk and container-derived metrics in the same platform sample.
    /// Empty Docker hosts still get a platform sample, independent of containers.
    pub async fn persist_with_disk(
        &self,
        platform_id: Uuid,
        stats: &[RuntimeContainerStat],
        disk: Option<citadel_platforms::HostDiskUsage>,
    ) -> Result<usize, RuntimeCapabilityError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        let inserted =
            persist_scoped_with_disk(&mut transaction, platform_id, None, stats, disk).await?;
        transaction.commit().await.map_err(storage)?;
        RuntimeWork::StatsCommit.units(1);
        RuntimeWork::StatsCommittedSamples.units(inserted as u64);
        Ok(inserted)
    }
    pub async fn persist_with_platform_stats(
        &self,
        platform_id: Uuid,
        stats: &[RuntimeContainerStat],
        platform_stats: Option<&citadel_platforms::RuntimePlatformStats>,
    ) -> Result<usize, RuntimeCapabilityError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        if let Some(sample) = platform_stats {
            persist_platform_metadata(&mut transaction, platform_id, sample).await?;
        }
        let inserted = persist_scoped_with_disk(
            &mut transaction,
            platform_id,
            None,
            stats,
            platform_stats.and_then(citadel_platforms::RuntimePlatformStats::disk),
        )
        .await?;
        transaction.commit().await.map_err(storage)?;
        RuntimeWork::StatsCommit.units(1);
        RuntimeWork::StatsCommittedSamples.units(inserted as u64);
        Ok(inserted)
    }
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ContainerStatsStore for PostgresContainerStatsStore {
    fn persist<'a>(
        &'a self,
        platform_id: Uuid,
        stats: &'a [RuntimeContainerStat],
    ) -> BoxFuture<'a, Result<usize, RuntimeCapabilityError>> {
        async move {
            if stats.is_empty() {
                return Ok(0);
            }
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let inserted = persist_scoped(&mut transaction, platform_id, None, stats).await?;
            transaction.commit().await.map_err(storage)?;
            RuntimeWork::StatsCommit.units(1);
            RuntimeWork::StatsCommittedSamples.units(inserted as u64);
            Ok(inserted)
        }
        .boxed()
    }
}

fn storage(error: impl std::fmt::Display) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), true)
}

pub(crate) async fn persist_scoped(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    platform_id: Uuid,
    node_id: Option<&str>,
    stats: &[RuntimeContainerStat],
) -> Result<usize, RuntimeCapabilityError> {
    if stats.is_empty() {
        return Ok(0);
    }
    persist_scoped_with_disk(transaction, platform_id, node_id, stats, None).await
}

pub(crate) async fn persist_scoped_with_disk(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    platform_id: Uuid,
    node_id: Option<&str>,
    stats: &[RuntimeContainerStat],
    disk: Option<citadel_platforms::HostDiskUsage>,
) -> Result<usize, RuntimeCapabilityError> {
    let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::StatsPersistence.start();
    let disk = disk.and_then(|d| {
        citadel_platforms::HostDiskUsage::new(d.used_bytes, d.total_bytes, d.usage_percent)
    });
    let mut identities = std::collections::HashSet::with_capacity(stats.len());
    if stats
        .iter()
        .any(|s| !identities.insert((&s.docker_container_id, s.created)))
    {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::InvalidRequest,
            "Duplicate container statistics in a direct capture",
            false,
        ));
    }
    let payload = serde_json::to_value(stats).map_err(storage)?;
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?;
    let inserted = sqlx::query_scalar::<_, i64>(concat!(
        include_str!("container_direct.sql"),
        include_str!("capture.sql")
    ))
    .bind(platform_id)
    .bind(&payload)
    .bind(node_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(storage)?;
    sqlx::query_scalar::<_, i64>(include_str!("platform_direct.sql"))
        .bind(platform_id)
        .bind(&payload)
        .bind(node_id)
        .bind(disk.map(|d| d.used_bytes))
        .bind(disk.map(|d| d.total_bytes))
        .bind(disk.map(|d| d.usage_percent))
        .bind(stats.is_empty().then(|| chrono::Utc::now().timestamp()))
        .fetch_one(&mut **transaction)
        .await
        .map_err(storage)?;
    Ok(usize::try_from(inserted).unwrap_or(usize::MAX))
}

/// Statistics update descriptor/counts only. Connectivity belongs to the health worker.
pub(crate) async fn persist_platform_metadata(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    platform: Uuid,
    sample: &citadel_platforms::RuntimePlatformStats,
) -> Result<(), RuntimeCapabilityError> {
    let descriptor = serde_json::json!({
        "containerCount": sample.container_count, "containersRunning": sample.containers_running,
        "containersPaused": sample.containers_paused, "containersStopped": sample.containers_stopped,
        "imageUsedBytes": sample.image_used_bytes, "volumeUsedBytes": sample.volume_used_bytes,
    });
    sqlx::query("UPDATE platforms SET networkcount=$2, volumecount=$3, imagecount=$4, memtotal=$5, platformdescriptor=(platformdescriptor::jsonb || $6::jsonb)::json WHERE id=$1 AND (networkcount IS DISTINCT FROM $2 OR volumecount IS DISTINCT FROM $3 OR imagecount IS DISTINCT FROM $4 OR memtotal IS DISTINCT FROM $5 OR NOT platformdescriptor::jsonb @> $6::jsonb)")
        .bind(platform).bind(sample.network_count).bind(sample.volume_count).bind(i32::try_from(sample.image_count).unwrap_or(i32::MAX))
        .bind(sample.mem_total).bind(descriptor).execute(&mut **transaction).await.map_err(storage)?;
    Ok(())
}
