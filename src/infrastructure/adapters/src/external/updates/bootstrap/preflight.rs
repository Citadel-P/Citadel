//! Live preflight evidence; never approval to replace Core.
use super::super::docker::{SOCKET, failed, refuse};
use super::super::registration::fingerprint_inputs;
use super::super::{InstallerComposeInputs, JournalDirectory, RegisteredComposeInstallation};
use super::inspection::inspect_installation;
use super::registration::REGISTRATION;
use citadel_execution::ProcessRunner;
use citadel_updates::UpdateError;
use std::path::Path;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckedUpdateInstallation {
    pub registration: RegisteredComposeInstallation,
    pub database: citadel_updates::UpdateDatabaseState,
    pub recovery_keys: super::super::CheckedRecoveryKeys,
    pub checkpoint_storage: super::checkpoint::CheckpointStorage,
}

/// Recheck the saved invocation and live resources without creating or repairing
/// registration. This is one preflight input, not transition or execution approval.
pub async fn check_registered_compose_installation(
    state_directory: &Path,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<CheckedUpdateInstallation, UpdateError> {
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    let state = JournalDirectory::open(state_directory.into())?;
    let _lock = state.existing_lock()?;
    let registered: RegisteredComposeInstallation = state.read(REGISTRATION)?;
    registered.validate()?;
    if registered.state_directory != state_directory
        || registered.executor_uid != rustix::process::geteuid().as_raw()
        || registered.executor_gid != rustix::process::getegid().as_raw()
        || registered.docker_socket != Path::new(SOCKET)
    {
        return Err(refuse(
            "Registration does not match the state directory, executor or local Docker socket.",
        ));
    }
    state.require_registration_only()?;
    let inputs = InstallerComposeInputs {
        directory: registered.directory.clone(),
        project: registered.project.clone(),
        compose_files: registered.compose_files.clone(),
    };
    // Reject edited inputs before calling Compose or contacting Docker.
    if fingerprint_inputs(&inputs.project, &inputs.capture()?) != registered.configuration_sha256 {
        return Err(UpdateError::Conflict(
            "Registered installer inputs have changed.".into(),
        ));
    }
    let (observed, source) = inspect_installation(
        &inputs,
        registered.state_directory.clone(),
        registered.id,
        process,
        cancellation,
        true,
    )
    .await?;
    state.verify_location()?;
    state.require_registration_only()?;
    let current: RegisteredComposeInstallation = state.read(REGISTRATION)?;
    if current != registered || observed != registered {
        return Err(UpdateError::Conflict("The live installation no longer matches its registration. Registration and recovery state were preserved.".into()));
    }
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    let source = source.ok_or_else(failed)?;
    let database = source.database;
    let checkpoint_storage = super::checkpoint::check_storage(&state, database.database_bytes)?;
    state.verify_location()?;
    state.require_registration_only()?;
    if state.read::<RegisteredComposeInstallation>(REGISTRATION)? != registered {
        return Err(UpdateError::Conflict(
            "Registration changed during checkpoint storage assessment.".into(),
        ));
    }
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    Ok(CheckedUpdateInstallation {
        registration: observed,
        database,
        recovery_keys: source.recovery_keys,
        checkpoint_storage,
    })
}
