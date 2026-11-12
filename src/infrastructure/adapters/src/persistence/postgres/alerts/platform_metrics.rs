use citadel_alerts::platform_metrics::{
    PlatformAlertBatchStore, PlatformMetricSample, observe_platform_metrics,
    platform_metric_observations,
};
use citadel_alerts::{AlertError, AlertEventSink};
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::{PgPool, Row};

pub struct PostgresPlatformAlertBatchStore {
    pool: PgPool,
    core_version: String,
    alerts: std::sync::Arc<super::PostgresAlertRepository>,
}
impl PostgresPlatformAlertBatchStore {
    pub fn new(
        pool: PgPool,
        core_version: &str,
        alerts: std::sync::Arc<super::PostgresAlertRepository>,
    ) -> Self {
        Self {
            pool,
            alerts,
            core_version: core_version.to_owned(),
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
                &self.core_version,
            )
            .await?;
        }
        Ok(())
    }
}
impl PlatformAlertBatchStore for PostgresPlatformAlertBatchStore {
    fn backlog(
        &self,
    ) -> BoxFuture<'_, Result<citadel_alerts::platform_metrics::PlatformAlertBacklog, AlertError>>
    {
        Box::pin(async move {
            let (count, oldest): (i64, Option<i64>) = sqlx::query_as("WITH pending AS MATERIALIZED (SELECT created FROM platformstats WHERE alertpending ORDER BY created LIMIT 10001) SELECT count(*),min(created) FROM pending")
                .fetch_one(&self.pool).await.map_err(storage)?;
            let cutoff = (chrono::Utc::now().timestamp() - 86400) / 60 * 60;
            Ok(citadel_alerts::platform_metrics::PlatformAlertBacklog {
                pending: count as usize,
                capped: count == 10001,
                oldest_created: oldest,
                blocks_compaction: oldest.is_some_and(|created| created < cutoff),
            })
        })
    }
    fn flush_pending<'a>(&'a self, batch_size: i64) -> BoxFuture<'a, Result<usize, AlertError>> {
        async move {
            flush_pending(&self.pool, &self.alerts, batch_size, &self.core_version)
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
    alerts: &super::PostgresAlertRepository,
    batch_size: i64,
    core_version: &str,
) -> Result<usize, sqlx::Error> {
    let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::AlertFlush.start();
    // Cached rule definitions and live entitlements may read the pool. Resolve
    // them before holding the flush connection/advisory lock, including cold caches.
    let mut rules = std::collections::HashMap::new();
    for kind in [
        "PlatformVersionMismatch",
        "PlatformCpuHigh",
        "PlatformRamHigh",
        "PlatformDiskHigh",
    ] {
        rules.insert(
            kind,
            alerts
                .configured_rules(kind)
                .await
                .map_err(|e| sqlx::Error::Protocol(e.to_string()))?,
        );
    }
    let custom = rules.values().any(|rules| {
        rules
            .iter()
            .any(|r| r.audit.created_by_actor_id.value() != uuid::Uuid::from_u128(1))
    });
    let advanced = !custom
        || alerts
            .entitlements
            .advanced_alerting()
            .await
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
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
    let rows = sqlx::query("WITH samples AS (SELECT stats.platformid,stats.created,stats.cpuusage,stats.memoryusage,stats.diskusage,stats.diskusedbytes,stats.disktotalbytes FROM platformstats stats JOIN unnest($1::uuid[],$2::text[]) claimed(id,revision) ON stats.id=claimed.id AND stats.xmin::text=claimed.revision), medians AS (SELECT platformid,percentile_cont(0.5) WITHIN GROUP(ORDER BY cpuusage) AS cpuusage,percentile_cont(0.5) WITHIN GROUP(ORDER BY memoryusage) AS memoryusage FROM samples GROUP BY platformid), latest AS (SELECT DISTINCT ON(platformid) platformid,diskusage,diskusedbytes,disktotalbytes FROM samples ORDER BY platformid,created DESC) SELECT p.id,p.name,p.agentversion,m.cpuusage,m.memoryusage,l.diskusage,l.diskusedbytes,l.disktotalbytes FROM medians m JOIN platforms p ON p.id=m.platformid JOIN latest l ON l.platformid=m.platformid")
        .bind(&ids).bind(&revisions).fetch_all(&mut *tx).await?;
    let mut changed = false;
    for row in rows {
        let platform = row.try_get("id")?;
        for observation in platform_metric_observations(platform, &sample(&row)?, core_version) {
            let (resets, winner) = citadel_alerts::rules::evaluation::select_observation_rules(
                &rules[observation.alert_type.as_str()],
                &observation,
                advanced,
            );
            for rule in resets.into_iter().chain(winner) {
                alerts
                    .process_rule_in(&mut tx, &mut changed, rule, &observation, None)
                    .await
                    .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
            }
        }
    }
    let acknowledged = sqlx::query("UPDATE platformstats stats SET alertpending=false FROM unnest($1::uuid[],$2::text[]) claimed(id,revision) WHERE stats.id=claimed.id AND stats.xmin::text=claimed.revision")
        .bind(&ids)
        .bind(&revisions)
        .execute(&mut *tx)
        .await?.rows_affected();
    tx.commit().await?;
    if changed {
        alerts.changed();
    }
    citadel_runtime::runtime_metrics::RuntimeWork::AlertFlush.success();
    Ok(acknowledged as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn batch_store_preserves_full_core_version_for_observations() {
        // No connection is opened: only the version supplied to the store matters.
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@localhost/unused")
            .unwrap();
        for version in ["0.1.1", "0.1.1-dev.2+source.abc"] {
            let store = PostgresPlatformAlertBatchStore::new(
                pool.clone(),
                version,
                std::sync::Arc::new(super::super::PostgresAlertRepository::new(pool.clone())),
            );
            assert_eq!(store.core_version, version);
        }
        pool.close().await;
    }
}
