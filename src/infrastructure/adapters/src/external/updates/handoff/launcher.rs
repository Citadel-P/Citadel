//! Launch once from the registered executor namespace, preserving ambiguous outcomes.
use super::super::{JournalDirectory, bootstrap::verify_handoff_source, docker::LocalDocker};
use super::{
    evidence::{HelperBinding, HelperLaunchIntent, conflict, prepare_helper_launch},
    runtime::private_home,
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateOperation, is_sha256};
use std::{
    path::Path,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

/// Launch confirmation only. Core must still observe acknowledgement and drain
/// before authorizing shutdown through DockerHelperHandoff.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LaunchedUpdateHelper {
    pub intent: HelperLaunchIntent,
    pub container_id: String,
}

/// Internal driver building block, not a public install action. Run in the
/// registered host/executor namespace after the database approval commits.
/// Core cannot call this directly: its host-root bind is read-only.
/// Await cancellation cleanup of Docker clients. Never retry, remove or restart
/// a container after failure: launch intent and binding is retained for recovery.
pub async fn launch_update_helper(
    state: &JournalDirectory,
    operation: &UpdateOperation,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<LaunchedUpdateHelper, UpdateError> {
    cancelled(cancellation)?;
    let intent = prepare_helper_launch(state, operation)?;
    let owner = state.existing_lock()?;
    intent.verify_saved(state)?;
    let home = private_home()?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(120),
    };
    verify_handoff_source(&docker, &intent.registration, None).await?;
    let image = verification::source_image(&docker, &intent).await?;
    let core = verification::container(&docker, &intent.registration.core_container_id).await?;
    if core["Image"] != image {
        return Err(conflict());
    }
    let arguments = create_arguments(&intent)?;
    state.verify_lock(&owner)?;
    intent.verify_saved(state)?;
    let output = docker
        .run(&arguments.iter().map(String::as_str).collect::<Vec<_>>())
        .await?;
    let id = std::str::from_utf8(&output)
        .map(str::trim)
        .map_err(|_| conflict())?;
    if !is_sha256(id)
        || id == intent.registration.core_container_id
        || id == intent.registration.database_container_id
    {
        return Err(conflict());
    }
    let binding = HelperBinding {
        nonce: intent.nonce,
        helper_container_id: id.into(),
    };
    verification::created_helper(&docker, &intent, &binding).await?;
    state.verify_lock(&owner)?;
    intent.verify_saved(state)?;
    cancelled(cancellation)?;
    // Recheck the source immediately before allowing the helper to execute.
    verify_handoff_source(&docker, &intent.registration, Some(id)).await?;
    verification::created_helper(&docker, &intent, &binding).await?;
    state.verify_lock(&owner)?;
    intent.verify_saved(state)?;
    cancelled(cancellation)?;
    state.create(&intent.file("binding"), &binding)?;
    // The child acquires this lock on entry. Never hold it across docker start
    // or while waiting for acknowledgement; the helper would fail to acquire it.
    drop(owner);
    docker.run(&["container", "start", id]).await?;
    verification::helper(&docker, &intent, &binding).await?;
    cancelled(cancellation)?;
    Ok(LaunchedUpdateHelper {
        intent,
        container_id: id.into(),
    })
}

pub(super) fn create_arguments(intent: &HelperLaunchIntent) -> Result<Vec<String>, UpdateError> {
    let reg = &intent.registration;
    let mut args = vec![
        "container".into(),
        "create".into(),
        "--name".into(),
        intent.helper_name(),
        "--pull=never".into(),
        "--platform".into(),
        format!("linux/{}", intent.operation.plan.source.architecture),
        "--restart=no".into(),
        format!(
            "--network={}",
            if intent.operation.plan.database.is_some() {
                reg.network.clone()
            } else {
                format!("container:{}", reg.database_container_id)
            }
        ),
        "--read-only".into(),
        "--no-healthcheck".into(),
        "--user".into(),
        format!("{}:{}", reg.executor_uid, reg.executor_gid),
        "--env".into(),
        "TMPDIR=/tmp".into(),
        "--tmpfs=/tmp:rw,noexec,nosuid,size=64m".into(),
        "--label".into(),
        format!("{}={}", verification::NONCE, intent.nonce),
        "--label".into(),
        format!("{}={}", verification::OPERATION, intent.operation.id),
        "--entrypoint".into(),
        "/app/citadel-server".into(),
    ];
    for path in [&reg.directory, &reg.state_directory] {
        args.extend(["--mount".into(), bind_mount(path, path)?]);
    }
    args.extend([
        "--mount".into(),
        bind_mount(&reg.docker_socket, Path::new(super::super::docker::SOCKET))?,
    ]);
    args.extend([
        "--mount".into(),
        format!(
            "type=volume,source={},destination=/app/data,readonly",
            reg.core_volume
        ),
    ]);
    args.extend([
        intent.operation.plan.helper_image.clone(),
        "run-update-helper".into(),
        "--state-directory".into(),
        reg.state_directory.to_str().ok_or_else(conflict)?.into(),
        "--operation-id".into(),
        intent.operation.id.to_string(),
        "--fence".into(),
        intent.operation.fence.to_string(),
    ]);
    Ok(args)
}

fn bind_mount(path: &Path, target: &Path) -> Result<String, UpdateError> {
    // Docker --mount parses CSV, not shell quoting. Quote entire fields and
    // escape embedded quotes so commas in legitimate host paths remain literal.
    let path = path.to_str().ok_or_else(conflict)?.replace('"', "\"\"");
    let target = target.to_str().ok_or_else(conflict)?.replace('"', "\"\"");
    Ok(format!(
        "type=bind,\"source={path}\",\"destination={target}\""
    ))
}

fn cancelled(cancellation: &CancellationToken) -> Result<(), UpdateError> {
    if cancellation.is_cancelled() {
        Err(UpdateError::Cancelled)
    } else {
        Ok(())
    }
}
