//! Compact processed host samples and independently scoped node I/O history.
use sqlx::PgPool;

pub(crate) async fn compact(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let now = chrono::Utc::now();
    let cutoff = now.timestamp().saturating_sub(86_400) / 60 * 60;
    let retained_from = now.timestamp() - 604_800 + i64::from(now.timestamp_subsec_nanos() > 0);
    let mut tx = super::maintenance::transaction(pool).await?;
    let candidate: Option<(i64, uuid::Uuid, i64, i64)> =
        sqlx::query_as(include_str!("platform_compaction_bounds.sql"))
            .bind(cutoff)
            .bind(retained_from)
            .bind(None::<i64>)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((source, platform, _, _)) = candidate else {
        tx.commit().await?;
        return Ok(0);
    };
    // Writers take this same lock before checking sessions and sealing watermarks.
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .fetch_optional(&mut *tx)
        .await?;
    // A queued capture can make a sample pending again. Recompute under a fresh
    // snapshot after acquiring the lock, not from the candidate's old snapshot.
    let bounds: Option<(i64, uuid::Uuid, i64, i64)> =
        sqlx::query_as(include_str!("platform_compaction_bounds.sql"))
            .bind(cutoff)
            .bind(retained_from)
            .bind(source)
            .fetch_optional(&mut *tx)
            .await?;
    let mut progress = 0;
    if let Some((_, _, from, until)) = bounds {
        let removed: i64 = sqlx::query_scalar(include_str!("platform_compact.sql"))
            .bind(source)
            .bind(from)
            .bind(until)
            .fetch_one(&mut *tx)
            .await?;
        progress = u64::try_from(removed).unwrap_or_default().max(1);
    }
    tx.commit().await?;
    Ok(progress)
}
