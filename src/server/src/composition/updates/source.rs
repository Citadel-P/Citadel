//! Operator recovery composition for an unchanged source installation.
use std::{path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(crate) async fn recover_source(
    state_directory: PathBuf,
    operation_id: Uuid,
    cancellation: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error>> {
    use citadel_adapters::{
        external::updates::DockerUpdateInstallation,
        persistence::postgres::{connection, updates::PostgresUpdateRepository},
    };
    use citadel_updates::{UpdateInstallationRuntime, UpdateRepository};
    let database = citadel_server::config::DatabaseConfig::from_env()?;
    let pool =
        connection::connect_pool(&database.database_url, 1, std::time::Duration::from_secs(5))
            .await?;
    let repository = PostgresUpdateRepository::new(pool.clone());
    let operation = repository
        .active()
        .await?
        .ok_or("No update retains installation ownership.")?;
    if operation.id != operation_id
        || operation.plan.source.catalog_sha256
            != citadel_database::MigrationRunner::catalog_sha256()?
    {
        return Err("Source build or operation does not match the retained update.".into());
    }
    let runtime = DockerUpdateInstallation::new(
        Arc::new(citadel_processes::SystemProcess),
        state_directory,
        env!("CITADEL_BUILD_VERSION").into(),
        pool.clone(),
    );
    runtime.recover_source(&operation, cancellation).await?;
    repository.abort_source(&operation).await?;
    pool.close().await;
    Ok(())
}
