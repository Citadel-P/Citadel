//! Optional long-range I/O summaries, serialized with the existing minute compactor.
use super::maintenance::{Discovery, HistoryFamily};
use sqlx::PgPool;
use tokio::sync::Mutex;

pub(crate) async fn compact(
    pool: &PgPool,
    discovery: &Mutex<Discovery>,
) -> Result<citadel_runtime::maintenance::CompactionProgress, sqlx::Error> {
    let now = chrono::Utc::now();
    let retained_from = now.timestamp() - 604_800 + i64::from(now.timestamp_subsec_nanos() > 0);
    let mut tx = super::maintenance::transaction(pool).await?;
    // This aggregation never benefits from paying JIT compilation per small block.
    sqlx::query("SET LOCAL jit=off").execute(&mut *tx).await?;
    let mut scan = discovery.lock().await;
    let (candidate, pending) = scan
        .next(&mut tx, HistoryFamily::Coarse, 0, retained_from)
        .await?;

    let Some((source, platform)) = candidate else {
        tx.commit().await?;
        return Ok(citadel_runtime::maintenance::CompactionProgress {
            scan_pending: scan.committed(0, pending),
            ..Default::default()
        });
    };
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .fetch_optional(&mut *tx)
        .await?;
    // A fresh snapshot after the lock prevents replaying a block another caller
    // completed while this transaction waited. Cancellation rolls back the rows
    // and watermark together; the advisory cursor does not replace durable state.
    let bounds: Option<(i64, uuid::Uuid, Option<i64>, Option<i64>)> =
        sqlx::query_as(include_str!("platform_coarse_bounds.sql"))
            .bind(retained_from)
            .bind(source)
            .bind(source - 1)
            .fetch_optional(&mut *tx)
            .await?;
    let progress = if let Some((_, _, Some(from), Some(until))) = bounds
        && from < until
    {
        sqlx::query(include_str!("platform_coarse.sql"))
            .bind(source)
            .bind(from)
            .bind(until)
            .execute(&mut *tx)
            .await?;
        1
    } else {
        0
    };
    tx.commit().await?;
    Ok(citadel_runtime::maintenance::CompactionProgress {
        processed: progress,
        scan_pending: scan.committed(progress, pending),
        ..Default::default()
    })
}
