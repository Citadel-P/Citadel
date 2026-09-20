use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AutomationRun {
    pub id: Uuid,
    pub action_id: Uuid,
    pub action_name: String,
    pub trigger: String,
    pub status: String,
    pub run_as_actor_id: Uuid,
    pub triggered_by_actor_id: Option<Uuid>,
    pub args_json: String,
    pub code_snapshot: Option<String>,
    pub code_hash: String,
    pub timeout_seconds: i32,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub logs: Option<String>,
    pub error_message: Option<String>,
}
