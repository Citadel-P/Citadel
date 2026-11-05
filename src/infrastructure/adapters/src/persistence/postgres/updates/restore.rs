//! Explicit checkpoint recovery into a fresh database, never a live rollback.
use super::{checkpoint::CHECKPOINT_PROBE_SQL, credentials::verify_credentials, *};
use citadel_database::MigrationRunner;

pub(crate) async fn require_empty_restore_database(pool: &PgPool) -> Result<(), UpdateError> {
    let empty: bool = sqlx::query_scalar(
        "SELECT NOT EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname NOT IN ('pg_catalog','information_schema') AND n.nspname NOT LIKE 'pg_toast%' AND c.relkind IN ('r','p','v','m','S','f'))",
    ).fetch_one(pool).await.map_err(storage)?;
    if !empty {
        return Err(UpdateError::Conflict(
            "Checkpoint recovery requires a new, empty database. Preserve the original database and update state.".into(),
        ));
    }
    Ok(())
}

/// Caller holds Core's database lease through restore and this transaction.
/// This is a new recovery outcome, not a forged successful helper journal.
pub(crate) async fn finish_checkpoint_restore(
    pool: &PgPool,
    checkpoint: &UpdateOperation,
    encryption_key: &[u8],
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<UpdateOperation, UpdateError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::raw_sql("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE; SET LOCAL statement_timeout='30s'; SET LOCAL lock_timeout='2s';")
        .execute(&mut *tx).await.map_err(storage)?;
    let row =
        sqlx::query("SELECT * FROM instanceupdates WHERE id=$1 AND ownsinstallation FOR UPDATE")
            .bind(checkpoint.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(UpdateError::NotFound)?;
    let mut restored = super::queries::read_operation(&row)?;
    if checkpoint.phase != UpdatePhase::BackingUp
        || checkpoint.revision != 2
        || restored.phase != UpdatePhase::Draining
        || restored.revision != 1
        || restored.id != checkpoint.id
        || restored.fence != checkpoint.fence
        || restored.plan != checkpoint.plan
        || restored.instance_id != checkpoint.instance_id
        || restored.actor_id != checkpoint.actor_id
        || restored.idempotency_key != checkpoint.idempotency_key
        || restored.created_at != checkpoint.created_at
        || restored.updated_at > checkpoint.updated_at
        || restored.checkpoint.is_some()
        || checkpoint.checkpoint.is_some()
        || row
            .try_get::<Option<serde_json::Value>, _>("completion")
            .map_err(storage)?
            .is_some()
    {
        return Err(UpdateError::Conflict(
            "Restored ownership does not match the source checkpoint.".into(),
        ));
    }
    // The checkpoint dump contains the source's last persisted Draining state.
    let probe: sqlx::types::Json<super::checkpoint::Probe> =
        sqlx::query_scalar(CHECKPOINT_PROBE_SQL)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
    let entries = super::checkpoint::checked_restore_migrations(probe.0, &restored)?;
    if MigrationRunner::validate_applied_catalog(&entries).map_err(storage)?
        != checkpoint.plan.source.catalog_sha256
    {
        return Err(UpdateError::Conflict(
            "Restore requires the exact source migration catalog.".into(),
        ));
    }
    verify_credentials(&mut tx, encryption_key).await?;
    restored.advance(restored.revision, UpdatePhase::Failed, Utc::now())?;
    restored.message = Some("Source checkpoint restored into a new database by an operator. The update was not installed.".into());
    operation_activity(&mut tx, &restored).await?;
    sqlx::query(
        "UPDATE instanceupdates SET operation=$2,revision=$3,ownsinstallation=FALSE WHERE id=$1",
    )
    .bind(restored.id)
    .bind(sqlx::types::Json(&restored))
    .bind(restored.revision as i64)
    .execute(&mut *tx)
    .await
    .map_err(storage)?;
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    tx.commit().await.map_err(storage)?;
    Ok(restored)
}
