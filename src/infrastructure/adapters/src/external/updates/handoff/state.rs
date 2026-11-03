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
