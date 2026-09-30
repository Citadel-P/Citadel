use chrono::{DateTime, Utc};
use citadel_primitives::AutoUpdateState;

use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwarmServiceOperation {
    pub id: Uuid,
    pub kind: SwarmServiceOperationKind,
    pub state: SwarmServiceOperationState,
    pub prepared_at: DateTime<Utc>,
    pub attempted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result_code: Option<String>,
    pub warnings: Vec<String>,
    pub result_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwarmService {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub docker_name: String,
    pub docker_service_id: Option<String>,
    pub spec: SwarmServiceSpec,
    pub health: SwarmServiceHealth,
    pub synchronization_state: SwarmServiceSynchronizationState,
    pub control_state: citadel_primitives::ResourceControlState,
    pub auto_update_state: AutoUpdateState,
    pub applied_image_digest: Option<String>,
    pub has_pending_desired_changes: bool,
    pub has_runtime_drift: bool,
    pub row_version: i64,
    pub audit: citadel_primitives::AuditMetadata,
    pub updated_at: DateTime<Utc>,

    // Related data populated by full resource reads.
    pub platform_name: Option<String>,
    pub platform_status: citadel_primitives::PlatformStatus,
    pub running_task_count: Option<i32>,
    pub desired_task_count: Option<i32>,
    pub update_state: Option<String>,
    pub update_message: Option<String>,
    pub current_operation: Option<SwarmServiceOperation>,
    pub tags: Vec<citadel_tags::TagSummary>,
    pub tasks: Option<Vec<serde_json::Value>>,
}
