//! Compact processed host samples and independently scoped node I/O history.
use citadel_runtime::runtime_metrics::RuntimeWork;
use sqlx::PgPool;
use std::sync::atomic::{AtomicI64, Ordering};
const OBSERVATIONS: i64 = 100_000;
const INPUT_BYTES: i64 = 32 * 1024 * 1024;

pub(crate) async fn compact(pool: &PgPool, cursor: &AtomicI64) -> Result<u64, sqlx::Error> {
    let now = chrono::Utc::now();
    let cutoff = now.timestamp().saturating_sub(86_400) / 60 * 60;
    let retained_from = now.timestamp() - 604_800 + i64::from(now.timestamp_subsec_nanos() > 0);
    let mut tx = super::maintenance::transaction(pool).await?;
    let discovery = RuntimeWork::CompactionDiscovery.start();
    let candidate: Option<(i64, uuid::Uuid, i64, i64)> =
        sqlx::query_as(include_str!("platform_compaction_bounds.sql"))
            .bind(cutoff)
            .bind(retained_from)
            .bind(None::<i64>)
            .bind(cursor.load(Ordering::Relaxed))
            .fetch_optional(&mut *tx)
            .await?;
    drop(discovery);
    let Some((source, platform, oldest, _)) = candidate else {
        tx.commit().await?;
        return Ok(0);
    };
    cursor.store(source, Ordering::Relaxed);
    RuntimeWork::CompactionDiscovery.age(std::time::Duration::from_secs(
        now.timestamp().saturating_sub(oldest).max(0) as u64,
    ));
    let wait = RuntimeWork::CompactionLockWait.start();
    // Writers take this same lock before checking sessions and sealing watermarks.
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .fetch_optional(&mut *tx)
        .await?;
    drop(wait);
    let hold = RuntimeWork::CompactionLockHold.start();
    // A queued capture can make a sample pending again. Recompute under a fresh
    // snapshot after acquiring the lock, not from the candidate's old snapshot.
    let bounds: Option<(i64, uuid::Uuid, i64, i64)> =
        sqlx::query_as(include_str!("platform_compaction_bounds.sql"))
            .bind(cutoff)
            .bind(retained_from)
            .bind(source)
            .bind(0i64)
            .fetch_optional(&mut *tx)
            .await?;
    let mut progress = 0;
    if let Some((_, _, from, until)) = bounds {
        let sizing = RuntimeWork::CompactionSizing.start();
        let mut end = from / 60 * 60;
        let mut observations = 0;
        let mut bytes = 0;
        let mut captures = 0;
        while end < until {
            let next = (end + 60).min(until);
            let (count, items, size): (i64, i64, i64) =
                sqlx::query_as(include_str!("platform_compaction_size.sql"))
                    .bind(source)
                    .bind(end.max(from))
                    .bind(next)
                    .fetch_one(&mut *tx)
                    .await?;
            if observations + items > OBSERVATIONS || bytes + size > INPUT_BYTES {
                if end <= from {
                    RuntimeWork::CompactionStalled.units(1);
                    tracing::warn!(
                        source,
                        minute = end,
                        observations = items,
                        input_bytes = size,
                        "Statistics minute exceeds compaction budget; left unsealed, later sources remain eligible"
                    );
                }
                break;
            }
            captures += count;
            observations += items;
            bytes += size;
            end = next;
        }
        drop(sizing);
        if end <= from {
            tx.commit().await?;
            // Stop the enclosing drain even if physical-container compaction
            // made progress. The existing failure pause prevents retrying this
            // dense source repeatedly within the same five-second batch.
            return Err(sqlx::Error::Protocol(format!(
                "Statistics source {source} minute {} exceeds the complete-minute compaction budget ({OBSERVATIONS} observations, {INPUT_BYTES} JSON bytes)",
                from / 60 * 60
            )));
        }
        RuntimeWork::CompactionInput.units(observations as u64);
        RuntimeWork::CompactionInput.queued(captures as usize);
        RuntimeWork::CompactionInput.buffered(bytes as usize);
        let removed: i64 = sqlx::query_scalar(include_str!("platform_compact.sql"))
            .bind(source)
            .bind(from)
            .bind(end)
            .fetch_one(&mut *tx)
            .await?;
        progress = u64::try_from(removed).unwrap_or_default().max(1);
    }
    tx.commit().await?;
    drop(hold);
    RuntimeWork::CompactionProgress.units(progress);
    Ok(progress)
}
