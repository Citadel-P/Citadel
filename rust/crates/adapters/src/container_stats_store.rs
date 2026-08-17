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
            let payload = serde_json::to_value(stats).map_err(storage)?;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
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
), platform_sample AS (
    SELECT COALESCE(SUM("memoryActive"), 0) AS memory_active,
           COALESCE(SUM("cpuUsage"), 0) AS cpu_usage,
           COALESCE(SUM("rxBytes"), 0) AS rx_bytes,
           COALESCE(SUM("txBytes"), 0) AS tx_bytes,
           MAX(created) AS created
    FROM eligible
), persisted_platform AS (
    INSERT INTO platformstats (
        id, platformid, memoryusage, cpuusage, rxbytes, txbytes, created)
    SELECT gen_random_uuid(), platform.id,
           CASE WHEN platform.memtotal > 0
                THEN sample.memory_active / platform.memtotal * 100.0 ELSE 0 END,
           CASE WHEN platform.cpucount > 0
                THEN sample.cpu_usage / platform.cpucount ELSE sample.cpu_usage END,
           sample.rx_bytes, sample.tx_bytes, sample.created
    FROM platform_sample sample
    JOIN platforms platform ON platform.id = $1
    WHERE sample.created IS NOT NULL
    ON CONFLICT (platformid, created) DO UPDATE SET
        memoryusage = EXCLUDED.memoryusage,
        cpuusage = EXCLUDED.cpuusage,
        rxbytes = EXCLUDED.rxbytes,
        txbytes = EXCLUDED.txbytes
)
SELECT COUNT(*) FROM inserted
"#,
            )
            .bind(platform_id)
            .bind(payload)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            let retention_cutoff = chrono::Utc::now()
                .timestamp()
                .saturating_sub(RETENTION_SECONDS);
            sqlx::query("DELETE FROM containerstats WHERE created < $1")
                .bind(retention_cutoff)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            sqlx::query("DELETE FROM platformstats WHERE created < $1")
                .bind(retention_cutoff)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            transaction.commit().await.map_err(storage)?;
            Ok(usize::try_from(inserted).unwrap_or(usize::MAX))
        }
        .boxed()
    }
}

fn storage(error: impl std::fmt::Display) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_is_bounded_to_seven_days() {
        assert_eq!(RETENTION_SECONDS, 604_800);
    }
}
