//! Optional long-range I/O summaries, serialized with the existing minute compactor.
use sqlx::PgPool;
use std::sync::atomic::{AtomicI64, Ordering};

pub(crate) async fn compact(pool: &PgPool, cursor: &AtomicI64) -> Result<Option<u64>, sqlx::Error> {
    let now = chrono::Utc::now();
    let retained_from = now.timestamp() - 604_800 + i64::from(now.timestamp_subsec_nanos() > 0);
    let mut tx = super::maintenance::transaction(pool).await?;
    // This aggregation never benefits from paying JIT compilation per small block.
    sqlx::query("SET LOCAL jit=off").execute(&mut *tx).await?;
    let candidate: Option<(i64, uuid::Uuid, i64, i64)> =
        sqlx::query_as(include_str!("platform_coarse_bounds.sql"))
            .bind(retained_from)
            .bind(None::<i64>)
            .bind(cursor.load(Ordering::Relaxed))
            .fetch_optional(&mut *tx)
            .await?;
    let Some((source, platform, _, _)) = candidate else {
        tx.commit().await?;
        return Ok(None);
    };
    cursor.store(source, Ordering::Relaxed);
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .fetch_optional(&mut *tx)
        .await?;
    // A fresh snapshot after the lock prevents replaying a block another caller
    // completed while this transaction waited. Cancellation rolls back the rows
    // and watermark together; the advisory cursor does not replace durable state.
    let bounds: Option<(i64, uuid::Uuid, i64, i64)> =
        sqlx::query_as(include_str!("platform_coarse_bounds.sql"))
            .bind(retained_from)
            .bind(source)
            .bind(0i64)
            .fetch_optional(&mut *tx)
            .await?;
    let progress = if let Some((_, _, from, until)) = bounds {
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
    Ok(Some(progress))
}
