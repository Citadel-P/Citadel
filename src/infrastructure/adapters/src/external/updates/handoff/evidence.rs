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
        self.verify_identity(state)?;
        if self.registration.fingerprint(std::path::Path::new("/"))?
            != self.registration.configuration_sha256
        {
            return Err(conflict());
        }
        Ok(())
    }

    fn verify_identity(&self, state: &JournalDirectory) -> Result<(), UpdateError> {
        state.verify_location()?;
        reject_recovered(state, self.operation.id)?;
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
            || state.read::<RegisteredComposeInstallation>("installation.json")? != *reg
        {
            return Err(conflict());
        }
        Ok(())
    }
    pub(super) fn verify_saved(&self, state: &JournalDirectory) -> Result<(), UpdateError> {
        self.verify(state)?;
        self.verify_saved_identity(state)
    }

    /// Post-publication callers must separately verify the approved target input
    /// fingerprint. Source-side authorization always retains verify_saved().
    pub(super) fn verify_saved_identity(
        &self,
        state: &JournalDirectory,
    ) -> Result<(), UpdateError> {
        self.verify_identity(state)?;
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
    pub started_at: String,
}

/// Call only after the transactional approval. The short registration lock is
/// released before this returns; Core never waits for a helper while holding it.
pub(super) fn prepare_helper_launch(
    state: &JournalDirectory,
    operation: &UpdateOperation,
) -> Result<HelperLaunchIntent, UpdateError> {
    let lock = state.existing_lock()?;
    reject_recovered(state, operation.id)?;
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

fn reject_recovered(state: &JournalDirectory, id: Uuid) -> Result<(), UpdateError> {
    let path = state.path().join(format!("checkpoint-{id}"));
    if path.try_exists().map_err(|_| conflict())?
        && JournalDirectory::open(path)?
            .read_optional::<serde_json::Value>("source-recovery.json")?
            .is_some()
    {
        return Err(conflict());
    }
    Ok(())
}
