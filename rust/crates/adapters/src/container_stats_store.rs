use citadel_platforms::{
    ContainerStatsStore, RuntimeCapabilityError, RuntimeContainerStat, RuntimeErrorKind,
};
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::PgPool;
use uuid::Uuid;

const RETENTION_SECONDS: i64 = 7 * 24 * 60 * 60;

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
    FROM eligible sample
    JOIN containers container ON container.id = sample.persisted_container_id
    JOIN platforms platform ON platform.id = container.platformid
    JOIN swarmtaskprojections task
      ON task.platformid = container.platformid
     AND task.dockernodeid = COALESCE(container.dockernodeid,
         CASE WHEN platform.connectortype IN ('Local','Agent','EdgeAgent') THEN platform.platformdescriptor::jsonb->>'nodeID' END)
     AND task.dockercontainerid = container.dockercontainerid
    JOIN swarmserviceprojections service
      ON service.platformid = task.platformid
     AND service.dockerserviceid = task.dockerserviceid
    WHERE container.isswarmtask AND service.ownership <> 'System'
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
           COALESCE(MAX(created), $7::bigint) AS created
    FROM eligible
), persisted_platform AS (
    INSERT INTO platformstats (
        id, platformid, memoryusage, cpuusage, rxbytes, txbytes, created, diskusedbytes, disktotalbytes, diskusage)
    SELECT gen_random_uuid(), platform.id,
           CASE WHEN platform.memtotal > 0
                THEN sample.memory_active / platform.memtotal * 100.0 ELSE 0 END,
           CASE WHEN platform.cpucount > 0
                THEN sample.cpu_usage / platform.cpucount ELSE sample.cpu_usage END,
           sample.rx_bytes, sample.tx_bytes, sample.created, $4, $5, $6
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
        diskusage = EXCLUDED.diskusage
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
    let retention_cutoff = chrono::Utc::now()
        .timestamp()
        .saturating_sub(RETENTION_SECONDS);
    sqlx::query("DELETE FROM containerstats WHERE id IN (SELECT id FROM containerstats WHERE created < $1 ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)")
                .bind(retention_cutoff)
                .execute(&mut **transaction)
                .await
                .map_err(storage)?;
    sqlx::query("DELETE FROM platformstats WHERE id IN (SELECT id FROM platformstats WHERE created < $1 ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)")
                .bind(retention_cutoff)
                .execute(&mut **transaction)
                .await
                .map_err(storage)?;
    sqlx::query("DELETE FROM swarmservicestats WHERE id IN (SELECT id FROM swarmservicestats WHERE created < $1 ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)")
                .bind(retention_cutoff)
                .execute(&mut **transaction)
                .await
                .map_err(storage)?;
    Ok(usize::try_from(inserted).unwrap_or(usize::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_is_bounded_to_seven_days() {
        assert_eq!(RETENTION_SECONDS, 604_800);
    }
}
