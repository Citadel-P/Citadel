//! Target-schema verification while normal Core work is still fenced.
use super::{
    checkpoint::{CHECKPOINT_PROBE_SQL, Probe, checked_migrations},
    credentials::verify_credentials,
};
use citadel_database::MigrationRunner;
use citadel_updates::{UpdateError, UpdateJournal};
use sqlx::PgPool;

/// Runs in the target image so the old source helper never queries migrated SQL.
pub async fn check_committed_update_receipt(
    pool: &PgPool,
    journal: &UpdateJournal,
    receipt: &citadel_updates::CompletionReceipt,
) -> Result<(), UpdateError> {
    use sqlx::Row;
    journal.validate_for(journal.history.first().ok_or_else(failed)?)?;
    if journal.operation.phase != citadel_updates::UpdatePhase::Succeeded
        || journal.completion_acknowledged
        || journal.maintenance_released
        || citadel_database::MigrationRunner::catalog_sha256().map_err(|_| failed())?
            != receipt.target.catalog_sha256
    {
        return Err(failed());
    }
    receipt.validate_for(journal)?;
    let mut tx = pool.begin().await.map_err(|_| failed())?;
    sqlx::raw_sql("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY; SET LOCAL statement_timeout = '5s'; SET LOCAL lock_timeout = '2s';").execute(&mut *tx).await.map_err(|_| failed())?;
    let row = sqlx::query("SELECT * FROM instanceupdates WHERE id=$1 AND ownsinstallation AND instanceid=(SELECT instanceid FROM citadelinstanceidentity WHERE id=1)")
        .bind(journal.operation.id).fetch_optional(&mut *tx).await.map_err(|_| failed())?.ok_or_else(failed)?;
    let committed = super::queries::read_operation(&row).map_err(|_| failed())?;
    let actual: Option<sqlx::types::Json<citadel_updates::CompletionReceipt>> =
        row.try_get("completion").map_err(|_| failed())?;
    if committed != journal.operation || actual.as_ref().map(|v| &v.0) != Some(receipt) {
        return Err(failed());
    }
    tx.rollback().await.map_err(|_| failed())
}

pub async fn check_target_update_database(
    pool: &PgPool,
    journal: &UpdateJournal,
    source_catalog: Option<&str>,
    encryption_key: &[u8],
) -> Result<(), UpdateError> {
    journal.validate_for(journal.history.first().ok_or_else(failed)?)?;
    if journal.operation.phase != citadel_updates::UpdatePhase::Verifying
        && !(journal.operation.phase == citadel_updates::UpdatePhase::Succeeded
            && source_catalog.is_none())
    {
        return Err(failed());
    }
    let mut tx = pool.begin().await.map_err(|_| failed())?;
    sqlx::raw_sql("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY; SET LOCAL statement_timeout = '5s'; SET LOCAL lock_timeout = '2s';").execute(&mut *tx).await.map_err(|_| failed())?;
    let probe: sqlx::types::Json<Probe> = sqlx::query_scalar(CHECKPOINT_PROBE_SQL)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| failed())?;
    let entries = checked_migrations(probe.0, journal)?;
    match source_catalog {
        Some(manifest) => MigrationRunner::validate_upgrade_source(
            &entries,
            manifest,
            &journal.operation.plan.source.catalog_sha256,
        )
        .map_err(|_| failed())?,
        None => {
            if MigrationRunner::validate_applied_catalog(&entries).map_err(|_| failed())?
                != journal.operation.plan.target.catalog_sha256
            {
                return Err(failed());
            }
            verify_credentials(&mut tx, encryption_key).await?;
        }
    }
    tx.rollback().await.map_err(|_| failed())
}

fn failed() -> UpdateError {
    UpdateError::Unavailable("Target database identity, migration history, ownership or encrypted credentials could not be verified. Maintenance remains active.".into())
}
