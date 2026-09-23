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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stack {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub stack_update_state: StackUpdateState,
    pub drift_policy: StackDriftPolicy,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    pub control_state: String,
    pub current_stack_release_id: Uuid,
    pub row_version: i64,
}
