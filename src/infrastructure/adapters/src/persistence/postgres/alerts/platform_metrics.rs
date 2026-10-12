use citadel_alerts::platform_metrics::{
    PlatformAlertBatchStore, PlatformMetricSample, observe_platform_metrics,
};
use citadel_alerts::{AlertError, AlertEventSink};
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::{PgPool, Row};

pub struct PostgresPlatformAlertBatchStore {
    pool: PgPool,
    compatibility: String,
}
impl PostgresPlatformAlertBatchStore {
    pub fn new(pool: PgPool, core_version: &str) -> Self {
        Self {
            pool,
            compatibility: core_version
                .split('.')
                .take(2)
                .collect::<Vec<_>>()
                .join("."),
        }
    }
    pub async fn observe_latest(
        &self,
        alerts: &dyn AlertEventSink,
        platform: uuid::Uuid,
    ) -> Result<(), AlertError> {
        let row = sqlx::query("SELECT p.name,p.agentversion,s.cpuusage,s.memoryusage,s.diskusage,s.diskusedbytes,s.disktotalbytes FROM platforms p JOIN LATERAL (SELECT * FROM platformstats WHERE platformid=p.id ORDER BY created DESC LIMIT 1) s ON true WHERE p.id=$1")
            .bind(platform).fetch_optional(&self.pool).await.map_err(storage)?;
        if let Some(row) = row {
            observe_platform_metrics(
                alerts,
                platform,
                &sample(&row).map_err(storage)?,
                &self.compatibility,
            )
            .await?;
        }
        Ok(())
    }
}
impl PlatformAlertBatchStore for PostgresPlatformAlertBatchStore {
    fn flush_pending<'a>(
        &'a self,
        alerts: &'a dyn AlertEventSink,
        batch_size: i64,
    ) -> BoxFuture<'a, Result<usize, AlertError>> {
        async move {
            flush_pending(&self.pool, alerts, batch_size, &self.compatibility)
                .await
                .map_err(storage)
        }
        .boxed()
    }
}
fn storage(error: sqlx::Error) -> AlertError {
    AlertError::Storage(error.to_string())
}
fn sample(row: &sqlx::postgres::PgRow) -> Result<PlatformMetricSample, sqlx::Error> {
    let used = row.try_get::<Option<i64>, _>("diskusedbytes")?;
    let total = row.try_get::<Option<i64>, _>("disktotalbytes")?;
    let usage = row.try_get::<Option<f64>, _>("diskusage")?;
    let disk = used
        .zip(total)
        .zip(usage)
        .and_then(|((used, total), usage)| {
            citadel_platforms::HostDiskUsage::new(used, total, usage)
        });
    Ok(PlatformMetricSample {
        name: row.try_get("name")?,
        agent_version: row.try_get("agentversion")?,
        cpu_usage: row.try_get("cpuusage")?,
        memory_usage: row.try_get("memoryusage")?,
        disk_usage: disk.map(|d| d.usage_percent),
        disk_used_bytes: used,
        disk_total_bytes: total,
    })
}

// Serialize flushers with a transaction advisory lock, never sample row locks
// across alert I/O. Capture tuple revisions; concurrent upserts remain pending.
// Rollback/cancellation releases the advisory lock and leaves samples retryable.
async fn flush_pending(
    pool: &PgPool,
    alerts: &dyn AlertEventSink,
    batch_size: i64,
    compatibility: &str,
) -> Result<usize, sqlx::Error> {
    let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::AlertFlush.start();
    let mut tx = pool.begin().await?;
    let claimed: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(4848495441444534)")
        .fetch_one(&mut *tx)
        .await?;
    if !claimed {
        return Ok(0);
    }
    let captured: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id,xmin::text FROM platformstats WHERE alertpending ORDER BY created,id LIMIT $1",
    )
    .bind(batch_size.max(1))
    .fetch_all(&mut *tx)
    .await?;
    let (ids, revisions): (Vec<_>, Vec<_>) = captured.into_iter().unzip();
    citadel_runtime::runtime_metrics::RuntimeWork::AlertFlush.units(ids.len() as u64);
    if ids.is_empty() {
        return Ok(0);
    }
    let rows = sqlx::query("WITH samples AS (SELECT stats.* FROM platformstats stats JOIN unnest($1::uuid[],$2::text[]) claimed(id,revision) ON stats.id=claimed.id AND stats.xmin::text=claimed.revision), medians AS (SELECT platformid,percentile_cont(0.5) WITHIN GROUP(ORDER BY cpuusage) AS cpuusage,percentile_cont(0.5) WITHIN GROUP(ORDER BY memoryusage) AS memoryusage FROM samples GROUP BY platformid), latest AS (SELECT DISTINCT ON(platformid) platformid,diskusage,diskusedbytes,disktotalbytes FROM samples ORDER BY platformid,created DESC) SELECT p.id,p.name,p.agentversion,m.cpuusage,m.memoryusage,l.diskusage,l.diskusedbytes,l.disktotalbytes FROM medians m JOIN platforms p ON p.id=m.platformid JOIN latest l ON l.platformid=m.platformid")
        .bind(&ids).bind(&revisions).fetch_all(&mut *tx).await?;
    for row in rows {
        let platform = row.try_get("id")?;
        observe_platform_metrics(alerts, platform, &sample(&row)?, compatibility)
            .await
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    }
    sqlx::query("UPDATE platformstats stats SET alertpending=false FROM unnest($1::uuid[],$2::text[]) claimed(id,revision) WHERE stats.id=claimed.id AND stats.xmin::text=claimed.revision")
        .bind(&ids)
        .bind(&revisions)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ids.len())
}
