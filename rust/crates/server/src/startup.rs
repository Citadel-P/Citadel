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
    citadel_adapters::maintenance_store::recover_on_startup(&state.pool).await?;
    state.readiness.set(true, false);
    Ok(())
}

/// Immediate abandoned-run recovery assumes one Core dispatching against a DB.
/// Keep an advisory lease for the entire lifecycle so another Core cannot
/// classify live executions as abandoned during its startup.
pub async fn acquire_job_lease(config: &Config) -> Result<sqlx::PgConnection, sqlx::Error> {
    use sqlx::Connection;
    let mut connection = sqlx::PgConnection::connect(&config.database_url).await?;
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(4848495441444547)")
        .fetch_one(&mut connection)
        .await?;
    if !acquired {
        return Err(sqlx::Error::Protocol("Another Citadel Core owns the background jobs for this database. Stop that instance before starting another.".into()));
    }
    Ok(connection)
}

pub async fn watch_job_lease(
    lease: std::sync::Arc<tokio::sync::Mutex<sqlx::PgConnection>>,
    token: tokio_util::sync::CancellationToken,
) -> Result<(), sqlx::Error> {
    loop {
        tokio::select! {
            ()=token.cancelled()=>return Ok(()),
            _=tokio::time::sleep(std::time::Duration::from_secs(5))=>{}
        }
        let mut connection = lease.lock().await;
        tokio::select! {
            ()=token.cancelled()=>return Ok(()),
            result=tokio::time::timeout(std::time::Duration::from_secs(2),sqlx::query("SELECT 1").execute(&mut *connection))=>{
                result.map_err(|_|sqlx::Error::Protocol("Core job lease connection timed out".into()))??;
            }
        }
    }
}
