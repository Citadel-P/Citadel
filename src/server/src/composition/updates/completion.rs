/// Read-only target CLI composition, separate from normal Core initialization.
#[cfg(target_os = "linux")]
pub(crate) async fn verify_completion(
    completion: &citadel_adapters::external::updates::TargetUpdateCompletion,
) -> Result<(), Box<dyn std::error::Error>> {
    use citadel_adapters::persistence::postgres::{
        connection, updates::check_committed_update_receipt,
    };
    use std::time::Duration;
    if completion.receipt.target.version != env!("CITADEL_BUILD_VERSION")
        || completion.receipt.target.catalog_sha256
            != citadel_database::MigrationRunner::catalog_sha256()?
    {
        return Err("Completion does not belong to this target build.".into());
    }
    let database = citadel_server::config::DatabaseConfig::from_env()
        .map_err(|_| "Cannot read target database configuration.")?;
    let pool = connection::connect_pool(&database.database_url, 1, Duration::from_secs(5))
        .await
        .map_err(|_| "Cannot connect to target database.")?;
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        check_committed_update_receipt(&pool, &completion.journal, &completion.receipt),
    )
    .await;
    tokio::time::timeout(Duration::from_secs(5), pool.close())
        .await
        .map_err(|_| "Cannot close target verification database connection.")?;
    result.map_err(|_| "Target receipt verification timed out.")??;
    Ok(())
}

pub(super) async fn resume_completed(
    config: &citadel_server::config::Config,
    pool: &sqlx::PgPool,
    operation: &citadel_updates::UpdateOperation,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<(), citadel_updates::UpdateError> {
    use citadel_adapters::{
        external::updates::{HostUpdateClient, HostUpdateRequest, HostUpdateResponse},
        persistence::postgres::updates::{
            PostgresUpdateRepository, check_committed_update_receipt, check_target_update_database,
        },
    };
    use citadel_updates::{UpdateError, UpdateRepository};
    let failed = || {
        UpdateError::Unavailable(
            "Completed update recovery could not be verified. Maintenance remains active.".into(),
        )
    };
    let directory = config.update_state_directory.as_ref().ok_or_else(failed)?;
    if directory != std::path::Path::new(&operation.plan.checkpoint_directory) {
        return Err(failed());
    }
    let client = HostUpdateClient::new(
        std::sync::Arc::new(citadel_processes::SystemProcess),
        directory.clone(),
    );
    let (response, image, architecture) = client
        .exchange(
            HostUpdateRequest::Completed(Box::new(operation.clone())),
            cancellation,
        )
        .await?;
    let HostUpdateResponse::Completed(completed) = response else {
        return Err(failed());
    };
    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname").map_err(|_| failed())?;
    if completed.journal.operation != *operation
        || image != operation.plan.target.image
        || architecture != operation.plan.target.architecture
        || !citadel_updates::is_sha256(&completed.container_id)
        || hostname.trim() != &completed.container_id[..12]
    {
        return Err(failed());
    }
    completed.recovery_keys.verify_loaded(
        &config.execution.paths.data_root,
        config.identity.jwt_key.expose(),
        config.identity.secret_encryption_key.expose(),
        config.identity.jwt_key_is_external,
        config.identity.encryption_key_is_external,
    )?;
    check_committed_update_receipt(
        pool,
        &completed.completion.journal,
        &completed.completion.receipt,
    )
    .await?;
    check_target_update_database(
        pool,
        &completed.journal,
        None,
        config.identity.secret_encryption_key.expose(),
    )
    .await?;
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    PostgresUpdateRepository::new(pool.clone())
        .release(&completed.journal, &operation.plan.target)
        .await
}
