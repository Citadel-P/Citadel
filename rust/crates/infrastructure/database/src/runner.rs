use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgConnection;
use sqlx::{Connection, Row};

const MIGRATION_LOCK_KEY: i64 = 0x4369_7461_6465_6c31;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(15);
const ADVISORY_LOCK_TIMEOUT: Duration = Duration::from_secs(30);
const ADVISORY_LOCK_RETRY: Duration = Duration::from_millis(100);

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("invalid embedded migration manifest: {0}")]
    InvalidManifest(String),
    #[error(
        "migration '{id}' checksum mismatch: manifest has {expected}, embedded SQL has {actual}"
    )]
    EmbeddedChecksum {
        id: String,
        expected: String,
        actual: String,
    },
    #[error("database contains unknown migration journal entry '{0}'")]
    UnknownMigration(String),
    #[error(
        "applied migration '{id}' checksum mismatch: database has {database}, binary has {embedded}"
    )]
    AppliedChecksum {
        id: String,
        database: String,
        embedded: String,
    },
    #[error("migration '{id}' failed: {message}")]
    Apply { id: String, message: String },
    #[error("database migration {operation} timed out after {timeout:?}")]
    Timeout {
        operation: &'static str,
        timeout: Duration,
    },
    #[error("PostgreSQL migration operation failed: {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationOutcome {
    pub applied: usize,
    pub already_applied: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    migrations: Vec<ManifestMigration>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestMigration {
    id: String,
    name: String,
    file: String,
    sha256: String,
    transactional: bool,
}

#[derive(Clone, Copy)]
struct EmbeddedMigration {
    id: &'static str,
    name: &'static str,
    file: &'static str,
    sql: &'static str,
    transactional: bool,
}

struct JournalEntry {
    checksum: String,
    completed: bool,
}

include!("../generated/catalog.rs");

pub struct MigrationRunner;

impl MigrationRunner {
    pub async fn migrate(database_url: &str) -> Result<MigrationOutcome, MigrationError> {
        let manifest = validate_manifest()?;
        let mut connection =
            tokio::time::timeout(CONNECTION_TIMEOUT, PgConnection::connect(database_url))
                .await
                .map_err(|_| MigrationError::Timeout {
                    operation: "connection",
                    timeout: CONNECTION_TIMEOUT,
                })??;
        acquire_advisory_lock(&mut connection).await?;
        sqlx::raw_sql("SET lock_timeout = '30s'; SET statement_timeout = '15min';")
            .execute(&mut connection)
            .await?;

        let result = migrate_locked(&mut connection, &manifest).await;
        let unlock = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(MIGRATION_LOCK_KEY)
            .execute(&mut connection)
            .await;

        match (result, unlock) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(MigrationError::Database(error)),
            (Ok(outcome), Ok(_)) => Ok(outcome),
        }
    }
}

async fn acquire_advisory_lock(connection: &mut PgConnection) -> Result<(), MigrationError> {
    let deadline = tokio::time::Instant::now() + ADVISORY_LOCK_TIMEOUT;
    loop {
        let acquired = sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
            .bind(MIGRATION_LOCK_KEY)
            .fetch_one(&mut *connection)
            .await?;
        if acquired {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(MigrationError::Timeout {
                operation: "advisory-lock acquisition",
                timeout: ADVISORY_LOCK_TIMEOUT,
            });
        }
        tokio::time::sleep(ADVISORY_LOCK_RETRY).await;
    }
}

async fn migrate_locked(
    connection: &mut PgConnection,
    manifest: &BTreeMap<String, ManifestMigration>,
) -> Result<MigrationOutcome, MigrationError> {
    ensure_journal(connection).await?;
    let applied = load_journal(connection).await?;
    let known = MIGRATIONS
        .iter()
        .map(|migration| migration.id)
        .collect::<BTreeSet<_>>();

    for (id, entry) in &applied {
        if !known.contains(id.as_str()) {
            return Err(MigrationError::UnknownMigration(id.clone()));
        }
        let expected = &manifest[id].sha256;
        if &entry.checksum != expected {
            return Err(MigrationError::AppliedChecksum {
                id: id.clone(),
                database: entry.checksum.clone(),
                embedded: expected.clone(),
            });
        }
    }

    let mut applied_now = 0;
    for migration in MIGRATIONS {
        if applied
            .get(migration.id)
            .is_some_and(|entry| entry.completed)
        {
            continue;
        }
        apply_migration(connection, migration, &manifest[migration.id].sha256).await?;
        applied_now += 1;
    }

    Ok(MigrationOutcome {
        applied: applied_now,
        already_applied: applied.values().filter(|entry| entry.completed).count(),
    })
}

async fn ensure_journal(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        r#"
CREATE TABLE IF NOT EXISTS citadel_schema_migrations (
    id text NOT NULL PRIMARY KEY,
    name text NOT NULL,
    checksum text NOT NULL,
    started_at timestamp with time zone NOT NULL,
    completed_at timestamp with time zone,
    failure text
);
"#,
    )
    .execute(connection)
    .await?;
    Ok(())
}

async fn load_journal(
    connection: &mut PgConnection,
) -> Result<BTreeMap<String, JournalEntry>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, checksum, completed_at IS NOT NULL AS completed FROM citadel_schema_migrations ORDER BY id",
    )
    .fetch_all(connection)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok((
                row.try_get("id")?,
                JournalEntry {
                    checksum: row.try_get("checksum")?,
                    completed: row.try_get("completed")?,
                },
            ))
        })
        .collect()
}

