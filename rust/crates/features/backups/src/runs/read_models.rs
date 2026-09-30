use chrono::{DateTime, Utc};

use uuid::Uuid;

#[derive(Debug)]
pub struct PlatformBackupSummary {
    pub platform_id: Uuid,
    pub policy_count: i32,
    pub enabled_policy_count: i32,
    pub docker_volume_policy_count: i32,
    pub stack_policy_count: i32,
    pub deployment_policy_count: i32,
    pub swarm_service_policy_count: i32,
    pub attention_policy_count: i32,
    pub last_run_status: Option<crate::BackupRunStatus>,
    pub last_run_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct VolumeBackupCoverage {
    pub platform_id: Uuid,
    pub volume_name: String,
    pub docker_node_id: Option<String>,
    pub status: crate::BackupCoverageStatus,
    pub policy_count: i32,
    pub last_run_id: Option<Uuid>,
    pub last_run_status: Option<crate::BackupRunStatus>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_successful_run_at: Option<DateTime<Utc>>,
}
