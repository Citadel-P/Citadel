use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::ScriptRun;
use crate::ScriptRunResult;

#[derive(Debug, Default, Clone)]
pub struct ScriptProgress {
    pub run_id: Option<Uuid>,
    pub status: Option<crate::ScriptRunStatus>,
    pub stream: Option<String>,
    pub progress_message: Option<String>,
    pub error_message: Option<String>,
    pub error: Option<ScriptProgressError>,
}

#[derive(Debug, Clone)]
pub struct ScriptProgressError {
    pub code: i32,
    pub message: String,
}

impl ScriptProgress {
    pub fn state(run: &ScriptRun, status: crate::ScriptRunStatus) -> Self {
        Self {
            run_id: Some(run.id),
            status: Some(status),
            progress_message: Some(format!(
                "Script \"{}\" {}.",
                run.script_name,
                status.as_str().to_lowercase()
            )),
            ..Self::default()
        }
    }

    pub fn completed(run: &ScriptRun, result: &ScriptRunResult) -> Self {
        let mut item = Self::state(run, result.status);
        if let Some(message) = &result.error {
            item.error_message = Some(message.clone());
            item.error = Some(ScriptProgressError {
                code: match result.status {
                    crate::ScriptRunStatus::TimedOut => 408,
                    crate::ScriptRunStatus::Cancelled => 499,
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
    sender: &broadcast::Sender<ScriptProgress>,
    item: ScriptProgress,
    _cancellation: &CancellationToken,
) {
    let _ = sender.send(item);
}
