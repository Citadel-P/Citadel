//! One-time initialization, completed before jobs and listeners are started.
use crate::composition::ServerComponents;
use citadel_database::MigrationRunner;
use citadel_server::config::Config;

/// This check runs before migrations, setup, recovery and worker construction.
/// This check never releases ownership. A retained claim can only enter the
/// separate target-acceptance path; ordinary migration/startup remains blocked.
pub async fn check_updates(config: &Config) -> Result<(), citadel_updates::UpdateError> {
    use citadel_updates::UpdateError;
    let registered_instance = match config.update_state_directory.as_ref() {
        None => None,
        Some(path) => {
            #[cfg(target_os = "linux")]
            {
                Some(
                    citadel_adapters::external::updates::check_startup_registration(path)?
                        .instance_id,
                )
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = path;
                return Err(UpdateError::Unavailable(
                    "Update registration requires Linux.".into(),
                ));
            }
        }
    };
    let pool = citadel_adapters::persistence::postgres::connection::connect_pool(
        &config.database_url,
        1,
        std::time::Duration::from_secs(5),
    )
    .await
    .map_err(|_| UpdateError::Unavailable("Cannot inspect database update state.".into()))?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        citadel_adapters::persistence::postgres::updates::check_startup_updates(
            &pool,
            registered_instance,
        ),
    )
    .await;
    if tokio::time::timeout(std::time::Duration::from_secs(5), pool.close())
        .await
        .is_err()
    {
        return Err(UpdateError::Unavailable(
            "Database update-state inspection did not close within its startup budget.".into(),
        ));
    }
    match result {
        Ok(Err(UpdateError::Storage(_))) | Err(_) => Err(UpdateError::Unavailable(
            "Cannot verify database update state before startup; check database availability and schema.".into(),
        )),
        Ok(result) => result,
    }
}

/// Migrate before constructing services that depend on the database schema.
pub async fn migrate(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let migration = MigrationRunner::migrate(&config.database_url).await?;
    tracing::info!(
        applied = migration.applied,
        already_applied = migration.already_applied,
        "database migrations completed"
    );
    Ok(())
}

pub async fn run(
    state: &ServerComponents,
    lease: &mut CoreJobLease,
) -> Result<(), Box<dyn std::error::Error>> {
    while_owned(
        lease,
        citadel_server::bootstrap::initialize_from_environment(&state.identity),
    )
    .await??;
    // Login and /ready must see persisted setup state before listeners open.
    state.readiness.set_setup(
        !while_owned(lease, state.identity.setup_status())
            .await??
            .requires_setup,
    );
    while_owned(
        lease,
        citadel_adapters::persistence::postgres::maintenance::recover_on_startup(&state.pool),
    )
    .await??;
    state.readiness.set(true, false);
    Ok(())
}

pub use citadel_adapters::persistence::postgres::job_lease::CoreJobLease;

/// Startup work has no detached monitor: the caller owns both futures. Verify
/// before polling work, and monitor until it finishes. Each subsequent phase
/// verifies again rather than trusting a successful probe from an earlier phase.
pub async fn while_owned<F: std::future::Future>(
    lease: &mut CoreJobLease,
    work: F,
) -> Result<F::Output, sqlx::Error> {
    lease.verify().await?;
    tokio::select! {
        biased;
        error = lease.wait_for_loss() => Err(error),
        result = work => Ok(result),
    }
}

pub async fn acquire_job_lease(config: &Config) -> Result<CoreJobLease, sqlx::Error> {
    CoreJobLease::acquire(&config.database_url).await
}

pub async fn watch_job_lease(
    lease: CoreJobLease,
    token: tokio_util::sync::CancellationToken,
) -> Result<CoreJobLease, sqlx::Error> {
    lease.watch(token).await
}
