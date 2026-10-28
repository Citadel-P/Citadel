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
    Automatic,
}
impl AutomationExecutor {
    pub fn validate(&self) -> Result<(), AutomationError> {
        if matches!(self, Self::Agent { agent_id } if agent_id.is_nil()) {
            return Err(AutomationError::Validation("Select an agent.".into()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_needs_no_agent_list_and_normalizes_saved_configuration() {
        for value in [
            serde_json::json!({"kind": "Automatic"}),
            serde_json::json!({"kind": "Automatic", "agentIds": [Uuid::now_v7()]}),
        ] {
            let executor: AutomationExecutor = serde_json::from_value(value).unwrap();
            assert_eq!(executor, AutomationExecutor::Automatic);
            executor.validate().unwrap();
            assert_eq!(
                serde_json::to_value(executor).unwrap(),
                serde_json::json!({"kind":"Automatic"})
            );
        }
        assert!(
            AutomationExecutor::Agent {
                agent_id: Uuid::nil()
            }
            .validate()
            .is_err()
        );
    }
}
