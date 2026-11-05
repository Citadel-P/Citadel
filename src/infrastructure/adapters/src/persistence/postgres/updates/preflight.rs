use citadel_database::{AppliedMigration, MigrationRunner};
use citadel_updates::{UpdateDatabaseState, UpdateError};
use sqlx::{PgPool, Row};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// One bounded, read-only snapshot. It never creates migration history or repairs it.
pub async fn read_update_database_state(
    pool: &PgPool,
    cancellation: &CancellationToken,
) -> Result<UpdateDatabaseState, UpdateError> {
    let read = async {
        let mut tx = pool.begin().await.map_err(unavailable)?;
        sqlx::raw_sql("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY; SET LOCAL statement_timeout = '5s'; SET LOCAL lock_timeout = '2s';")
            .execute(&mut *tx).await.map_err(unavailable)?;
        let instance: Option<Uuid> = sqlx::query_scalar("SELECT instanceid FROM public.citadelinstanceidentity WHERE id=1 AND EXISTS(SELECT 1 FROM public.instancesetupstates WHERE id=1 AND initializedat IS NOT NULL)")
            .fetch_optional(&mut *tx).await.map_err(unavailable)?;
        let instance_id = instance
            .filter(|id| !id.is_nil())
            .ok_or_else(|| unavailable("identity"))?;
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM public.instanceupdates WHERE ownsinstallation)",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if owned {
            return Err(UpdateError::Conflict(
                "An update retains installation ownership; recovery verification is required."
                    .into(),
            ));
        }
        let rows = sqlx::query("SELECT id, checksum, completed_at IS NOT NULL AS completed, failure IS NOT NULL AS failed FROM public.citadel_schema_migrations ORDER BY id LIMIT 1025")
            .fetch_all(&mut *tx).await.map_err(unavailable)?;
        if rows.len() > 1024 {
            return Err(unavailable("journal limit"));
        }
        let entries = rows
            .into_iter()
            .map(|row| {
                Ok(AppliedMigration {
                    id: row.try_get("id")?,
                    checksum: row.try_get("checksum")?,
                    completed: row.try_get("completed")?,
                    failed: row.try_get("failed")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(unavailable)?;
        let catalog_sha256 = MigrationRunner::validate_applied_catalog(&entries)
            .map_err(|_| UpdateError::Unavailable("The live migration journal does not exactly match this Core build. Preserve its history and resolve missing, failed, unknown or altered migrations before updating.".into()))?;
        let bytes: i64 = sqlx::query_scalar("SELECT pg_database_size(current_database())")
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
        let database_bytes = u64::try_from(bytes)
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| unavailable("size"))?;
        tx.rollback().await.map_err(unavailable)?;
        Ok(UpdateDatabaseState {
            instance_id,
            catalog_sha256,
            database_bytes,
        })
    };
    tokio::select! {
        biased;
        () = cancellation.cancelled() => Err(UpdateError::Cancelled),
        result = tokio::time::timeout(Duration::from_secs(10), read) => result.map_err(|_| unavailable("timeout"))?,
    }
}

fn unavailable(_: impl std::fmt::Display) -> UpdateError {
    UpdateError::Unavailable("Cannot read initialized instance identity, update ownership and migration history from the existing database.".into())
}
