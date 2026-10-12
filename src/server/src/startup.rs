//! One-time initialization, completed before jobs and listeners are started.
use crate::composition::ServerComponents;
use citadel_database::MigrationRunner;
use citadel_server::config::Config;

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

pub async fn run(state: &ServerComponents) -> Result<(), Box<dyn std::error::Error>> {
    citadel_server::bootstrap::initialize_from_environment(&state.identity).await?;
    // Login and /ready must see persisted setup state before listeners open.
    state
        .readiness
        .set_setup(!state.identity.setup_status().await?.requires_setup);
    citadel_adapters::persistence::postgres::maintenance::recover_on_startup(&state.pool).await?;
    state.readiness.set(true, false);
    Ok(())
}

pub use citadel_adapters::persistence::postgres::job_lease::CoreJobLease;

pub async fn acquire_job_lease(config: &Config) -> Result<CoreJobLease, sqlx::Error> {
    CoreJobLease::acquire(&config.database_url).await
}

pub async fn watch_job_lease(
    lease: CoreJobLease,
    token: tokio_util::sync::CancellationToken,
) -> Result<CoreJobLease, sqlx::Error> {
    lease.watch(token).await
}
