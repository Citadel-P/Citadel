use crate::{UpdateError, UpdatePlan};
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdatePhase {
    Preparing,
    Draining,
    BackingUp,
    ReplacingCore,
    Verifying,
    Succeeded,
    Postponed,
    Failed,
    RecoveryRequired,
}

impl UpdatePhase {
    pub const fn terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Postponed | Self::Failed | Self::RecoveryRequired
        )
    }

    pub const fn can_transition(self, next: Self) -> bool {
        use UpdatePhase::*;
        matches!(
            (self, next),
            (Preparing, Draining | Failed | Postponed)
                | (Draining, BackingUp | Failed | Postponed | RecoveryRequired)
                | (BackingUp, ReplacingCore | Failed | RecoveryRequired)
                | (ReplacingCore, Verifying | RecoveryRequired)
                | (Verifying, Succeeded | RecoveryRequired)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateOperation {
    pub id: Uuid,
    pub instance_id: Uuid,
    pub actor_id: ActorId,
    pub idempotency_key: Uuid,
    pub fence: Uuid,
    pub plan: UpdatePlan,
    pub phase: UpdatePhase,
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub checkpoint: Option<String>,
    pub message: Option<String>,
}

impl UpdateOperation {
    pub fn approve(
        plan: UpdatePlan,
        actor_id: ActorId,
        key: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Self, UpdateError> {
        plan.validate(now)?;
        if key.is_nil() || actor_id.value().is_nil() {
            return Err(UpdateError::Conflict(
                "The update review has expired or is invalid. Review again.".into(),
            ));
        }
        Ok(Self {
            id: Uuid::now_v7(),
            instance_id: plan.instance_id,
            actor_id,
            idempotency_key: key,
            fence: Uuid::now_v7(),
            plan,
            phase: UpdatePhase::Preparing,
            revision: 0,
            created_at: now,
            updated_at: now,
            checkpoint: None,
            message: None,
        })
    }

    pub fn advance(
        &mut self,
        expected_revision: u64,
        next: UpdatePhase,
        now: DateTime<Utc>,
    ) -> Result<(), UpdateError> {
        if self.revision != expected_revision
            || !self.phase.can_transition(next)
            || now < self.updated_at
        {
            return Err(UpdateError::Conflict(
                "The update transition is stale or not permitted.".into(),
            ));
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| UpdateError::Conflict("Update revision exhausted.".into()))?;
        self.phase = next;
        self.updated_at = now;
        Ok(())
    }
}
