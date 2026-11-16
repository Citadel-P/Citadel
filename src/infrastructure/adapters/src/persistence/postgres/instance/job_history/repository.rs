use super::super::repository::storage;
use citadel_instance::{
    InstanceError, JobHistory, JobHistoryBatch, JobHistoryBucket, JobHistoryEvent,
    JobHistoryRecord, JobHistoryRepository, JobHistorySession,
};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Row};

pub struct PostgresJobHistory(pub PgPool);
impl JobHistoryRepository for PostgresJobHistory {
    fn persist<'a>(
        &'a self,
        batch: &'a JobHistoryBatch,
    ) -> BoxFuture<'a, Result<(), InstanceError>> {
        Box::pin(async move {
            let session = &batch.session;
            let snapshot = &session.snapshot;
            let mut tx = self.0.begin().await.map_err(storage)?;
            sqlx::query("SET LOCAL statement_timeout = '3000ms'")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            // A retry of the same captured batch is idempotent, including after an ambiguous commit.
            let updated = sqlx::query("INSERT INTO jobdiagnosticsessions(bootid,coreid,version,heartbeat,stopped,lostevents,snapshot,flushsequence) VALUES($1,$2,$3,$4,$5,$6,$7,$9) ON CONFLICT(bootid) DO UPDATE SET flushsequence=EXCLUDED.flushsequence,heartbeat=EXCLUDED.heartbeat,stopped=EXCLUDED.stopped,lostevents=EXCLUDED.lostevents,snapshot=CASE WHEN $8 THEN EXCLUDED.snapshot ELSE jobdiagnosticsessions.snapshot END WHERE jobdiagnosticsessions.flushsequence < EXCLUDED.flushsequence")
                .bind(snapshot.boot_id).bind(session.core_id).bind(&session.version).bind(session.heartbeat_at as i64).bind(session.stopped_at.map(|v| v as i64)).bind(session.lost_events as i64).bind(sqlx::types::Json(snapshot)).bind(batch.checkpoint).bind(batch.sequence as i64).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if updated != 0 {
                if !batch.events.is_empty() {
                    let events = serde_json::to_value(&batch.events).map_err(storage)?;
                    sqlx::query("INSERT INTO jobdiagnosticevents(bootid,sequence,jobid,observed,kind,problem) SELECT $1,(v->>'sequence')::bigint,v->>'jobId',(v->>'at')::bigint,v->>'kind',NULLIF(v->'problem','null'::jsonb) FROM jsonb_array_elements($2) v ON CONFLICT DO NOTHING")
                        .bind(snapshot.boot_id).bind(events).execute(&mut *tx).await.map_err(storage)?;
                }
                if !batch.buckets.is_empty() {
                    // Session locking serializes checkpoint writes with compaction.
                    // An hourly row seals its inputs; never recreate fine rows.
                    let buckets = serde_json::to_value(&batch.buckets).map_err(storage)?;
                    let sealed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jsonb_array_elements($2) v JOIN jobdiagnosticbuckets b ON b.bootid=$1 AND b.jobid=v->>'jobId' AND b.bucket=((v->>'at')::bigint/3600000)*3600000 AND b.resolutionms=3600000)")
                        .bind(snapshot.boot_id).bind(&buckets).fetch_one(&mut *tx).await.map_err(storage)?;
                    if sealed {
                        return Err(InstanceError::Conflict(
                            "A diagnostic checkpoint overlaps a sealed history hour.".into(),
                        ));
                    }
                    // Checkpoints contain complete windows, not replayable increments.
                    sqlx::query("INSERT INTO jobdiagnosticbuckets(bootid,jobid,bucket,data) SELECT $1,v->>'jobId',(v->>'at')::bigint,v FROM jsonb_array_elements($2) v ON CONFLICT(bootid,jobid,bucket) DO UPDATE SET data=EXCLUDED.data")
                        .bind(snapshot.boot_id).bind(buckets).execute(&mut *tx).await.map_err(storage)?;
                }
            }
            tx.commit().await.map_err(storage)
        })
    }
    fn read<'a>(
        &'a self,
        job: Option<&'a str>,
        hours: u32,
    ) -> BoxFuture<'a, Result<JobHistory, InstanceError>> {
        Box::pin(async move {
            let now = chrono::Utc::now().timestamp_millis() as u64;
            let cutoff = now.saturating_sub(u64::from(hours) * 3_600_000) as i64;
            let mut tx = self.0.begin().await.map_err(storage)?;
            sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            sqlx::query("SET LOCAL statement_timeout = '3000ms'")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            let mut sessions = Vec::new();
            let rows = sqlx::query("SELECT coreid,version,heartbeat,stopped,lostevents,snapshot FROM jobdiagnosticsessions WHERE heartbeat >= $1 ORDER BY heartbeat DESC LIMIT 17")
                .bind(cutoff).fetch_all(&mut *tx).await.map_err(storage)?;
            let mut truncated = rows.len() > 16;
            for row in rows.into_iter().take(16) {
                let mut snapshot: citadel_instance::JobsSnapshot = row
                    .try_get::<sqlx::types::Json<_>, _>("snapshot")
                    .map_err(storage)?
                    .0;
                if let Some(job) = job {
                    snapshot.jobs.retain(|j| j.id == job);
                } else {
                    snapshot.jobs.clear();
                }
                sessions.push(JobHistorySession {
                    core_id: row.try_get("coreid").map_err(storage)?,
                    version: row.try_get("version").map_err(storage)?,
                    heartbeat_at: row.try_get::<i64, _>("heartbeat").map_err(storage)? as u64,
                    stopped_at: row
                        .try_get::<Option<i64>, _>("stopped")
                        .map_err(storage)?
                        .map(|v| v as u64),
                    lost_events: row.try_get::<i64, _>("lostevents").map_err(storage)? as u64,
                    snapshot,
                });
            }
            let boots: Vec<_> = sessions.iter().map(|s| s.snapshot.boot_id).collect();
            let rows = sqlx::query("SELECT bootid,sequence,jobid,observed,kind,problem FROM jobdiagnosticevents WHERE observed >= $1 AND ($2::text IS NULL OR jobid=$2 OR jobid='') AND bootid=ANY($3) ORDER BY observed DESC,sequence DESC LIMIT 201")
                .bind(cutoff).bind(job).bind(&boots).fetch_all(&mut *tx).await.map_err(storage)?;
            truncated |= rows.len() > 200;
            let events = rows
                .into_iter()
                .take(200)
                .map(|row| {
                    Ok(
                            JobHistoryRecord {
                                boot_id: row.try_get("bootid").map_err(storage)?,
                                event:
                                    JobHistoryEvent {
                                        sequence: row
                                            .try_get::<i64, _>("sequence")
                                            .map_err(storage)?
                                            as u64,
                                        job_id: row.try_get("jobid").map_err(storage)?,
                                        at: row.try_get::<i64, _>("observed").map_err(storage)?
                                            as u64,
                                        kind: row.try_get("kind").map_err(storage)?,
                                        problem:
                                            row.try_get::<Option<
                                                sqlx::types::Json<citadel_instance::JobProblem>,
                                            >, _>(
                                                "problem"
                                            )
                                            .map_err(storage)?
                                            .map(|p| p.0),
                                    },
                            },
                        )
                })
                .collect::<Result<Vec<_>, InstanceError>>()?;
            // Trends are fetched for one selected job only; never scan every job's measurements.
            let mut buckets = Vec::new();
            if let Some(job) = job {
                let rows = sqlx::query_scalar::<_,sqlx::types::Json<JobHistoryBucket>>("SELECT data || jsonb_build_object('resolutionMs',resolutionms) FROM jobdiagnosticbuckets WHERE jobid=$1 AND bucket >= $2 AND bucket+resolutionms > $4 AND bootid=ANY($3) ORDER BY bucket DESC LIMIT 2017")
                    .bind(job).bind(cutoff / 3_600_000 * 3_600_000).bind(&boots).bind(cutoff).fetch_all(&mut *tx).await.map_err(storage)?;
                truncated |= rows.len() > 2016;
                buckets = rows.into_iter().take(2016).map(|v| v.0).collect();
            }
            tx.commit().await.map_err(storage)?;
            Ok(JobHistory {
                observed_at: now,
                sessions,
                events,
                buckets,
                truncated,
            })
        })
    }
}
