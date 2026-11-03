use super::super::{
    InstallationLock, InstallerComposeInputs, JournalDirectory, RegisteredComposeInstallation,
    registration::fingerprint_inputs,
};
use super::storage::{cancelled, create_attempt, failed, sync_bundle, write_input};
use crate::external::backups::system_recovery::{
    PgDumpSystemBackupBuilder, inspect_recovery_bundle,
};
use citadel_backups::{BackupError, CitadelSystemBackupBuilder};
use citadel_updates::{CoreIdentity, UpdateError, UpdateOperation, UpdatePhase};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

/// Artifact evidence only, not proof of writer quiescence, restore success or
/// permission to replace Core. Its bundle uses the existing recovery format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedUpdateCheckpoint {
    pub format: u32,
    pub operation_id: Uuid,
    pub fence: Uuid,
    pub instance_id: Uuid,
    pub source: CoreIdentity,
    pub bundle: PathBuf,
    pub bundle_checksums_sha256: String,
    pub configuration_sha256: String,
    pub installer_inputs: Vec<CheckpointInstallerInput>,
    pub required_external_configuration: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckpointInstallerInput {
    pub original_path: PathBuf,
    pub saved_name: String,
    pub sha256: String,
}

/// Low-level helper preparation, not a live-Core update entry point. The caller
/// must already own the durable operation and host lock, finish draining, and
/// stop/quiesce Core and all source writers before invoking this function. Keep
/// that ownership until handoff/recovery is resolved. No phase is advanced here.
/// Cancellation is cooperative: await this future so pg_dump cleanup completes.
pub async fn prepare_update_checkpoint(
    state: &JournalDirectory,
    owner: &InstallationLock,
    operation: &UpdateOperation,
    backup: &PgDumpSystemBackupBuilder,
    cancellation: &CancellationToken,
) -> Result<PreparedUpdateCheckpoint, UpdateError> {
    cancelled(cancellation)?;
    state.verify_lock(owner)?;
    operation.plan.validate(operation.created_at)?;
    let journal = state.journal(operation.id)?;
    journal.validate_for(operation)?;
    if journal.operation != *operation
        || operation.id.is_nil()
        || operation.fence.is_nil()
        || operation.actor_id.value().is_nil()
        || operation.idempotency_key.is_nil()
        || operation.instance_id != operation.plan.instance_id
        || operation.phase != UpdatePhase::BackingUp
        || operation.revision != 2
        || operation.checkpoint.is_some()
        || !citadel_updates::is_sha256(&journal.helper_container_id)
        || journal.verified_target.is_some()
        || journal.completion_acknowledged
        || journal.maintenance_released
    {
        return Err(UpdateError::Conflict(
            "Checkpoint preparation requires the owned backup phase.".into(),
        ));
    }
    let registration: RegisteredComposeInstallation = state.read("installation.json")?;
    registration.validate()?;
    if registration.id != operation.plan.registration_id
        || registration.instance_id != operation.instance_id
        || registration.state_directory != state.path()
        || registration.executor_uid != rustix::process::geteuid().as_raw()
        || registration.executor_gid != rustix::process::getegid().as_raw()
        || registration.configuration_sha256 != operation.plan.configuration_sha256
        || registration.environment_file != registration.directory.join(".env")
        || PathBuf::from(&operation.plan.checkpoint_directory) != state.path()
    {
        return Err(UpdateError::Conflict(
            "Checkpoint registration does not match its approved operation.".into(),
        ));
    }
    let inputs = InstallerComposeInputs {
        directory: registration.directory.clone(),
        project: registration.project.clone(),
        compose_files: registration.compose_files.clone(),
    };
    let captured = inputs
        .capture()?
        .into_iter()
        .map(|(path, bytes)| (path, Zeroizing::new(bytes)))
        .collect::<Vec<_>>();
    // Capture once, then use those bytes for both the fingerprint and snapshot.
    let fingerprint = fingerprint_inputs(&inputs.project, &captured);
    if fingerprint != registration.configuration_sha256 {
        return Err(changed());
    }
    let stage_path = state.path().join(format!("checkpoint-{}", operation.id));
    let backup = backup
        .for_checkpoint(
            stage_path,
            operation.instance_id,
            &operation.plan.source.version,
        )
        .map_err(|_| failed())?;
    cancelled(cancellation)?;
    let stage = create_attempt(state, &format!("checkpoint-{}", operation.id))?;
    stage.create("operation.json", operation)?;
    stage.create("installation.json", &registration)?;
    let mut installer_inputs = Vec::new();
    for (index, (path, bytes)) in captured.iter().enumerate() {
        cancelled(cancellation)?;
        let saved_name = format!("input-{index}");
        installer_inputs.push(CheckpointInstallerInput {
            original_path: path.clone(),
            sha256: write_input(&stage, &saved_name, bytes)?,
            saved_name,
        });
    }
    stage.sync()?;
    let result = backup.build(operation.id, cancellation).await;
    // Cancellation must not hide unresolved subprocess/bundle cleanup.
    if matches!(result, Err(BackupError::CleanupPending)) {
        return Err(failed());
    }
    cancelled(cancellation)?;
    let bundle = result.map_err(|_| failed())?;
    let checked = tokio::select! {
        biased;
        () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
        checked = inspect_recovery_bundle(&bundle) => checked.map_err(|_| failed())?,
    };
    if checked.instance_id != operation.instance_id
        || checked.core_version != operation.plan.source.version
    {
        return Err(failed());
    }
    sync_bundle(&bundle, &checked.files, cancellation).await?;
    let receipt = PreparedUpdateCheckpoint {
        format: 1,
        operation_id: operation.id,
        fence: operation.fence,
        instance_id: operation.instance_id,
        source: operation.plan.source.clone(),
        bundle,
        bundle_checksums_sha256: checked.checksums_sha256,
        configuration_sha256: fingerprint,
        installer_inputs,
        required_external_configuration: checked.required_external_configuration,
    };
    state.verify_lock(owner)?;
    stage.verify_location()?;
    if state.journal(operation.id)? != journal
        || state.read::<RegisteredComposeInstallation>("installation.json")? != registration
        || registration.fingerprint(std::path::Path::new("/"))? != registration.configuration_sha256
    {
        return Err(changed());
    }
    cancelled(cancellation)?;
    stage.create("checkpoint.json", &receipt)?;
    Ok(receipt)
}

fn changed() -> UpdateError {
    UpdateError::Conflict(
        "Installer inputs or update ownership changed during checkpoint preparation.".into(),
    )
}