async fn apply_migration(
    connection: &mut PgConnection,
    migration: &EmbeddedMigration,
    checksum: &str,
) -> Result<(), MigrationError> {
    sqlx::query(
        r#"
INSERT INTO citadel_schema_migrations (id, name, checksum, started_at, completed_at, failure)
VALUES ($1, $2, $3, CURRENT_TIMESTAMP, NULL, NULL)
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name,
    checksum = EXCLUDED.checksum,
    started_at = CURRENT_TIMESTAMP,
    completed_at = NULL,
    failure = NULL
WHERE citadel_schema_migrations.completed_at IS NULL
"#,
    )
    .bind(migration.id)
    .bind(migration.name)
    .bind(checksum)
    .execute(&mut *connection)
    .await?;

    apply_in_runner_transaction(connection, migration).await
}

async fn apply_in_runner_transaction(
    connection: &mut PgConnection,
    migration: &EmbeddedMigration,
) -> Result<(), MigrationError> {
    let mut transaction = connection.begin().await?;
    let apply_result = sqlx::raw_sql(migration.sql)
        .execute(&mut *transaction)
        .await;
    if let Err(error) = apply_result {
        transaction.rollback().await?;
        return record_failure(connection, migration, error).await;
    }

    sqlx::query(
        "UPDATE citadel_schema_migrations SET completed_at = CURRENT_TIMESTAMP, failure = NULL WHERE id = $1",
    )
    .bind(migration.id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn record_failure(
    connection: &mut PgConnection,
    migration: &EmbeddedMigration,
    error: sqlx::Error,
) -> Result<(), MigrationError> {
    let message = bounded_error(&error.to_string());
    sqlx::query(
        "UPDATE citadel_schema_migrations SET failure=$2 WHERE id=$1 AND completed_at IS NULL",
    )
    .bind(migration.id)
    .bind(&message)
    .execute(&mut *connection)
    .await?;
    Err(MigrationError::Apply {
        id: migration.id.to_owned(),
        message,
    })
}

fn validate_manifest() -> Result<BTreeMap<String, ManifestMigration>, MigrationError> {
    let manifest: Manifest = serde_json::from_str(super::MIGRATION_MANIFEST_JSON)
        .map_err(|error| MigrationError::InvalidManifest(error.to_string()))?;
    if manifest.schema_version != u32::try_from(MIGRATIONS.len()).unwrap_or(u32::MAX) {
        return Err(MigrationError::InvalidManifest(format!(
            "unsupported schema version {}",
            manifest.schema_version
        )));
    }
    if manifest.migrations.len() != MIGRATIONS.len() {
        return Err(MigrationError::InvalidManifest(
            "embedded migration count differs from the manifest".to_owned(),
        ));
    }

    let mut by_id = BTreeMap::new();
    for migration in manifest.migrations {
        if by_id.insert(migration.id.clone(), migration).is_some() {
            return Err(MigrationError::InvalidManifest(
                "duplicate migration id".to_owned(),
            ));
        }
    }

    for embedded in MIGRATIONS {
        let entry = by_id.get(embedded.id).ok_or_else(|| {
            MigrationError::InvalidManifest(format!(
                "embedded migration '{}' is absent from the manifest",
                embedded.id
            ))
        })?;
        if entry.name != embedded.name
            || entry.file != embedded.file
            || entry.transactional != embedded.transactional
        {
            return Err(MigrationError::InvalidManifest(format!(
                "metadata differs for migration '{}'",
                embedded.id
            )));
        }
        let actual = sha256(embedded.sql.as_bytes());
        if actual != entry.sha256 {
            return Err(MigrationError::EmbeddedChecksum {
                id: embedded.id.to_owned(),
                expected: entry.sha256.clone(),
                actual,
            });
        }
    }
    Ok(by_id)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn bounded_error(message: &str) -> String {
    const MAX_ERROR_CHARS: usize = 4_096;
    message.chars().take(MAX_ERROR_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_manifest_and_baseline_are_consistent() {
        let manifest = validate_manifest().expect("embedded catalog should be valid");
        assert_eq!(
            manifest.len(),
            1,
            "the unreleased database has one baseline"
        );
        assert_eq!(manifest.len(), MIGRATIONS.len());
        assert_eq!(MIGRATIONS[0].id, "0001");
        assert!(!MIGRATIONS[0].sql.contains("__EFMigrationsHistory"));
        assert!(!MIGRATIONS[0].sql.contains("START TRANSACTION"));
        let tables = |sql: &str| {
            sql.lines()
                .filter_map(|line| line.strip_prefix("CREATE TABLE "))
                .filter_map(|line| line.split_whitespace().next())
                .map(str::to_owned)
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(tables(MIGRATIONS[0].sql), tables(super::super::SCHEMA_SQL));
        assert_eq!(
            MIGRATIONS[0].sql.matches("INSERT INTO ").count(),
            super::super::SCHEMA_SQL.matches("INSERT INTO ").count()
        );
    }

    #[test]
    fn stored_failure_evidence_is_bounded() {
        let message = "x".repeat(5_000);
        assert_eq!(bounded_error(&message).len(), 4_096);
    }
}
