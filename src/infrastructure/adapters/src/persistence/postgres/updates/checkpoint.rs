//! Read-only validation of a checkpoint restored into an isolated PostgreSQL instance.
use citadel_database::{AppliedMigration, MigrationRunner};
use citadel_updates::{UpdateError, UpdateJournal, UpdateOperation};
use serde::Deserialize;
use uuid::Uuid;

pub(crate) const CHECKPOINT_PROBE_SQL: &str = include_str!("checkpoint.sql");

pub(crate) async fn check_checkpoint_source(
    pool: &sqlx::PgPool,
    journal: &UpdateJournal,
) -> Result<(), UpdateError> {
    let value: serde_json::Value = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sqlx::query_scalar(CHECKPOINT_PROBE_SQL).fetch_one(pool),
    )
    .await
    .map_err(|_| failed())?
    .map_err(|_| failed())?;
    validate_checkpoint_database(&serde_json::to_vec(&value).map_err(|_| failed())?, journal)
}

#[derive(Deserialize)]
struct Probe {
    instance: Uuid,
    initialized: bool,
    migrations: Vec<Migration>,
    owners: Vec<Owner>,
}
#[derive(Deserialize)]
struct Migration {
    id: String,
    checksum: String,
    completed: bool,
    failed: bool,
}
#[derive(Deserialize)]
struct Owner {
    id: Uuid,
    instance: Uuid,
    actor: Uuid,
    key: Uuid,
    revision: i64,
    operation: UpdateOperation,
}

pub(crate) fn validate_checkpoint_database(
    bytes: &[u8],
    journal: &UpdateJournal,
) -> Result<(), UpdateError> {
    let entries = checked_migrations(bytes, journal)?;
    let catalog = MigrationRunner::validate_applied_catalog(&entries).map_err(|_| failed())?;
    if catalog != journal.operation.plan.source.catalog_sha256 {
        return Err(failed());
    }
    Ok(())
}

pub(super) fn checked_migrations(
    bytes: &[u8],
    journal: &UpdateJournal,
) -> Result<Vec<AppliedMigration>, UpdateError> {
    let probe: Probe = serde_json::from_slice(bytes).map_err(|_| failed())?;
    if probe.instance != journal.operation.instance_id
        || !probe.initialized
        || probe.migrations.len() > 1024
        || probe.owners.len() != 1
    {
        return Err(failed());
    }
    let owner = &probe.owners[0];
    if owner.id != journal.operation.id
        || owner.instance != probe.instance
        || owner.actor != journal.operation.actor_id.value()
        || owner.key != journal.operation.idempotency_key
        || u64::try_from(owner.revision).ok() != Some(owner.operation.revision)
        || !journal.history.contains(&owner.operation)
    {
        return Err(failed());
    }
    Ok(probe
        .migrations
        .into_iter()
        .map(|m| AppliedMigration {
            id: m.id,
            checksum: m.checksum,
            completed: m.completed,
            failed: m.failed,
        })
        .collect())
}

fn failed() -> UpdateError {
    UpdateError::Unavailable("Restored checkpoint identity, ownership or migration history does not match the approved source.".into())
}
