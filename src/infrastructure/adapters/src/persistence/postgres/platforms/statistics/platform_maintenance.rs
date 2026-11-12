//! Compact processed host samples and independently scoped node I/O history.
use citadel_runtime::{maintenance::CompactionProgress, runtime_metrics::RuntimeWork};
use sqlx::PgPool;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};
const SIZING_BUDGET: Duration = Duration::from_secs(1);
const OBSERVATIONS: i64 = 100_000;
const INPUT_BYTES: i64 = 32 * 1024 * 1024;

pub(crate) async fn compact(
    pool: &PgPool,
    cursor: &AtomicI64,
) -> Result<CompactionProgress, sqlx::Error> {
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
        return Ok(CompactionProgress::default());
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
        let started = Instant::now();
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
            // Leave most of the worker's five-second batch for expansion and
            // commit. A slow sizing query still has the transaction's timeout.
            if started.elapsed() >= SIZING_BUDGET {
                break;
            }
        }
        drop(sizing);
        if end <= from {
            tx.commit().await?;
            return Ok(CompactionProgress {
                processed: 0,
                stalled_source: Some(source),
            });
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
    Ok(CompactionProgress {
        processed: progress,
        stalled_source: None,
    })
}
