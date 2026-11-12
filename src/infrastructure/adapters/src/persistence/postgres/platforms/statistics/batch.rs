//! Set-based writes across sources; scope locks fence reconfiguration/revocation.
use super::batch_input::{MetadataInput, PlatformCapture, live, scope_ids};
use citadel_platforms::stats_ingestion::{
    ContainerStatsSample, PlatformStatsSample, StatsBatchStore, StatsScope, StatsWriteOutcome,
};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use citadel_runtime::runtime_metrics::RuntimeWork;
use futures_util::future::BoxFuture;
use sqlx::types::Json;
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

async fn lock_scopes<'a>(
    tx: &mut Transaction<'_, Postgres>,
    scopes: impl Iterator<Item = &'a StatsScope>,
) -> Result<(), RuntimeCapabilityError> {
    sqlx::query("SET LOCAL lock_timeout = '500ms'")
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    let (platforms, agents) = scope_ids(scopes);
    // Stable parent order also prevents metadata lock upgrades between writers.
    sqlx::query("SELECT id FROM platforms WHERE id=ANY($1::uuid[]) ORDER BY id FOR NO KEY UPDATE")
        .bind(&platforms)
        .fetch_all(&mut **tx)
        .await
        .map_err(storage)?;
    if !agents.is_empty() {
        sqlx::query("SELECT agentid FROM edgeagentbindings WHERE agentid=ANY($1::uuid[]) ORDER BY agentid FOR SHARE")
            .bind(&agents).fetch_all(&mut **tx).await.map_err(storage)?;
    }
    Ok(())
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
            let mut accepted_samples: Vec<_> = samples.iter().filter(|s| live(&s.scope)).collect();
            if accepted_samples.is_empty() {
                return Ok(StatsWriteOutcome {
                    persisted: 0,
                    stale: samples.len(),
                });
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            lock_scopes(&mut tx, accepted_samples.iter().map(|s| &s.scope)).await?;
            // A session may close while waiting for its scope lock.
            accepted_samples.retain(|s| live(&s.scope));
            if accepted_samples.is_empty() {
                return Ok(StatsWriteOutcome {
                    persisted: 0,
                    stale: samples.len(),
                });
            }
            let accepted: i64 = sqlx::query_scalar(concat!(
                include_str!("container_batch.sql"),
                include_str!("capture.sql")
            ))
            .bind(Json(&accepted_samples))
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
            let mut accepted_samples: Vec<_> = samples.iter().filter(|s| live(&s.scope)).collect();
            if accepted_samples.is_empty() {
                return Ok(StatsWriteOutcome {
                    persisted: 0,
                    stale: samples.len(),
                });
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            lock_scopes(&mut tx, accepted_samples.iter().map(|s| &s.scope)).await?;
            accepted_samples.retain(|s| live(&s.scope));
            if accepted_samples.is_empty() {
                return Ok(StatsWriteOutcome {
                    persisted: 0,
                    stale: samples.len(),
                });
            }
            let metadata: Vec<_> = accepted_samples
                .iter()
                .filter_map(|s| MetadataInput::from_sample(s))
                .collect();
            if !metadata.is_empty() {
                sqlx::query(include_str!("platform_metadata_batch.sql"))
                    .bind(Json(&metadata))
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            let captures: Vec<_> = accepted_samples
                .iter()
                .map(|s| PlatformCapture::from(*s))
                .collect();
            let accepted: i64 = sqlx::query_scalar(include_str!("platform_batch.sql"))
                .bind(Json(&captures))
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
