//! Placement for an individual script; workflow orchestration remains separate.
use crate::{ScriptError, ScriptProgress, ScriptRunClaim, ScriptRunResult};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ScriptExecutor {
    #[default]
    Core,
    Agent {
        #[serde(rename = "agentId")]
        agent_id: Uuid,
    },
    Automatic,
}
impl ScriptExecutor {
    pub fn validate(&self) -> Result<(), ScriptError> {
        if matches!(self, Self::Agent { agent_id } if agent_id.is_nil()) {
            return Err(ScriptError::Validation("Select an agent.".into()));
        }
        Ok(())
    }
}

pub enum RemoteScriptOutcome {
    Completed(ScriptRunResult),
    Deferred,
}

pub trait RemoteScriptExecutor: Send + Sync {
    /// Commits executor ownership before sending the script. Only a definitive
    /// pre-execution refusal may return Deferred; ambiguity must never replay.
    fn execute<'a>(
        &'a self,
        claim: &'a ScriptRunClaim,
        script: crate::sandbox::ScriptExecution,
        progress: &'a tokio::sync::broadcast::Sender<ScriptProgress>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RemoteScriptOutcome, ScriptError>>;
    fn resolve<'a>(
        &'a self,
        run: &'a crate::ScriptRun,
        actor: citadel_primitives::ActorId,
        reason: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), ScriptError>>;
    fn reconcile<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, ScriptError>>;
}

impl From<ScriptRunResult> for RemoteScriptOutcome {
    fn from(result: ScriptRunResult) -> Self {
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
            let executor: ScriptExecutor = serde_json::from_value(value).unwrap();
            assert_eq!(executor, ScriptExecutor::Automatic);
            executor.validate().unwrap();
            assert_eq!(
                serde_json::to_value(executor).unwrap(),
                serde_json::json!({"kind":"Automatic"})
            );
        }
        assert!(
            ScriptExecutor::Agent {
                agent_id: Uuid::nil()
            }
            .validate()
            .is_err()
        );
    }
}
