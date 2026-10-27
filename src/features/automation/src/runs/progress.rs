use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::AutomationRun;
use crate::AutomationRunResult;

#[derive(Debug, Default, Clone)]
pub struct AutomationProgress {
    pub run_id: Option<Uuid>,
    pub status: Option<crate::AutomationRunStatus>,
    pub stream: Option<String>,
    pub progress_message: Option<String>,
    pub error_message: Option<String>,
    pub error: Option<AutomationProgressError>,
}

#[derive(Debug, Clone)]
pub struct AutomationProgressError {
    pub code: i32,
    pub message: String,
}

impl AutomationProgress {
    pub fn state(run: &AutomationRun, status: crate::AutomationRunStatus) -> Self {
        Self {
            run_id: Some(run.id),
            status: Some(status),
            progress_message: Some(format!(
                "Action \"{}\" {}.",
                run.action_name,
                status.as_str().to_lowercase()
            )),
            ..Self::default()
        }
    }

    pub fn completed(run: &AutomationRun, result: &AutomationRunResult) -> Self {
        let mut item = Self::state(run, result.status);
        if let Some(message) = &result.error {
            item.error_message = Some(message.clone());
            item.error = Some(AutomationProgressError {
                code: match result.status {
                    crate::AutomationRunStatus::TimedOut => 408,
                    crate::AutomationRunStatus::Cancelled => 499,
                    _ => 500,
                },
                message: message.clone(),
            });
        }
        item
    }
}

/// Progress is observational. Slow viewers never cancel a durable execution.
pub(crate) async fn send(
    sender: &broadcast::Sender<AutomationProgress>,
    item: AutomationProgress,
    _cancellation: &CancellationToken,
) {
    let _ = sender.send(item);
}
