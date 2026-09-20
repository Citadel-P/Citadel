use citadel_platforms::{
    ContainerStatsStore, RuntimeCapabilityError, RuntimeContainerStat, RuntimeErrorKind,
};
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
    let payload = serde_json::to_value(stats).map_err(storage)?;
    let inserted = sqlx::query_scalar::<_, i64>(
                r#"
WITH incoming AS (
    SELECT * FROM jsonb_to_recordset($2::jsonb) AS value(
        "dockerContainerId" text, "memoryActive" double precision,
        "memoryCache" double precision, "cpuUsage" double precision,
        "memoryLimit" double precision, "rxBytes" double precision,
        "txBytes" double precision, created bigint)
), eligible AS (
    SELECT incoming.*, container.id AS persisted_container_id
    FROM incoming
    JOIN containers container
      ON container.platformid = $1
     AND container.dockercontainerid = incoming."dockerContainerId"
     AND container.dockernodeid IS NOT DISTINCT FROM $3
), inserted AS (
    INSERT INTO containerstats (
        id, containerid, memoryactive, memorycache, cpuusage,
        memorylimit, rxbytes, txbytes, created)
    SELECT gen_random_uuid(), eligible.persisted_container_id, eligible."memoryActive",
           eligible."memoryCache", eligible."cpuUsage", eligible."memoryLimit",
           eligible."rxBytes", eligible."txBytes", eligible.created
    FROM eligible
    ON CONFLICT (containerid, created) DO UPDATE SET
        memoryactive = EXCLUDED.memoryactive,
        memorycache = EXCLUDED.memorycache,
        cpuusage = EXCLUDED.cpuusage,
        memorylimit = EXCLUDED.memorylimit,
        rxbytes = EXCLUDED.rxbytes,
        txbytes = EXCLUDED.txbytes
    RETURNING 1
), persisted_service AS (
    INSERT INTO swarmservicestats (
        id, platformid, dockerserviceid, dockertaskid, servicename,
        swarmserviceid, stackid, taskkey, created,
        memoryactive, memorycache, cpuusage, memorylimit, rxbytes, txbytes)
    SELECT gen_random_uuid(), $1, service.dockerserviceid, task.dockertaskid,
           service.name, service.swarmserviceid, service.stackid,
           CASE WHEN task.slot IS NOT NULL THEN 'slot:' || task.slot::text
                ELSE 'node:' || task.dockernodeid END,
           sample.created, sample."memoryActive", sample."memoryCache",
           sample."cpuUsage", sample."memoryLimit", sample."rxBytes", sample."txBytes"
    FROM incoming sample
    JOIN platforms platform ON platform.id = $1
    JOIN swarmtaskprojections task
      ON task.platformid = platform.id
     AND task.dockernodeid = COALESCE($3,
         CASE WHEN platform.connectortype IN ('Local','Agent','EdgeAgent') THEN platform.platformdescriptor::jsonb->>'nodeID' END)
     AND task.dockercontainerid = sample."dockerContainerId"
    JOIN swarmserviceprojections service
      ON service.platformid = task.platformid
     AND service.dockerserviceid = task.dockerserviceid
    WHERE service.ownership <> 'System'
    ON CONFLICT (platformid, dockertaskid, created) DO UPDATE SET
        memoryactive = EXCLUDED.memoryactive, memorycache = EXCLUDED.memorycache,
        cpuusage = EXCLUDED.cpuusage, memorylimit = EXCLUDED.memorylimit,
        rxbytes = EXCLUDED.rxbytes, txbytes = EXCLUDED.txbytes,
        swarmserviceid = EXCLUDED.swarmserviceid, stackid = EXCLUDED.stackid
), platform_sample AS (
    SELECT COALESCE(SUM("memoryActive"), 0) AS memory_active,
           COALESCE(SUM("cpuUsage"), 0) AS cpu_usage,
           COALESCE(SUM("rxBytes"), 0) AS rx_bytes,
           COALESCE(SUM("txBytes"), 0) AS tx_bytes,
           COALESCE(created, $7::bigint) AS created
    FROM eligible
    GROUP BY created
    UNION ALL SELECT 0,0,0,0,$7::bigint WHERE $7::bigint IS NOT NULL AND NOT EXISTS(SELECT 1 FROM eligible)
), persisted_platform AS (
    INSERT INTO platformstats (
        id, platformid, memoryusage, cpuusage, rxbytes, txbytes, created, diskusedbytes, disktotalbytes, diskusage, alertpending)
    SELECT gen_random_uuid(), platform.id,
           CASE WHEN platform.memtotal > 0
                THEN sample.memory_active / platform.memtotal * 100.0 ELSE 0 END,
           CASE WHEN platform.cpucount > 0
                THEN sample.cpu_usage / platform.cpucount ELSE sample.cpu_usage END,
           sample.rx_bytes, sample.tx_bytes, sample.created, $4, $5, $6, true
    FROM platform_sample sample
    JOIN platforms platform ON platform.id = $1
    WHERE sample.created IS NOT NULL AND $3::text IS NULL
    ON CONFLICT (platformid, created) DO UPDATE SET
        memoryusage = EXCLUDED.memoryusage,
        cpuusage = EXCLUDED.cpuusage,
        rxbytes = EXCLUDED.rxbytes,
        txbytes = EXCLUDED.txbytes,
        diskusedbytes = EXCLUDED.diskusedbytes,
        disktotalbytes = EXCLUDED.disktotalbytes,
        diskusage = EXCLUDED.diskusage,
        alertpending = true
)
SELECT COUNT(*) FROM inserted
"#,
            )
            .bind(platform_id)
            .bind(payload)
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
