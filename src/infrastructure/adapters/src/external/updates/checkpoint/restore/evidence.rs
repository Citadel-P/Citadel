use super::super::super::{
    InstallationLock, JournalDirectory, RegisteredComposeInstallation,
    files::{open_secure, read_bounded},
    registration::fingerprint_inputs,
};
use super::super::{
    PreparedUpdateCheckpoint,
    storage::{cancelled, sync_bundle},
};
use crate::external::backups::system_recovery::inspect_recovery_bundle;
use citadel_updates::{UpdateError, UpdateJournal, UpdateOperation, UpdatePhase};
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

pub(super) struct Evidence {
    pub stage: JournalDirectory,
    pub receipt: PreparedUpdateCheckpoint,
    pub registration: RegisteredComposeInstallation,
    pub journal: UpdateJournal,
}

pub(super) async fn check(
    state: &JournalDirectory,
    owner: &InstallationLock,
    operation: &UpdateOperation,
    cancellation: &CancellationToken,
) -> Result<Evidence, UpdateError> {
    cancelled(cancellation)?;
    state.verify_lock(owner)?;
    let journal = state.journal(operation.id)?;
    journal.validate_for(operation)?;
    operation.plan.validate(operation.created_at)?;
    if operation.id.is_nil()
        || operation.fence.is_nil()
        || operation.actor_id.value().is_nil()
        || operation.idempotency_key.is_nil()
        || !citadel_updates::is_sha256(&journal.helper_container_id)
        || operation.instance_id != operation.plan.instance_id
        || journal.operation != *operation
        || operation.phase != UpdatePhase::BackingUp
        || operation.revision != 2
        || operation.checkpoint.is_some()
        || journal.verified_target.is_some()
        || journal.completion_acknowledged
        || journal.maintenance_released
    {
        return Err(failed());
    }
    let registration: RegisteredComposeInstallation = state.read("installation.json")?;
    registration.validate()?;
    if registration.id != operation.plan.registration_id
        || registration.instance_id != operation.instance_id
        || registration.state_directory != state.path()
        || Path::new(&operation.plan.checkpoint_directory) != state.path()
        || registration.executor_uid != rustix::process::geteuid().as_raw()
        || registration.executor_gid != rustix::process::getegid().as_raw()
        || registration.configuration_sha256 != operation.plan.configuration_sha256
        || registration.fingerprint(Path::new("/"))? != registration.configuration_sha256
    {
        return Err(failed());
    }
    let stage = JournalDirectory::open(state.path().join(format!("checkpoint-{}", operation.id)))?;
    let receipt: PreparedUpdateCheckpoint = stage.read("checkpoint.json")?;
    if receipt.format != 1
        || receipt.operation_id != operation.id
        || receipt.fence != operation.fence
        || receipt.instance_id != operation.instance_id
        || receipt.source != operation.plan.source
        || receipt.configuration_sha256 != registration.configuration_sha256
        || receipt.bundle != stage.path().join(operation.id.simple().to_string())
        || stage.read::<UpdateOperation>("operation.json")? != *operation
        || stage.read::<RegisteredComposeInstallation>("installation.json")? != registration
    {
        return Err(failed());
    }
    let paths: Vec<_> = registration
        .compose_files
        .iter()
        .chain([&registration.environment_file])
        .collect();
    if paths.len() != receipt.installer_inputs.len() {
        return Err(failed());
    }
    let mut captured = Vec::new();
    for (index, (path, input)) in paths.into_iter().zip(&receipt.installer_inputs).enumerate() {
        if input.original_path != *path || input.saved_name != format!("input-{index}") {
            return Err(failed());
        }
        use std::os::unix::fs::MetadataExt;
        let file = open_secure(&stage.path().join(&input.saved_name), false)?;
        let metadata = file.metadata().map_err(|_| failed())?;
        if metadata.nlink() != 1 || metadata.mode() & 0o077 != 0 {
            return Err(failed());
        }
        let bytes = Zeroizing::new(read_bounded(file, 1024 * 1024)?);
        if Sha256::digest(&*bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            != input.sha256
        {
            return Err(failed());
        }
        captured.push((path.clone(), bytes));
    }
    if fingerprint_inputs(&registration.project, &captured) != receipt.configuration_sha256 {
        return Err(failed());
    }
    let bundle = tokio::select! {
        biased;
        () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
        value = inspect_recovery_bundle(&receipt.bundle) => value.map_err(|_| failed())?,
    };
    if bundle.instance_id != operation.instance_id
        || bundle.core_version != receipt.source.version
        || bundle.checksums_sha256 != receipt.bundle_checksums_sha256
        || bundle.required_external_configuration != receipt.required_external_configuration
    {
        return Err(failed());
    }
    sync_bundle(&receipt.bundle, &bundle.files, cancellation).await?;
    state.verify_lock(owner)?;
    Ok(Evidence {
        stage,
        receipt,
        registration,
        journal,
    })
}

pub(super) fn failed() -> UpdateError {
    UpdateError::Unavailable("Checkpoint restore verification failed. Preserve update state; no replacement or source resume was authorized.".into())
}

/// Re-read the saved restore proof and bundle before authorizing replacement.
/// This performs no restore or cleanup and cannot repair an incomplete rehearsal.
pub(in crate::external::updates) async fn verified_checkpoint(
    state: &JournalDirectory,
    owner: &InstallationLock,
    operation: &UpdateOperation,
    docker: &super::super::super::docker::LocalDocker<'_>,
) -> Result<PreparedUpdateCheckpoint, UpdateError> {
    let evidence = check(state, owner, operation, docker.cancellation).await?;
    let receipt: super::RestoredUpdateCheckpoint = evidence.stage.read("restore-verified.json")?;
    let sandbox: super::sandbox::Sandbox = evidence.stage.read("restore-intent.json")?;
    let id: String = evidence.stage.read("restore-container.json")?;
    if receipt.format != 2
        || receipt.operation_id != operation.id
        || receipt.fence != operation.fence
        || receipt.instance_id != operation.instance_id
        || receipt.catalog_sha256 != operation.plan.source.catalog_sha256
        || receipt.bundle_checksums_sha256 != evidence.receipt.bundle_checksums_sha256
        || receipt.database_image != sandbox.image
        || receipt.sandbox_id != id
        || !citadel_updates::is_sha256(&id)
        || sandbox.dump != evidence.receipt.bundle.join("database/citadel.dump")
    {
        return Err(failed());
    }
    let raw = docker
        .json(&[
            "container",
            "inspect",
            &evidence.registration.database_container_id,
        ])
        .await?;
    let database = super::super::super::docker::single(&raw)?;
    if database["Id"] != evidence.registration.database_container_id
        || database["Image"] != receipt.database_image
    {
        return Err(failed());
    }
    for filter in [format!("id={id}"), format!("name=^/{}$", sandbox.name())] {
        if !docker.containers(&filter).await?.is_empty() {
            return Err(failed());
        }
    }
    state.verify_lock(owner)?;
    Ok(evidence.receipt)
}
