//! Detached update executable ownership; never constructs normal Core services.
use std::{io::Read, path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub async fn helper(
    state_directory: PathBuf,
    operation_id: Uuid,
    fence: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    use citadel_adapters::external::updates::{
        DockerReleaseInspection, GithubReleaseArtifacts, UpdateHelperBudget, run_update_helper,
    };
    if !citadel_server::config::release_checks_allowed_from_env()? {
        return Err("Release checks are disabled by server configuration.".into());
    }
    let process = Arc::new(citadel_processes::SystemProcess);
    let inspector = citadel_updates::ReleaseInspectionService::new(
        Arc::new(GithubReleaseArtifacts::new(process.clone())?),
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
