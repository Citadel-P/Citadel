//! Placement for an individual action; workflow orchestration remains separate.
use crate::{AutomationError, AutomationProgress, AutomationRunClaim, AutomationRunResult};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum AutomationExecutor {
    #[default]
    Core,
    Agent {
        #[serde(rename = "agentId")]
        agent_id: Uuid,
    },
    Automatic {
        #[serde(rename = "agentIds")]
        agent_ids: Vec<Uuid>,
    },
}
impl AutomationExecutor {
    pub fn agent_ids(&self) -> &[Uuid] {
        match self {
            Self::Core => &[],
            Self::Agent { agent_id } => std::slice::from_ref(agent_id),
            Self::Automatic { agent_ids } => agent_ids,
        }
    }
    pub fn validate(&self) -> Result<(), AutomationError> {
        let ids = self.agent_ids();
        if ids.len() > 64
            || ids.iter().any(Uuid::is_nil)
            || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
            || matches!(self, Self::Automatic { .. }) && ids.is_empty()
        {
            return Err(AutomationError::Validation(
                "Select between 1 and 64 distinct eligible agents.".into(),
            ));
        }
        Ok(())
    }
}

pub enum RemoteAutomationOutcome {
    Completed(AutomationRunResult),
    Deferred,
}

pub trait RemoteAutomationExecutor: Send + Sync {
    /// Commits executor ownership before sending the script. Only a definitive
    /// pre-execution refusal may return Deferred; ambiguity must never replay.
    fn execute<'a>(
        &'a self,
        claim: &'a AutomationRunClaim,
        script: crate::sandbox::AutomationScript,
        progress: &'a tokio::sync::broadcast::Sender<AutomationProgress>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RemoteAutomationOutcome, AutomationError>>;
    fn resolve<'a>(
        &'a self,
        run: &'a crate::AutomationRun,
        actor: citadel_primitives::ActorId,
        reason: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), AutomationError>>;
    fn reconcile<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, AutomationError>>;
}

impl From<AutomationRunResult> for RemoteAutomationOutcome {
    fn from(result: AutomationRunResult) -> Self {
        Self::Completed(result)
    }
}
