use crate::*;

#[derive(Debug, Clone)]
pub struct BackupPolicy {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub source: Value,
    pub backup_repository_id: Uuid,
    pub enabled: bool,
    pub cron: Option<String>,
    pub time_zone: Option<String>,
    pub webhook: Option<Value>,
    pub keep_last_successful: i32,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
    pub control_state: String,
    pub current_run_id: Option<Uuid>,
    pub last_scheduled_run_at: Option<DateTime<Utc>>,
    pub first_successful_run_at: Option<DateTime<Utc>>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}
