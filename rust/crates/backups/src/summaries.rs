use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformBackupSummary {
    pub platform_id: Uuid,
    pub policy_count: i32,
    pub enabled_policy_count: i32,
    pub docker_volume_policy_count: i32,
    pub stack_policy_count: i32,
    pub deployment_policy_count: i32,
    pub swarm_service_policy_count: i32,
    pub attention_policy_count: i32,
    pub last_run_status: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
}
