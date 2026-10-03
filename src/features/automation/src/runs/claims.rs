use crate::*;

#[derive(Debug, Clone)]
pub struct AutomationRunClaim {
    pub run: AutomationRun,
}

pub struct AutomationRunResult {
    pub status: AutomationRunStatus,
    pub exit_code: Option<i32>,
    pub logs: String,
    pub error: Option<String>,
}

impl AutomationRunResult {
    pub(crate) fn success(exit_code: Option<i32>, logs: String) -> Self {
        Self {
            status: crate::AutomationRunStatus::Succeeded,
            exit_code,
            logs,
            error: None,
        }
    }
    pub(crate) fn failed(exit_code: Option<i32>, message: String) -> Self {
        Self {
            status: crate::AutomationRunStatus::Failed,
            exit_code,
            logs: message.clone(),
            error: Some(message),
        }
    }
    pub(crate) fn cancelled() -> Self {
        Self {
            status: crate::AutomationRunStatus::Cancelled,
            exit_code: None,
            logs: String::new(),
            error: Some("Automation run was cancelled.".to_owned()),
        }
    }
    pub(crate) fn timed_out() -> Self {
        Self {
            status: crate::AutomationRunStatus::TimedOut,
            exit_code: None,
            logs: String::new(),
            error: Some("Automation run timed out.".to_owned()),
        }
    }
}
