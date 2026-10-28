/// Committed queue work, including stale runs recovered even when no run is claimable.
pub struct ScriptClaimResult {
    pub claim: Option<ScriptRunClaim>,
    pub recovered_scripts: Vec<uuid::Uuid>,
}

use crate::*;

#[derive(Debug, Clone)]
pub struct ScriptRunClaim {
    pub run: ScriptRun,
}

pub struct ScriptRunResult {
    pub status: ScriptRunStatus,
    pub exit_code: Option<i32>,
    pub logs: String,
    pub error: Option<String>,
}

impl ScriptRunResult {
    pub(crate) fn success(exit_code: Option<i32>, logs: String) -> Self {
        Self {
            status: crate::ScriptRunStatus::Succeeded,
            exit_code,
            logs,
            error: None,
        }
    }
    pub(crate) fn failed(exit_code: Option<i32>, message: String) -> Self {
        Self {
            status: crate::ScriptRunStatus::Failed,
            exit_code,
            logs: message.clone(),
            error: Some(message),
        }
    }
    pub(crate) fn cancelled() -> Self {
        Self {
            status: crate::ScriptRunStatus::Cancelled,
            exit_code: None,
            logs: String::new(),
            error: Some("Script run was cancelled.".to_owned()),
        }
    }
    pub(crate) fn timed_out() -> Self {
        Self {
            status: crate::ScriptRunStatus::TimedOut,
            exit_code: None,
            logs: String::new(),
            error: Some("Script run timed out.".to_owned()),
        }
    }
}
