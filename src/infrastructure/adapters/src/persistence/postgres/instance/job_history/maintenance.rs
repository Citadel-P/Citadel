use super::PostgresJobHistory;
use crate::persistence::postgres::instance::repository::storage;
use citadel_instance::{InstanceError, JobHistoryBucket, JobHistoryMaintenance};
use futures_util::future::BoxFuture;
use uuid::Uuid;

impl JobHistoryMaintenance for PostgresJobHistory {
    fn maintain(&self) -> BoxFuture<'_, Result<u64, InstanceError>> {
        Box::pin(async move {
            let now = chrono::Utc::now().timestamp_millis();
            let retained = now - 7 * 86_400_000;
            let cutoff = (now - 86_400_000) / 3_600_000 * 3_600_000;
            let mut tx = self.0.begin().await.map_err(storage)?;
            sqlx::raw_sql("SET LOCAL lock_timeout='500ms'; SET LOCAL statement_timeout='3s'")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            // Expiry is independently bounded, including after downtime. Keep an
            // overlapping boundary hour intact; reads disclose its resolution.
            let mut changed = sqlx::query("DELETE FROM jobdiagnosticevents WHERE (bootid,sequence) IN (SELECT bootid,sequence FROM jobdiagnosticevents WHERE observed<$1 ORDER BY observed LIMIT 1000 FOR UPDATE SKIP LOCKED)")
                .bind(retained).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            changed += sqlx::query("DELETE FROM jobdiagnosticbuckets WHERE (bootid,jobid,bucket) IN (SELECT bootid,jobid,bucket FROM jobdiagnosticbuckets WHERE bucket<$1 AND bucket+resolutionms<=$2 ORDER BY bucket LIMIT 1000 FOR UPDATE SKIP LOCKED)")
                .bind(retained).bind(retained).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            changed += sqlx::query("DELETE FROM jobdiagnosticsessions WHERE bootid IN (SELECT s.bootid FROM jobdiagnosticsessions s WHERE heartbeat<$1 AND NOT EXISTS(SELECT 1 FROM jobdiagnosticevents e WHERE e.bootid=s.bootid) AND NOT EXISTS(SELECT 1 FROM jobdiagnosticbuckets b WHERE b.bootid=s.bootid) LIMIT 16 FOR UPDATE OF s SKIP LOCKED)")
                .bind(retained).execute(&mut *tx).await.map_err(storage)?.rows_affected();

            let candidate: Option<(Uuid, String, i64)> =
                sqlx::query_as(include_str!("compaction_candidate.sql"))
                    .bind(retained / 3_600_000 * 3_600_000)
                    .bind(cutoff)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(storage)?;
            if let Some((boot, job, hour)) = candidate {
                // Reporter checkpoints and this compactor serialize on the session.
                // Its most recent checkpoint hour remains mutable even after an outage.
                let checkpoint: Option<(i64, Option<i64>)> = sqlx::query_as("SELECT (snapshot->>'observedAt')::bigint,stopped FROM jobdiagnosticsessions WHERE bootid=$1 FOR UPDATE")
                    .bind(boot).fetch_optional(&mut *tx).await.map_err(storage)?;
                if checkpoint.is_some_and(|(at, stopped)| {
                    stopped.is_some() || at / 3_600_000 * 3_600_000 >= hour + 3_600_000
                }) {
                    let rows: Vec<sqlx::types::Json<JobHistoryBucket>> = sqlx::query_scalar("SELECT data FROM jobdiagnosticbuckets WHERE bootid=$1 AND jobid=$2 AND bucket >= $3 AND bucket < $3+3600000 AND resolutionms=300000 ORDER BY bucket")
                        .bind(boot).bind(&job).bind(hour).fetch_all(&mut *tx).await.map_err(storage)?;
                    if !rows.is_empty() {
                        let summary = rollup(rows.iter().map(|r| &r.0), hour as u64);
                        let removed = sqlx::query("DELETE FROM jobdiagnosticbuckets WHERE bootid=$1 AND jobid=$2 AND bucket >= $3 AND bucket < $3+3600000 AND resolutionms=300000")
                            .bind(boot).bind(&job).bind(hour).execute(&mut *tx).await.map_err(storage)?.rows_affected();
                        sqlx::query("INSERT INTO jobdiagnosticbuckets(bootid,jobid,bucket,resolutionms,data) VALUES($1,$2,$3,3600000,$4)")
                            .bind(boot).bind(&job).bind(hour).bind(sqlx::types::Json(summary)).execute(&mut *tx).await.map_err(storage)?;
                        changed += removed;
                    }
                }
            }
            tx.commit().await.map_err(storage)?;
            Ok(changed)
        })
    }
}

// Inputs are ordered and share a boot/job/hour. Sum numerators and denominators,
// never means. Preserve unknown units, observed gaps and the latest backlog.
fn rollup<'a>(mut rows: impl Iterator<Item = &'a JobHistoryBucket>, hour: u64) -> JobHistoryBucket {
    let first = rows.next().expect("nonempty source hour");
    let mut result = first.clone();
    let mut previous_at = first.at;
    for next in rows {
        result.iterations += next.iterations;
        result.productive += next.productive;
        result.no_work += next.no_work;
        result.failures += next.failures;
        result.elapsed_ms += next.elapsed_ms;
        result.productive_ms += next.productive_ms;
        result.no_work_ms += next.no_work_ms;
        result.units = if result.unit == next.unit {
            result.units.zip(next.units).map(|(a, b)| a + b)
        } else {
            result.unit = None;
            None
        };
        result.gap |= next.gap || next.at > previous_at + 300_000;
        if next.through >= result.through {
            result.through = next.through;
            result.backlog = next.backlog;
            result.backlog_is_lower_bound = next.backlog_is_lower_bound;
        }
        previous_at = next.at;
    }
    result.at = hour;
    result.resolution_ms = 3_600_000;
    result
}
