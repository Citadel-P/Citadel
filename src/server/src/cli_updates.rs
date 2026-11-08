//! Detached update executable ownership; never constructs normal Core services.
use citadel_server::config::environment;
use std::{io::Read, path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub async fn restore_checkpoint(
    checkpoint_directory: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use citadel_adapters::external::{
        backups::system_recovery::CitadelSystemRestoreOptions,
        updates::{JournalDirectory, PreparedUpdateCheckpoint, restore_update_checkpoint},
    };
    let database = citadel_server::config::DatabaseConfig::from_env()?;
    let key = citadel_server::config::recovery_encryption_key_from_env()?;
    let receipt: PreparedUpdateCheckpoint =
        JournalDirectory::open(checkpoint_directory.clone())?.read("checkpoint.json")?;
    let options = CitadelSystemRestoreOptions {
        bundle: receipt.bundle,
        database_url: database.database_url,
        pg_restore: std::env::var_os(environment::CITADEL_PG_RESTORE_PATH)
            .unwrap_or_else(|| "pg_restore".into()),
        maximum_output: 1024 * 1024,
        secret_encryption_key: Some(zeroize::Zeroizing::new(key.expose().to_vec())),
    };
    let token = CancellationToken::new();
    let recovery = restore_update_checkpoint(
        &checkpoint_directory,
        &options,
        env!("CITADEL_BUILD_VERSION"),
        &token,
    );
    tokio::pin!(recovery);
    let operation = tokio::select! {
        result = &mut recovery => result?,
        () = crate::app::shutdown_signal(token.clone()) => recovery.await?,
    };
    println!(
        "Source checkpoint restored; update {} recorded as failed. Preserve the original installation and start the recovered source with its original keys and data volume and a new update-state directory.",
        operation.id
    );
    Ok(())
}

pub async fn host() -> Result<(), Box<dyn std::error::Error>> {
    use citadel_adapters::external::updates::{
        HostUpdateRequest, MAX_HANDOFF_REQUEST, execute_host_update,
    };
    let mut input = Vec::new();
    std::io::stdin()
        .take(MAX_HANDOFF_REQUEST as u64 + 1)
        .read_to_end(&mut input)?;
    if input.len() > MAX_HANDOFF_REQUEST {
        return Err("Invalid host update request.".into());
    }
    let request: HostUpdateRequest =
        serde_json::from_slice(&input).map_err(|_| "Invalid host update request.")?;
    let token = CancellationToken::new();
    let run = execute_host_update(request, &citadel_processes::SystemProcess, &token);
    tokio::pin!(run);
    let response = tokio::select! {
        result = &mut run => result?,
        () = crate::app::shutdown_signal(token.clone()) => run.await?,
    };
    println!("{}", serde_json::to_string(&response)?);
    Ok(())
}

pub async fn helper(
    state_directory: PathBuf,
    operation_id: Uuid,
    fence: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    use citadel_adapters::external::updates::{
        DockerReleaseInspection, UpdateHelperBudget, run_update_helper,
    };
    if !citadel_server::config::release_checks_allowed_from_env()? {
        return Err("Release checks are disabled by server configuration.".into());
    }
    let process = Arc::new(citadel_processes::SystemProcess);
    let inspector = citadel_updates::ReleaseInspectionService::new(
        crate::composition::updates::releases::artifacts(process.clone())?,
        Arc::new(DockerReleaseInspection::new(process.clone())),
    );
    let token = CancellationToken::new();
    let budget = UpdateHelperBudget::default();
    let run = run_update_helper(
        state_directory,
        operation_id,
        fence,
        &inspector,
        process.as_ref(),
        &budget,
        &token,
    );
    tokio::pin!(run);
    tokio::select! {
        result = &mut run => result?,
        () = crate::app::shutdown_signal(token.clone()) => run.await?,
    }
    Ok(())
}

pub async fn exchange() -> Result<(), Box<dyn std::error::Error>> {
    use citadel_adapters::external::updates::{MAX_HANDOFF_REQUEST, execute_helper_handoff};
    let mut input = Vec::new();
    std::io::stdin()
        .take(MAX_HANDOFF_REQUEST as u64 + 1)
        .read_to_end(&mut input)?;
    let token = CancellationToken::new();
    let exchange = execute_helper_handoff(&input, &citadel_processes::SystemProcess, &token);
    tokio::pin!(exchange);
    let output = tokio::select! {
        result = &mut exchange => result?,
        () = crate::app::shutdown_signal(token.clone()) => exchange.await?,
    };
    println!("{}", std::str::from_utf8(&output)?);
    Ok(())
}

/// Target-schema read only; does not initialize configuration keys or migrate.
pub async fn target_receipt() -> Result<(), Box<dyn std::error::Error>> {
    use citadel_adapters::external::updates::{MAX_HANDOFF_REQUEST, TargetUpdateCompletion};
    let mut input = Vec::new();
    std::io::stdin()
        .take(MAX_HANDOFF_REQUEST as u64 + 1)
        .read_to_end(&mut input)?;
    if input.len() > MAX_HANDOFF_REQUEST {
        return Err("Invalid target completion request.".into());
    }
    let completion: TargetUpdateCompletion =
        serde_json::from_slice(&input).map_err(|_| "Invalid target completion request.")?;
    crate::composition::updates::verify_completion(&completion).await?;
    println!("{}", serde_json::to_string(&completion.receipt)?);
    Ok(())
}

/// Run inside the original Core container, with its original configuration/keys.
/// This command never restores a database or retries a destructive operation.
pub async fn recover_source(
    state_directory: PathBuf,
    operation_id: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let token = CancellationToken::new();
    let recovery =
        crate::composition::updates::recover_source(state_directory, operation_id, &token);
    tokio::pin!(recovery);
    tokio::select! {
        result = &mut recovery => result?,
        () = crate::app::shutdown_signal(token.clone()) => recovery.await?,
    };
    println!(
        "Update {} safely stopped before source shutdown. Restart the original Core to resume normal admission.",
        operation_id
    );
    Ok(())
}
