//! Target-only startup while the single-Core lease and maintenance claim are held.
use citadel_adapters::{
    external::updates::{TargetUpdateCompletion, TargetUpdateSession},
    persistence::postgres::{
        connection,
        updates::{PostgresUpdateRepository, check_target_update_database},
    },
};
use citadel_database::MigrationRunner;
use citadel_server::config::Config;
use citadel_updates::{UpdateError, UpdatePhase, UpdateRepository};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// This is acceptance, not admission: even a committed receipt retains the
/// installation claim until host reconciliation is independently verified.
/// The caller must hold and monitor CoreJobLease for the entire future.
pub(crate) async fn accept_target(
    config: &Config,
    cancellation: &CancellationToken,
) -> Result<(), UpdateError> {
    let pool = connection::connect_pool(&config.database_url, 1, Duration::from_secs(5))
        .await
        .map_err(|_| unavailable())?;
    let result = tokio::time::timeout(
        Duration::from_secs(900),
        accept(config, &pool, cancellation),
    )
    .await
    .map_err(|_| unavailable())
    .and_then(std::convert::identity);
    tokio::time::timeout(Duration::from_secs(5), pool.close())
        .await
        .map_err(|_| unavailable())?;
    result
}

async fn accept(
    config: &Config,
    pool: &sqlx::PgPool,
    cancellation: &CancellationToken,
) -> Result<(), UpdateError> {
    let repository = PostgresUpdateRepository::new(pool.clone());
    let operation = tokio::time::timeout(Duration::from_secs(5), repository.active())
        .await
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?
        .ok_or_else(unavailable)?;
    let target = &operation.plan.target;
    // A restarted source, development executable or unrelated image must not
    // migrate under someone else's retained claim. Container identity is checked
    // independently by TargetUpdateSession before database verification.
    if !matches_target_build(target)? || operation.phase.terminal() || cancellation.is_cancelled() {
        return Err(unavailable());
    }
    let process = citadel_processes::SystemProcess;
    let session = TargetUpdateSession::connect(&operation, &process, cancellation).await?;
    let journal = &session.evidence.journal;
    if journal.operation.phase != UpdatePhase::Verifying
        || config
            .update_state_directory
            .as_ref()
            .is_some_and(|path| path != &session.evidence.intent.registration.state_directory)
    {
        return Err(unavailable());
    }
    let verify_keys = || {
        session.evidence.recovery_keys.verify_loaded(
            &config.execution.paths.data_root,
            config.identity.jwt_key.expose(),
            config.identity.secret_encryption_key.expose(),
            config.identity.jwt_key_is_external,
            config.identity.encryption_key_is_external,
        )
    };
    verify_keys()?;
    // Only the target executable checks/migrates the database after replacement.
    // The exact source catalog must be an unchanged prefix of this build's catalog.
    check_target_update_database(
        pool,
        journal,
        Some(&session.evidence.source_catalog),
        config.identity.secret_encryption_key.expose(),
    )
    .await?;
    session.revalidate(&process, cancellation).await?;
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    tokio::select! {
        biased;
        () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
        result = MigrationRunner::migrate(&config.database_url) => {
            result.map_err(|_| unavailable())?;
        }
    }
    check_target_update_database(
        pool,
        journal,
        None,
        config.identity.secret_encryption_key.expose(),
    )
    .await?;
    session.revalidate(&process, cancellation).await?;
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    verify_keys()?;
    let completed = journal.completed(target, chrono::Utc::now())?;
    // Outcome, receipt and Activities commit together before the helper sees
    // acknowledgement. An interrupted commit never authorizes acknowledgement.
    let receipt = repository
        .import(&completed)
        .await
        .map_err(|_| unavailable())?
        .ok_or_else(unavailable)?;
    session
        .acknowledge(
            &TargetUpdateCompletion {
                journal: completed,
                receipt,
            },
            &process,
            cancellation,
        )
        .await?;
    session.wait_for_exit(&process, cancellation).await
}

fn matches_target_build(target: &citadel_updates::CoreIdentity) -> Result<bool, UpdateError> {
    let architecture = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        return Ok(false);
    };
    Ok(target.version == env!("CITADEL_BUILD_VERSION")
        && target.architecture == architecture
        && target.catalog_sha256 == MigrationRunner::catalog_sha256().map_err(|_| unavailable())?)
}

fn unavailable() -> UpdateError {
    UpdateError::Unavailable(
        "Target startup could not be verified. Preserve the checkpoint and update state; maintenance remains active.".into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_only_accepts_the_exact_compiled_target_identity() {
        let identity = citadel_updates::CoreIdentity {
            version: env!("CITADEL_BUILD_VERSION").into(),
            architecture: if cfg!(target_arch = "x86_64") {
                "amd64"
            } else if cfg!(target_arch = "aarch64") {
                "arm64"
            } else {
                return;
            }
            .into(),
            catalog_sha256: MigrationRunner::catalog_sha256().unwrap(),
            image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64)),
        };
        assert!(matches_target_build(&identity).unwrap());
        for field in ["version", "architecture", "catalog"] {
            let mut other = identity.clone();
            match field {
                "version" => other.version.push_str("-other"),
                "architecture" => other.architecture = "other".into(),
                "catalog" => other.catalog_sha256 = "0".repeat(64),
                _ => unreachable!(),
            }
            assert!(!matches_target_build(&other).unwrap(), "{field}");
        }
    }
}
