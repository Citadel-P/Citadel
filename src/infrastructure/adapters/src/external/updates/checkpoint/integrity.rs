use super::super::{
    InstallationLock, JournalDirectory, RegisteredComposeInstallation,
    files::{open_secure, read_bounded},
    registration::fingerprint_inputs,
};
use super::{
    PreparedUpdateCheckpoint,
    preparation::check_preparation,
    storage::{cancelled, sync_bundle},
};
use crate::external::backups::system_recovery::inspect_recovery_bundle;
use citadel_updates::{UpdateError, UpdateOperation};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

/// Check the saved bundle and installer inputs immediately before replacement.
/// Integrity verification does not claim that PostgreSQL restored this archive.
pub(in crate::external::updates) async fn verify_checkpoint_integrity(
    state: &JournalDirectory,
    owner: &InstallationLock,
    operation: &UpdateOperation,
    cancellation: &CancellationToken,
) -> Result<PreparedUpdateCheckpoint, UpdateError> {
    cancelled(cancellation)?;
    let (journal, registration) = check_preparation(state, owner, operation)?;
    let stage = JournalDirectory::open(state.path().join(format!("checkpoint-{}", operation.id)))?;
    let receipt =
        verify_checkpoint_artifacts(&stage, operation, &registration, cancellation).await?;
    let (current_journal, current_registration) = check_preparation(state, owner, operation)?;
    if current_journal != journal || current_registration != registration {
        return Err(failed());
    }
    cancelled(cancellation)?;
    Ok(receipt)
}

/// Shared immutable artifact verification for replacement and explicit recovery.
pub(super) async fn verify_checkpoint_artifacts(
    stage: &JournalDirectory,
    operation: &UpdateOperation,
    registration: &RegisteredComposeInstallation,
    cancellation: &CancellationToken,
) -> Result<PreparedUpdateCheckpoint, UpdateError> {
    let receipt: PreparedUpdateCheckpoint = stage.read("checkpoint.json")?;
    if receipt.format != 1
        || receipt.operation_id != operation.id
        || receipt.fence != operation.fence
        || receipt.instance_id != operation.instance_id
        || receipt.source != operation.plan.source
        || receipt.configuration_sha256 != registration.configuration_sha256
        || receipt.bundle != stage.path().join(operation.id.simple().to_string())
        || stage.read::<UpdateOperation>("operation.json")? != *operation
        || stage.read::<RegisteredComposeInstallation>("installation.json")? != *registration
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
    stage.verify_location()?;
    cancelled(cancellation)?;
    Ok(receipt)
}

pub(super) fn failed() -> UpdateError {
    UpdateError::Unavailable("Checkpoint integrity verification failed. Preserve update state; no replacement or source resume was authorized.".into())
}
