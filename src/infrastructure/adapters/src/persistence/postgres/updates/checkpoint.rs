//! Read-only source identity, update ownership and migration-catalog validation.
use citadel_database::{AppliedMigration, MigrationRunner};
use citadel_updates::{UpdateError, UpdateJournal, UpdateOperation};
use serde::Deserialize;
use uuid::Uuid;

pub(super) const CHECKPOINT_PROBE_SQL: &str = include_str!("checkpoint.sql");

pub(crate) async fn check_checkpoint_source(
    pool: &sqlx::PgPool,
    journal: &UpdateJournal,
) -> Result<(), UpdateError> {
    let probe: sqlx::types::Json<Probe> = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sqlx::query_scalar(CHECKPOINT_PROBE_SQL).fetch_one(pool),
    )
    .await
    .map_err(|_| failed())?
    .map_err(|_| failed())?;
    let entries = checked_migrations(probe.0, journal)?;
    let catalog = MigrationRunner::validate_applied_catalog(&entries).map_err(|_| failed())?;
    if catalog != journal.operation.plan.source.catalog_sha256 {
        return Err(failed());
    }
    Ok(())
}

#[derive(Deserialize)]
pub(super) struct Probe {
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

pub(super) fn checked_migrations(
    probe: Probe,
    journal: &UpdateJournal,
) -> Result<Vec<AppliedMigration>, UpdateError> {
    checked_owner_migrations(probe, &journal.operation, &journal.history)
}

pub(super) fn checked_restore_migrations(
    probe: Probe,
    restored: &UpdateOperation,
) -> Result<Vec<AppliedMigration>, UpdateError> {
    checked_owner_migrations(probe, restored, std::slice::from_ref(restored))
}

fn checked_owner_migrations(
    probe: Probe,
    operation: &UpdateOperation,
    history: &[UpdateOperation],
) -> Result<Vec<AppliedMigration>, UpdateError> {
    if probe.instance != operation.instance_id
        || !probe.initialized
        || probe.migrations.len() > 1024
        || probe.owners.len() != 1
    {
        return Err(failed());
    }
    let owner = &probe.owners[0];
    if owner.id != operation.id
        || owner.instance != probe.instance
        || owner.actor != operation.actor_id.value()
        || owner.key != operation.idempotency_key
        || u64::try_from(owner.revision).ok() != Some(owner.operation.revision)
        || !history.contains(&owner.operation)
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
    UpdateError::Unavailable("Source database identity, ownership or migration history does not match the approved source.".into())
}
