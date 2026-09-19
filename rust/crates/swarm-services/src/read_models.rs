//! Transport-neutral read results and supporting filters.
//! Repository implementations populate these types; server adapters map them to HTTP views.

use serde_json::Value;

use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SwarmServiceDuplicateDraft {
    pub name: String,
    pub source_name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    pub tag_ids: Vec<Uuid>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SwarmServiceFilter {
    pub tags: Vec<String>,
    pub platform_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}
#[derive(Debug, Clone, PartialEq)]
pub struct SwarmServiceDetails {
    pub service: SwarmService,
    pub platform_name: Option<String>,
    pub platform_status: String,
    pub running_task_count: Option<i32>,
    pub desired_task_count: Option<i32>,
    pub update_state: Option<String>,
    pub update_message: Option<String>,
    pub current_operation: Option<SwarmServiceOperation>,
    pub tags: Vec<TagSummary>,
    pub tasks: Option<Vec<Value>>,
    pub effective_permission: citadel_domain::EffectivePermission,
}

impl std::ops::Deref for SwarmServiceDetails {
    type Target = SwarmService;
    fn deref(&self) -> &Self::Target {
        &self.service
    }
}
