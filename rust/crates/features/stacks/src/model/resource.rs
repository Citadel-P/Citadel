use chrono::{DateTime, Utc};

use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackRelease {
    pub id: Uuid,
    pub stack_id: Uuid,
    pub platform_id: Uuid,
    pub status: StackReleaseStatus,
    pub version: String,
    pub spec: StackSpec,
    pub source: Option<StackReleaseSource>,
    pub resource_bindings: Option<Vec<ResourceBindingSnapshot>>,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,

    // Related data populated by full resource reads.
    pub actor_name: String,
    pub actor_type: String,
    pub platform_status: citadel_primitives::PlatformStatus,
    pub platform_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stack {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub stack_update_state: StackUpdateState,
    pub drift_policy: StackDriftPolicy,
    pub control_state: citadel_primitives::ResourceControlState,
    pub current_stack_release_id: Uuid,
    pub row_version: i64,
    pub audit: citadel_primitives::AuditMetadata,

    // Related data populated by full resource reads.
    pub status: StackReleaseStatus,
    pub platform_type: citadel_platforms::PlatformKind,
    pub platform_id: Option<Uuid>,
    pub version: Option<String>,
    pub spec: Option<StackSpec>,
    pub source: Option<StackReleaseSource>,
    pub resource_bindings: Option<Vec<ResourceBindingSnapshot>>,
    pub platform_status: citadel_primitives::PlatformStatus,
    pub platform_name: Option<String>,
    pub tags: Vec<citadel_tags::TagSummary>,
    pub latest_activity: Option<citadel_activities::ActivitySummary>,
}
