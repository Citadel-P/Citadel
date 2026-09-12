//! One-time initialization, completed before jobs and listeners are started.
use crate::state::AppState;
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

pub async fn run(state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
    citadel_server::bootstrap::initialize_from_environment(&state.identity).await?;
    // Login must use persisted setup state before listeners open, not wait for
    // the first background probe after a fresh bootstrap or ordinary restart.
    state
        .readiness
        .set_setup(!state.identity.setup_status().await?.requires_setup);
    state.readiness.set(true, false);
    Ok(())
}
