//! Evaluate one smoothed observation per committed Platform writer batch.
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
    mut signals: tokio::sync::watch::Receiver<()>,
) -> Result<(), std::convert::Infallible> {
    // Commit notifications avoid adding another full flush interval of alert
    // latency. PostgreSQL retains pending rows across missed signals/restarts.
    loop {
        super::notifications::wait(&mut signals, &cancel, interval).await;
        if cancel.is_cancelled() {
            return Ok(());
        }
        let drain = async {
            for _ in 0..10 {
                let count = flush_pending(&pool, alerts.as_ref(), batch_size).await?;
                if count < batch_size.max(1) as usize {
                    break;
                }
            }
            Ok::<_, sqlx::Error>(())
        };
        tokio::select! {
            () = cancel.cancelled() => return Ok(()),
            result = tokio::time::timeout(Duration::from_secs(5), drain) => match result {
                Ok(Ok(())) => {},
                Ok(Err(error)) => tracing::warn!(%error, "Platform threshold batch failed; retaining samples"),
                Err(_) => tracing::warn!("Platform threshold flush exhausted its time budget; retaining samples"),
            }
        }
    }
}

// Serialize flushers with a transaction advisory lock, never sample row locks
// across alert I/O. Capture tuple revisions; concurrent upserts remain pending.
// Rollback/cancellation releases the advisory lock and leaves samples retryable.
pub(super) async fn flush_pending(
    pool: &PgPool,
    alerts: &dyn AlertEventSink,
    batch_size: i64,
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
        super::platforms::observe_metric_row(alerts, platform, row)
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
