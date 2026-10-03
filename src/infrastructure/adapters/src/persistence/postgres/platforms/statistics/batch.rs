//! Set-based writes across sources; scope locks fence reconfiguration/revocation.
use citadel_platforms::stats_ingestion::{
    ContainerStatsSample, PlatformStatsSample, StatsBatchStore, StatsScope, StatsWriteOutcome,
};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use citadel_runtime::runtime_metrics::RuntimeWork;
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

#[derive(Clone)]
pub struct PostgresStatsBatchStore {
    pool: PgPool,
}
impl PostgresStatsBatchStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    let permanent = error.as_database_error().is_some_and(|error| {
        error.code().is_some_and(|code| {
            code.starts_with("22") || code.starts_with("23") || code.starts_with("42")
        })
    });
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), !permanent)
}

async fn lock_scopes(
    tx: &mut Transaction<'_, Postgres>,
    payload: &Value,
) -> Result<(), RuntimeCapabilityError> {
    sqlx::query("SET LOCAL lock_timeout = '500ms'")
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    // Stable parent order also prevents metadata lock upgrades between writers.
    sqlx::query("SELECT p.id FROM platforms p WHERE p.id IN (SELECT platform_id FROM jsonb_to_recordset($1) AS i(platform_id uuid)) ORDER BY p.id FOR NO KEY UPDATE")
        .bind(payload).fetch_all(&mut **tx).await.map_err(storage)?;
    if payload
        .as_array()
        .unwrap()
        .iter()
        .any(|row| !row["agent_id"].is_null())
    {
        sqlx::query("SELECT b.agentid FROM edgeagentbindings b WHERE b.agentid IN (SELECT agent_id FROM jsonb_to_recordset($1) AS i(agent_id uuid)) ORDER BY b.agentid FOR SHARE")
        .bind(payload).fetch_all(&mut **tx).await.map_err(storage)?;
    }
    Ok(())
}

// A session may close while the transaction waits for a scope lock.
fn prune_closed<'a>(payload: &mut Value, mut scopes: impl Iterator<Item = &'a StatsScope>) {
    payload.as_array_mut().unwrap().retain(|_| {
        !scopes
            .next()
            .unwrap()
            .closed
            .as_ref()
            .is_some_and(|token| token.is_cancelled())
    });
}

impl StatsBatchStore<ContainerStatsSample> for PostgresStatsBatchStore {
    fn partition(&self, sample: &ContainerStatsSample) -> uuid::Uuid {
        sample.scope.platform_id
    }
    fn persist_batch<'a>(
        &'a self,
        samples: &'a [ContainerStatsSample],
    ) -> BoxFuture<'a, Result<StatsWriteOutcome, RuntimeCapabilityError>> {
        Box::pin(async move {
            if samples.is_empty() {
                return Ok(StatsWriteOutcome::default());
            }
            let live: Vec<_> = samples
                .iter()
                .filter(|s| !s.scope.closed.as_ref().is_some_and(|t| t.is_cancelled()))
                .collect();
            let mut payload = serde_json::to_value(&live).map_err(|e| {
                RuntimeCapabilityError::new(RuntimeErrorKind::InvalidRequest, e.to_string(), false)
            })?;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            lock_scopes(&mut tx, &payload).await?;
            prune_closed(&mut payload, live.iter().map(|s| &s.scope));
            let accepted: i64 = sqlx::query_scalar(include_str!("container_batch.sql"))
                .bind(payload)
                .fetch_one(&mut *tx)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            RuntimeWork::StatsCommit.units(1);
            RuntimeWork::StatsCommittedSamples.units(accepted as u64);
            Ok(StatsWriteOutcome {
                persisted: accepted as usize,
                stale: samples.len().saturating_sub(accepted as usize),
            })
        })
    }
}

impl StatsBatchStore<PlatformStatsSample> for PostgresStatsBatchStore {
    fn partition(&self, sample: &PlatformStatsSample) -> uuid::Uuid {
        sample.scope.platform_id
    }
    fn persist_batch<'a>(
        &'a self,
        samples: &'a [PlatformStatsSample],
    ) -> BoxFuture<'a, Result<StatsWriteOutcome, RuntimeCapabilityError>> {
        Box::pin(async move {
            if samples.is_empty() {
                return Ok(StatsWriteOutcome::default());
            }
            let live: Vec<_> = samples
                .iter()
                .filter(|s| !s.scope.closed.as_ref().is_some_and(|t| t.is_cancelled()))
                .collect();
            let mut payload = serde_json::to_value(&live).map_err(|e| {
                RuntimeCapabilityError::new(RuntimeErrorKind::InvalidRequest, e.to_string(), false)
            })?;
            for (value, sample) in payload.as_array_mut().unwrap().iter_mut().zip(live.iter()) {
                if let Some(metadata) = value.get_mut("metadata").and_then(Value::as_object_mut) {
                    let disk = sample.metadata.as_ref().and_then(|m| m.disk());
                    metadata.insert(
                        "diskUsedBytes".into(),
                        serde_json::json!(disk.map(|d| d.used_bytes)),
                    );
                    metadata.insert(
                        "diskTotalBytes".into(),
                        serde_json::json!(disk.map(|d| d.total_bytes)),
                    );
                    metadata.insert(
                        "diskUsage".into(),
                        serde_json::json!(disk.map(|d| d.usage_percent)),
                    );
                }
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            lock_scopes(&mut tx, &payload).await?;
            prune_closed(&mut payload, live.iter().map(|s| &s.scope));
            sqlx::query(include_str!("platform_metadata_batch.sql"))
                .bind(&payload)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            let accepted: i64 = sqlx::query_scalar(include_str!("platform_batch.sql"))
                .bind(payload)
                .fetch_one(&mut *tx)
                .await
                .map_err(storage)?;
            if accepted > 0 {
                sqlx::query("SELECT pg_notify($1, '')")
                    .bind(citadel_runtime::RuntimeSignal::PlatformStats.channel())
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            RuntimeWork::StatsCommit.units(1);
            Ok(StatsWriteOutcome {
                persisted: accepted as usize,
                stale: samples.len().saturating_sub(accepted as usize),
            })
        })
    }
}
