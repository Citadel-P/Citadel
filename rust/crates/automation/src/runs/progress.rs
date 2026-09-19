use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{AutomationRun, AutomationRunResult};

#[derive(Debug, Default)]
pub struct AutomationProgress {
    pub run_id: Option<Uuid>,
    pub status: Option<String>,
    pub stream: Option<String>,
    pub progress_message: Option<String>,
    pub error_message: Option<String>,
    pub error: Option<AutomationProgressError>,
}

#[derive(Debug)]
pub struct AutomationProgressError {
    pub code: i32,
    pub message: String,
}

impl AutomationProgress {
    pub fn state(run: &AutomationRun, status: &str) -> Self {
        Self {
            run_id: Some(run.id),
            status: Some(status.into()),
            progress_message: Some(format!(
                "Action \"{}\" {}.",
                run.action_name,
                status.to_lowercase()
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
                    "TimedOut" => 408,
                    "Cancelled" => 499,
                    _ => 500,
                },
                message: message.clone(),
            });
        }
        item
    }
}

/// A slow or disconnected viewer cannot retain a child process indefinitely.
pub(crate) async fn send(
    sender: &mpsc::Sender<AutomationProgress>,
    item: AutomationProgress,
    cancellation: &CancellationToken,
) {
    tokio::select! {
        biased;
        () = cancellation.cancelled() => {},
        result = tokio::time::timeout(std::time::Duration::from_secs(5), sender.send(item)) => {
            if !matches!(result, Ok(Ok(()))) { cancellation.cancel(); }
        }
    }
}
