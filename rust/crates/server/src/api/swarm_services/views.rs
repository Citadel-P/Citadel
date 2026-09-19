use super::spec::*;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[schema(as = swarm_services::model::ResourceCapabilities)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceCapabilities {
    pub can_view_logs: bool,
    pub can_inspect: bool,
    pub can_apply: bool,
    pub can_view_resource_bindings: bool,
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceOperationView {
    pub id: Uuid,
    pub kind: String,
    pub state: String,
    pub prepared_at: DateTime<Utc>,
    pub attempted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result_code: Option<String>,
    pub warnings: Vec<String>,
    pub result_message: Option<String>,
}
impl From<citadel_swarm_services::SwarmServiceOperation> for SwarmServiceOperationView {
    fn from(value: citadel_swarm_services::SwarmServiceOperation) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            state: value.state,
            prepared_at: value.prepared_at,
            attempted_at: value.attempted_at,
            completed_at: value.completed_at,
            result_code: value.result_code,
            warnings: value.warnings,
            result_message: value.result_message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSwarmServiceView {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub docker_name: String,
    pub docker_service_id: Option<String>,
    pub spec: SwarmServiceSpec,
    pub health: String,
    pub synchronization_state: String,
    pub control_state: String,
    pub auto_update_state: AutoUpdateState,
    pub applied_image_digest: Option<String>,
    pub has_pending_desired_changes: bool,
    pub has_runtime_drift: bool,
    pub row_version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub platform_name: Option<String>,
    pub platform_status: String,
    pub running_task_count: Option<i32>,
    pub desired_task_count: Option<i32>,
    pub update_state: Option<String>,
    pub update_message: Option<String>,
    pub current_operation: Option<SwarmServiceOperationView>,
    pub tags: Vec<TagSummary>,
    pub tasks: Option<Vec<Value>>,
    pub capabilities: Option<SwarmServiceCapabilities>,
}
impl From<citadel_swarm_services::SwarmServiceDetails> for ManagedSwarmServiceView {
    fn from(value: citadel_swarm_services::SwarmServiceDetails) -> Self {
        Self {
            id: value.service.id,
            platform_id: value.service.platform_id,
            name: value.service.name,
            description: value.service.description,
            docker_name: value.service.docker_name,
            docker_service_id: value.service.docker_service_id,
            spec: value.service.spec.into(),
            health: value.service.health,
            synchronization_state: value.service.synchronization_state,
            control_state: value.service.control_state,
            auto_update_state: value.service.auto_update_state.into(),
            applied_image_digest: value.service.applied_image_digest,
            has_pending_desired_changes: value.service.has_pending_desired_changes,
            has_runtime_drift: value.service.has_runtime_drift,
            row_version: value.service.row_version,
            created_at: value.service.created_at,
            updated_at: value.service.updated_at,
            platform_name: value.platform_name,
            platform_status: value.platform_status,
            running_task_count: value.running_task_count,
            desired_task_count: value.desired_task_count,
            update_state: value.update_state,
            update_message: value.update_message,
            current_operation: value.current_operation.map(|item| item.into()),
            tags: value.tags.into_iter().map(|item| item.into()).collect(),
            tasks: value.tasks,
            capabilities: Some(super::capabilities::capabilities(
                value.effective_permission,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSwarmServicesView {
    pub swarm_services: Vec<ManagedSwarmServiceView>,
    pub capabilities: ResourceCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceProgressItem {
    pub service_id: Uuid,
    pub operation_id: Option<Uuid>,
    pub stage: String,
    pub message: String,
    #[serde(default)]
    pub is_completed: bool,
    #[serde(default)]
    pub is_warning: bool,
    pub error_message: Option<String>,
}
impl From<citadel_swarm_services::SwarmServiceProgressItem> for SwarmServiceProgressItem {
    fn from(value: citadel_swarm_services::SwarmServiceProgressItem) -> Self {
        Self {
            service_id: value.service_id,
            operation_id: value.operation_id,
            stage: value.stage,
            message: value.message,
            is_completed: value.is_completed,
            is_warning: value.is_warning,
            error_message: value.error_message,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceAdoptionSource {
    pub docker_service_id: String,
    pub name: String,
    pub platform_id: Uuid,
    pub platform_name: String,
}
impl From<citadel_swarm_services::adoption::SwarmServiceAdoptionSource>
    for SwarmServiceAdoptionSource
{
    fn from(value: citadel_swarm_services::adoption::SwarmServiceAdoptionSource) -> Self {
        Self {
            docker_service_id: value.docker_service_id,
            name: value.name,
            platform_id: value.platform_id,
            platform_name: value.platform_name,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceAdoptionIssue {
    pub code: String,
    pub message: String,
}
impl From<citadel_swarm_services::adoption::SwarmServiceAdoptionIssue>
    for SwarmServiceAdoptionIssue
{
    fn from(value: citadel_swarm_services::adoption::SwarmServiceAdoptionIssue) -> Self {
        Self {
            code: value.code,
            message: value.message,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceAdoptionDraft {
    pub source: SwarmServiceAdoptionSource,
    pub name: String,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    pub issues: Vec<SwarmServiceAdoptionIssue>,
    pub preview_fingerprint: String,
}
impl From<citadel_swarm_services::adoption::SwarmServiceAdoptionDraft>
    for SwarmServiceAdoptionDraft
{
    fn from(value: citadel_swarm_services::adoption::SwarmServiceAdoptionDraft) -> Self {
        Self {
            source: value.source.into(),
            name: value.name,
            description: value.description,
            spec: value.spec.into(),
            issues: value.issues.into_iter().map(|item| item.into()).collect(),
            preview_fingerprint: value.preview_fingerprint,
        }
    }
}
