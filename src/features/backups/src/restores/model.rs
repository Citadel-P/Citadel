use crate::*;

#[derive(Debug, Clone)]
pub struct BackupRestoreRun {
    pub id: Uuid,
    pub backup_run_id: Uuid,
    pub backup_repository_id: Uuid,
    pub source_backup_run_item_id: Option<Uuid>,
    pub target_platform_id: Uuid,
    pub target_docker_node_id: Option<String>,
    pub target_volume_name: String,
    pub overwrite_existing: bool,
    pub status: BackupRestoreStatus,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RestoreClaim {
    pub repository: BackupRepository,
    pub run: BackupRestoreRun,
    pub source: BackupRun,
    pub source_item: Option<BackupRunItem>,
}

#[derive(Debug, Clone)]
pub struct RestoreExecutionResult {
    pub status: BackupRestoreStatus,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub logs: Vec<BackupLog>,
}
