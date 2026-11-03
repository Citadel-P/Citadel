//! Helper-owned execution state. Phase and its evidence are one atomic write.
use super::super::{InstallationLock, JournalDirectory, bootstrap::ObservedReplacement};
use super::{
    checkpoint::SourceProcess,
    evidence::{HelperAcknowledgement, HelperLaunchIntent, binding, conflict},
    preparation::PreparedTarget,
    replacement::CoreReplacementIntent,
};
use citadel_updates::{UpdateError, UpdateJournal, UpdatePhase};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Import a failed helper's final observation while the original Core is alive.
/// Taking the lock proves the driver cannot still write or authorize replacement.
/// This never releases maintenance, removes evidence or resumes a process.
pub(in crate::external::updates) async fn failed_journal(
    operation: &citadel_updates::UpdateOperation,
    process: &dyn citadel_execution::ProcessRunner,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<UpdateJournal, UpdateError> {
    let state = JournalDirectory::open(operation.plan.checkpoint_directory.clone().into())?;
    let owner = state.existing_lock()?;
    let intent: HelperLaunchIntent = state.read(&format!("launch-{}.json", operation.id))?;
    if intent.operation != *operation {
        return Err(conflict());
    }
    intent.verify_saved(&state)?;
    let execution = ExecutionJournal::read_for(&state, &intent)?;
    if !matches!(
        execution.journal.operation.phase,
        UpdatePhase::Failed | UpdatePhase::RecoveryRequired
    ) || execution.journal.maintenance_released
    {
        return Err(conflict());
    }
    let home = super::runtime::private_home()?;
    let docker = super::super::docker::LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: std::time::Instant::now() + std::time::Duration::from_secs(30),
    };
    let helper =
        super::verification::helper_container(&docker, &intent, &execution.acknowledgement.binding)
            .await?;
    if helper["State"]["Running"] != false
        || helper["State"]["Status"] != "exited"
        || helper["State"]["StartedAt"] != execution.acknowledgement.started_at
    {
        return Err(conflict());
    }
    state.verify_lock(&owner)?;
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    Ok(execution.journal)
}

/// Only the launcher creates launch/binding records. After acknowledgement the
/// detached driver is the sole writer of this journal. Other processes publish
/// immutable shutdown/completion messages, so they cannot overwrite its state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::external::updates) struct ExecutionJournal {
    pub(in crate::external::updates) journal: UpdateJournal,
    pub(super) acknowledgement: HelperAcknowledgement,
    pub(super) prepared: Option<PreparedTarget>,
    /// Written before SIGTERM. BackingUp in `journal` confirms the clean exit.
    pub(super) stop: Option<SourceProcess>,
    /// Written together with ReplacingCore before publishing inputs or Compose.
    pub(super) replacement: Option<CoreReplacementIntent>,
    pub(super) submitted: bool,
    /// Written together with Verifying after confirmed submission and observation.
    pub(super) target: Option<ObservedReplacement>,
}

impl ExecutionJournal {
    pub(super) fn read(state: &JournalDirectory, id: Uuid) -> Result<Self, UpdateError> {
        let saved: Self = state.read(&format!("{id}.json"))?;
        if saved.journal.operation.id != id {
            return Err(conflict());
        }
        saved.validate()?;
        Ok(saved)
    }

    /// Read one complete execution snapshot bound to the reserved operation.
    /// Callers check the source or target configuration for their current phase.
    pub(super) fn read_for(
        state: &JournalDirectory,
        intent: &HelperLaunchIntent,
    ) -> Result<Self, UpdateError> {
        state.require_lock_held()?;
        intent.verify_saved_identity(state)?;
        let execution = Self::read(state, intent.operation.id)?;
        execution.journal.validate_for(&intent.operation)?;
        if execution.acknowledgement.binding != binding(state, intent)? {
            return Err(conflict());
        }
        Ok(execution)
    }

    fn validate(&self) -> Result<(), UpdateError> {
        let initial = self.journal.history.first().ok_or_else(conflict)?;
        self.journal.validate_for(initial)?;
        let reached = |phase| {
            self.journal
                .history
                .iter()
                .any(|entry| entry.phase == phase)
        };
        if self.acknowledgement.started_at.is_empty()
            || self.acknowledgement.binding.helper_container_id != self.journal.helper_container_id
            || (self.stop.is_some() && self.prepared.is_none())
            || (reached(UpdatePhase::BackingUp) && self.stop.is_none())
            || self.replacement.is_some() != reached(UpdatePhase::ReplacingCore)
            || self.target.is_some() != reached(UpdatePhase::Verifying)
            || (self.submitted && self.replacement.is_none())
            || (self.target.is_some() && !self.submitted)
        {
            return Err(conflict());
        }
        Ok(())
    }

    pub(super) fn save(
        &self,
        state: &JournalDirectory,
        owner: &InstallationLock,
    ) -> Result<(), UpdateError> {
        state.verify_lock(owner)?;
        self.validate()?;
        state.write(&format!("{}.json", self.journal.operation.id), self)
    }
}
