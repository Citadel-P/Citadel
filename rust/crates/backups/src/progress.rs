use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRunStreamItem {
    pub run_id: Uuid,
    pub status: Option<String>,
    pub message: Option<String>,
    pub stream: Option<String>,
    pub exit_code: Option<i32>,
}

impl BackupRunStreamItem {
    pub fn message(id: Uuid, status: &str, message: impl Into<String>) -> Self {
        Self {
            run_id: id,
            status: Some(status.into()),
            message: Some(message.into()),
            stream: Some("stdout".into()),
            exit_code: None,
        }
    }
}

pub fn is_terminal(status: &str) -> bool {
    !matches!(
        status,
        "Queued" | "Preparing" | "Running" | "Processing" | "ApplyingRetention"
    )
}
