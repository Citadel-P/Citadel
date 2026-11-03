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
