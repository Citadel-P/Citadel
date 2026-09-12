//! Persist samples immediately, then evaluate one smoothed observation per flush.
//! PostgreSQL is the buffer: idle input cannot strand samples or grow an in-memory queue.
use citadel_alerts::AlertEventSink;
use sqlx::{PgPool, Row};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub(super) async fn run(
    cancel: CancellationToken,
    pool: PgPool,
    alerts: Arc<dyn AlertEventSink>,
    interval: Duration,
    batch_size: i64,
) -> Result<(), std::convert::Infallible> {
    let mut last_flush = tokio::time::Instant::now();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! { ()=cancel.cancelled()=>return Ok(()), _=tick.tick()=>{} }
        let flush_batch = async {
            if last_flush.elapsed() < interval {
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM platformstats WHERE alertpending LIMIT $1) samples")
                    .bind(batch_size).fetch_one(&pool).await?;
                if count < batch_size {
                    return Ok::<bool, sqlx::Error>(false);
                }
            }
            flush_pending(&pool, alerts.as_ref(), batch_size).await?;
            Ok(true)
        };
        let result =
            tokio::select! { ()=cancel.cancelled()=>return Ok(()), result=flush_batch=>result };
        match result {
            Ok(true) => {
                last_flush = tokio::time::Instant::now();
            }
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(%error,"Platform threshold batch failed; retrying retained samples")
            }
        }
    }
}

// Lock only the captured samples until their observations have been evaluated.
// A delayed sample or an upsert arriving during a flush stays pending for the
// next batch. There is no wall-clock cursor that can skip older timestamps.
pub(super) async fn flush_pending(
    pool: &PgPool,
    alerts: &dyn AlertEventSink,
    batch_size: i64,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        "SELECT id FROM platformstats WHERE alertpending ORDER BY created,id LIMIT $1 FOR UPDATE SKIP LOCKED",
    ).bind(batch_size.max(1)).fetch_all(&mut *tx).await?;
    if ids.is_empty() {
        return Ok(());
    }
    let rows = sqlx::query("WITH samples AS (SELECT * FROM platformstats WHERE id=ANY($1)), medians AS (SELECT platformid,percentile_cont(0.5) WITHIN GROUP(ORDER BY cpuusage) AS cpuusage,percentile_cont(0.5) WITHIN GROUP(ORDER BY memoryusage) AS memoryusage FROM samples GROUP BY platformid), latest AS (SELECT DISTINCT ON(platformid) platformid,diskusage,diskusedbytes,disktotalbytes FROM samples ORDER BY platformid,created DESC) SELECT p.id,p.name,p.agentversion,m.cpuusage,m.memoryusage,l.diskusage,l.diskusedbytes,l.disktotalbytes FROM medians m JOIN platforms p ON p.id=m.platformid JOIN latest l ON l.platformid=m.platformid")
        .bind(&ids).fetch_all(&mut *tx).await?;
    for row in rows {
        let platform = row.try_get("id")?;
        super::platforms::observe_metric_row(alerts, platform, row)
            .await
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    }
    sqlx::query("UPDATE platformstats SET alertpending=false WHERE id=ANY($1)")
        .bind(&ids)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}
