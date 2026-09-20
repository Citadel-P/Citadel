use crate::*;

#[derive(Debug, Clone)]
pub struct BackupRun {
    pub id: Uuid,
    pub backup_policy_id: Uuid,
    pub policy_name_snapshot: String,
    pub backup_repository_id: Uuid,
    pub repository_type_snapshot: String,
    pub source_snapshot: Value,
    pub trigger: String,
    pub status: String,
    pub snapshot_availability: String,
    pub restic_snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub files_processed: Option<i64>,
    pub bytes_processed: Option<i64>,
    pub bytes_added: Option<i64>,
    pub warnings: Vec<String>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
    pub items: Vec<BackupRunItem>,
}

#[derive(Debug, Clone)]
pub struct BackupRunItem {
    pub id: Uuid,
    pub backup_run_id: Uuid,
    pub platform_id: Uuid,
    pub volume_name: String,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
    pub status: String,
    pub restic_snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub files_processed: Option<i64>,
    pub bytes_processed: Option<i64>,
    pub bytes_added: Option<i64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackupLog {
    pub stream: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct BackupClaim {
    pub policy: BackupPolicy,
    pub repository: BackupRepository,
    pub run: BackupRun,
}

#[derive(Debug, Clone)]
pub struct BackupRunItemResult {
    pub id: Uuid,
    pub status: &'static str,
    pub restic_snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub files_processed: Option<i64>,
    pub bytes_processed: Option<i64>,
    pub bytes_added: Option<i64>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackupExecutionResult {
    pub status: &'static str,
    pub snapshot_availability: &'static str,
    pub restic_snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub files_processed: Option<i64>,
    pub bytes_processed: Option<i64>,
    pub bytes_added: Option<i64>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
    pub logs: Vec<BackupLog>,
    pub items: Vec<BackupRunItemResult>,
}
