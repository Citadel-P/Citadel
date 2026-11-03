use crate::{CoreIdentity, UpdateError, UpdateOperation, UpdatePhase};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const UPDATE_PROTOCOL: u32 = 1;

/// The helper owns this journal independently of the database being migrated.
/// A terminal update is never rewritten into a successful recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateJournal {
    pub protocol: u32,
    pub operation: UpdateOperation,
    pub helper_container_id: String,
    /// Complete bounded transition chain, including the reserved initial state.
    pub history: Vec<UpdateOperation>,
    pub verified_target: Option<CoreIdentity>,
    pub completion_acknowledged: bool,
    pub maintenance_released: bool,
}

impl UpdateJournal {
    /// Construct completion only in the target process after catalog and readiness
    /// verification. Import this value transactionally before acknowledging it to
    /// the helper; constructing it does not release installation ownership.
    pub fn completed(
        &self,
        running: &CoreIdentity,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Self, UpdateError> {
        self.validate_for(&self.operation)?;
        if self.operation.phase != UpdatePhase::Verifying
            || running != &self.operation.plan.target
            || self.verified_target.is_some()
            || self.completion_acknowledged
            || self.maintenance_released
        {
            return Err(UpdateError::Conflict(
                "Target verification is not ready for completion.".into(),
            ));
        }
        let mut completed = self.clone();
        completed
            .operation
            .advance(self.operation.revision, UpdatePhase::Succeeded, now)?;
        completed.history.push(completed.operation.clone());
        completed.verified_target = Some(running.clone());
        completed.validate_for(&self.operation)?;
        Ok(completed)
    }

    pub fn validate_for(&self, operation: &UpdateOperation) -> Result<(), UpdateError> {
        let actual = &self.operation;
        if self.protocol != UPDATE_PROTOCOL
            || actual.id != operation.id
            || actual.fence != operation.fence
            || actual.instance_id != operation.instance_id
            || actual.plan != operation.plan
            || actual.actor_id != operation.actor_id
            || actual.idempotency_key != operation.idempotency_key
            || actual.created_at != operation.created_at
            || actual.revision < operation.revision
            || self.helper_container_id.is_empty()
        {
            return Err(UpdateError::Conflict(
                "The update journal does not match its reserved operation.".into(),
            ));
        }
        if self.history.is_empty()
            || self.history.len() > 10
            || self.history.last() != Some(actual)
            || !self.history.contains(operation)
            || self
                .history
                .first()
                .is_none_or(|first| first.phase != UpdatePhase::Preparing || first.revision != 0)
        {
            return Err(UpdateError::Conflict(
                "Update history is missing or conflicts with persisted state.".into(),
            ));
        }
        for pair in self.history.windows(2) {
            let previous = &pair[0];
            let next = &pair[1];
            if next.id != actual.id
                || next.fence != actual.fence
                || next.plan != actual.plan
                || next.instance_id != actual.instance_id
                || next.actor_id != actual.actor_id
                || next.created_at != actual.created_at
                || next.idempotency_key != actual.idempotency_key
                || previous.revision.checked_add(1) != Some(next.revision)
                || !previous.phase.can_transition(next.phase)
                || next.updated_at < previous.updated_at
                || (previous.checkpoint.is_some() && previous.checkpoint != next.checkpoint)
            {
                return Err(UpdateError::Conflict(
                    "Update history contains an invalid transition.".into(),
                ));
            }
            if matches!(
                next.phase,
                UpdatePhase::ReplacingCore | UpdatePhase::Verifying | UpdatePhase::Succeeded
            ) && next
                .checkpoint
                .as_ref()
                .is_none_or(|value| value.is_empty())
            {
                return Err(UpdateError::Conflict(
                    "Replacement requires a completed recovery checkpoint.".into(),
                ));
            }
        }
        if self.completion_acknowledged && actual.phase != UpdatePhase::Succeeded {
            return Err(UpdateError::Conflict(
                "Only a successful update can acknowledge completion.".into(),
            ));
        }
        if (actual.phase == UpdatePhase::Succeeded || self.completion_acknowledged)
            && self.verified_target.as_ref() != Some(&operation.plan.target)
        {
            return Err(UpdateError::Conflict(
                "The journal has no matching target verification.".into(),
            ));
        }
        if self.maintenance_released
            && !self.stopped_before_replacement()
            && (!self.completion_acknowledged || actual.phase != UpdatePhase::Succeeded)
        {
            return Err(UpdateError::Conflict(
                "Maintenance cannot be released before committed completion.".into(),
            ));
        }
        Ok(())
    }

    fn stopped_before_replacement(&self) -> bool {
        matches!(
            self.operation.phase,
            UpdatePhase::Failed | UpdatePhase::Postponed
        ) && !self.completion_acknowledged
            && self.verified_target.is_none()
            && self.history.iter().all(|entry| {
                matches!(
                    entry.phase,
                    UpdatePhase::Preparing
                        | UpdatePhase::Draining
                        | UpdatePhase::BackingUp
                        | UpdatePhase::Failed
                        | UpdatePhase::Postponed
                )
            })
    }

    /// Release a stopped attempt only after its outcome commits and the caller
    /// verifies cleanup and the unchanged running source, including its catalog.
    /// A journal flag alone is not readiness evidence.
    pub fn permits_source_resume(
        &self,
        committed: &UpdateOperation,
        running: &CoreIdentity,
    ) -> Result<(), UpdateError> {
        self.validate_for(committed)?;
        if &self.operation != committed
            || !self.stopped_before_replacement()
            || !self.maintenance_released
            || running != &committed.plan.source
        {
            return Err(UpdateError::Conflict(
                "The stopped update is not committed and released for the unchanged source.".into(),
            ));
        }
        Ok(())
    }
}

/// Target-owned, database-committed completion. The helper cannot manufacture
/// this acknowledgement from a health probe or its own observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletionReceipt {
    pub instance_id: Uuid,
    pub operation_id: Uuid,
    pub fence: Uuid,
    pub revision: u64,
    pub target: CoreIdentity,
}

impl CompletionReceipt {
    pub fn permits_resume(
        &self,
        journal: &UpdateJournal,
        running: &CoreIdentity,
    ) -> Result<(), UpdateError> {
        journal.validate_for(&journal.operation)?;
        let operation = &journal.operation;
        if !journal.maintenance_released
            || !journal.completion_acknowledged
            || operation.phase != UpdatePhase::Succeeded
            || self.instance_id != operation.instance_id
            || self.operation_id != operation.id
            || self.fence != operation.fence
            || self.revision != operation.revision
            || self.target != operation.plan.target
            || &self.target != running
        {
            return Err(UpdateError::Conflict(
                "Completion is not committed and released for the running instance.".into(),
            ));
        }
        Ok(())
    }
}
