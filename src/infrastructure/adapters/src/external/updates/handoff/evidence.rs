use super::super::{JournalDirectory, RegisteredComposeInstallation};
use citadel_updates::{UPDATE_PROTOCOL, UpdateError, UpdateOperation, UpdatePhase, is_sha256};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HelperLaunchIntent {
    pub protocol: u32,
    pub nonce: Uuid,
    pub operation: UpdateOperation,
    pub registration: RegisteredComposeInstallation,
}

impl HelperLaunchIntent {
    pub fn helper_name(&self) -> String {
        format!("citadel-update-{}", self.nonce.simple())
    }
    pub(super) fn file(&self, kind: &str) -> String {
        format!("{kind}-{}.json", self.operation.id)
    }
    pub(super) fn verify(&self, state: &JournalDirectory) -> Result<(), UpdateError> {
        state.verify_location()?;
        self.operation.plan.validate(self.operation.created_at)?;
        self.registration.validate()?;
        let op = &self.operation;
        let reg = &self.registration;
        if self.protocol != UPDATE_PROTOCOL
            || self.nonce.is_nil()
            || op.id.is_nil()
            || op.fence.is_nil()
            || op.actor_id.value().is_nil()
            || op.idempotency_key.is_nil()
            || op.phase != UpdatePhase::Preparing
            || op.revision != 0
            || op.updated_at != op.created_at
            || op.checkpoint.is_some()
            || op.message.is_some()
            || op.instance_id != op.plan.instance_id
            || op.instance_id != reg.instance_id
            || op.plan.registration_id != reg.id
            || op.plan.configuration_sha256 != reg.configuration_sha256
            || std::path::Path::new(&op.plan.checkpoint_directory) != state.path()
            || reg.state_directory != state.path()
            || reg.environment_file != reg.directory.join(".env")
            || reg.executor_uid != rustix::process::geteuid().as_raw()
            || reg.executor_gid != rustix::process::getegid().as_raw()
            || !is_sha256(&reg.core_container_id)
            || !is_sha256(&reg.database_container_id)
            || reg.docker_socket != std::path::Path::new("/var/run/docker.sock")
            || reg.fingerprint(std::path::Path::new("/"))? != reg.configuration_sha256
            || state.read::<RegisteredComposeInstallation>("installation.json")? != *reg
        {
            return Err(conflict());
        }
        Ok(())
    }
    pub(super) fn verify_saved(&self, state: &JournalDirectory) -> Result<(), UpdateError> {
        self.verify(state)?;
        if state.read::<Self>(&self.file("launch"))? != *self {
            return Err(conflict());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HelperBinding {
    pub nonce: Uuid,
    pub helper_container_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HelperAcknowledgement {
    pub binding: HelperBinding,
    pub incarnation: Uuid,
    pub started_at: String,
}

/// Call only after the transactional approval. The short registration lock is
/// released before this returns; Core never waits for a helper while holding it.
pub fn prepare_helper_launch(
    state: &JournalDirectory,
    operation: &UpdateOperation,
) -> Result<HelperLaunchIntent, UpdateError> {
    let lock = state.existing_lock()?;
    state.require_registration_only()?;
    let intent = HelperLaunchIntent {
        protocol: UPDATE_PROTOCOL,
        nonce: Uuid::now_v7(),
        operation: operation.clone(),
        registration: state.read("installation.json")?,
    };
    intent.verify(state)?;
    state.verify_lock(&lock)?;
    state.create(&intent.file("launch"), &intent)?;
    Ok(intent)
}

/// Bind a confirmed Docker create before starting it. An ambiguous create must
/// be reconciled by exact name/nonce, never blindly retried or replaced here.
pub fn bind_helper_launch(
    state: &JournalDirectory,
    intent: &HelperLaunchIntent,
    helper_container_id: &str,
) -> Result<(), UpdateError> {
    let lock = state.existing_lock()?;
    intent.verify_saved(state)?;
    if !is_sha256(helper_container_id)
        || [
            intent.registration.core_container_id.as_str(),
            intent.registration.database_container_id.as_str(),
        ]
        .contains(&helper_container_id)
    {
        return Err(conflict());
    }
    state.verify_lock(&lock)?;
    state.create(
        &intent.file("binding"),
        &HelperBinding {
            nonce: intent.nonce,
            helper_container_id: helper_container_id.into(),
        },
    )
}

pub(super) fn binding(
    state: &JournalDirectory,
    intent: &HelperLaunchIntent,
) -> Result<HelperBinding, UpdateError> {
    let binding: HelperBinding = state.read(&intent.file("binding"))?;
    if binding.nonce != intent.nonce
        || !is_sha256(&binding.helper_container_id)
        || binding.helper_container_id == intent.registration.core_container_id
        || binding.helper_container_id == intent.registration.database_container_id
    {
        return Err(conflict());
    }
    Ok(binding)
}

pub(super) fn conflict() -> UpdateError {
    UpdateError::Conflict("Update helper ownership or handoff evidence is missing, stale or inconsistent. Preserve the registered update state.".into())
}
