use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BackupProgressItem {
    pub run_id: Uuid,
    pub status: Option<String>,
    pub message: Option<String>,
    pub stream: Option<String>,
    pub exit_code: Option<i32>,
}

impl BackupProgressItem {
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

// This stream carries both backup and restore states; restore states are a
// subset of the backup run vocabulary. Unknown wire values are never terminal.
pub fn is_terminal(status: &str) -> bool {
    status
        .parse::<crate::BackupRunStatus>()
        .is_ok_and(crate::BackupRunStatus::is_terminal)
}
