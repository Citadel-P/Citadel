use crate::api::resources::swarm_services::spec::*;
use crate::api::resources::tags::views::TagSummary;
use crate::api::resources::{
    common::{DuplicateSourceInput, PlatformStatus, ResourceControlState},
    swarm_services::requests::CreateSwarmServiceInput,
};
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
    pub kind: SwarmServiceOperationKind,
    pub state: SwarmServiceOperationState,
    pub prepared_at: DateTime<Utc>,
    #[schema(required = true)]
    pub attempted_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub completed_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub result_code: Option<String>,
    pub warnings: Vec<String>,
    #[schema(required = true)]
    pub result_message: Option<String>,
}

impl TryFrom<citadel_swarm_services::SwarmServiceOperation> for SwarmServiceOperationView {
    type Error = serde_json::Error;

    fn try_from(value: citadel_swarm_services::SwarmServiceOperation) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            kind: serde_json::from_value(value.kind.into())?,
            state: serde_json::from_value(value.state.into())?,
            prepared_at: value.prepared_at,
            attempted_at: value.attempted_at,
            completed_at: value.completed_at,
            result_code: value.result_code,
            warnings: value.warnings,
            result_message: value.result_message,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSwarmServiceView {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    #[schema(required = true)]
    pub description: Option<String>,
    pub docker_name: String,
    #[schema(required = true)]
    pub docker_service_id: Option<String>,
    pub spec: SwarmServiceSpec,
    pub health: SwarmServiceHealth,
    pub synchronization_state: SwarmServiceSynchronizationState,
    pub control_state: ResourceControlState,
    pub auto_update_state: AutoUpdateState,
    #[schema(required = true)]
    pub applied_image_digest: Option<String>,
    pub has_pending_desired_changes: bool,
    pub has_runtime_drift: bool,
    pub row_version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[schema(required = true)]
    pub platform_name: Option<String>,
    pub platform_status: PlatformStatus,
    #[schema(required = true)]
    pub running_task_count: Option<i32>,
    #[schema(required = true)]
    pub desired_task_count: Option<i32>,
    #[schema(required = true)]
    pub update_state: Option<String>,
    #[schema(required = true)]
    pub update_message: Option<String>,
    #[schema(required = true)]
    pub current_operation: Option<SwarmServiceOperationView>,
    pub tags: Vec<TagSummary>,
    #[schema(required = true)]
    pub tasks: Option<Vec<Value>>,
    #[schema(required = true)]
    pub capabilities: Option<SwarmServiceCapabilities>,
}

impl TryFrom<citadel_swarm_services::SwarmServiceDetails> for ManagedSwarmServiceView {
    type Error = serde_json::Error;

    fn try_from(value: citadel_swarm_services::SwarmServiceDetails) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.service.id,
            platform_id: value.service.platform_id,
            name: value.service.name,
            description: value.service.description,
            docker_name: value.service.docker_name,
            docker_service_id: value.service.docker_service_id,
            spec: value.service.spec.try_into()?,
            health: serde_json::from_value(value.service.health.into())?,
            synchronization_state: serde_json::from_value(
                value.service.synchronization_state.into(),
            )?,
            control_state: serde_json::from_value(value.service.control_state.into())?,
            auto_update_state: value.service.auto_update_state.try_into()?,
            applied_image_digest: value.service.applied_image_digest,
            has_pending_desired_changes: value.service.has_pending_desired_changes,
            has_runtime_drift: value.service.has_runtime_drift,
            row_version: value.service.row_version,
            created_at: value.service.created_at,
            updated_at: value.service.updated_at,
            platform_name: value.platform_name,
            platform_status: serde_json::from_value(value.platform_status.into())?,
            running_task_count: value.running_task_count,
            desired_task_count: value.desired_task_count,
            update_state: value.update_state,
            update_message: value.update_message,
            current_operation: value.current_operation.map(TryInto::try_into).transpose()?,
            tags: value.tags.into_iter().map(|item| item.into()).collect(),
            tasks: value.tasks,
            capabilities: Some(
                crate::api::resources::swarm_services::capabilities::capabilities(
                    value.effective_permission,
                ),
            ),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSwarmServicesView {
    pub swarm_services: Vec<ManagedSwarmServiceView>,
    pub capabilities: ResourceCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceProgressItem {
    pub service_id: Uuid,
    #[schema(required = true)]
    pub operation_id: Option<Uuid>,
    pub stage: String,
    pub message: String,
    #[serde(default)]
    pub is_completed: bool,
    #[serde(default)]
    pub is_warning: bool,
    #[schema(required = true)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceAdoptionDraftView {
    pub source: SwarmServiceAdoptionSource,
    pub draft: CreateSwarmServiceInput,
    pub issues: Vec<SwarmServiceAdoptionIssue>,
    pub preview_fingerprint: String,
}

impl TryFrom<citadel_swarm_services::adoption::SwarmServiceAdoptionDraft>
    for SwarmServiceAdoptionDraftView
{
    type Error = serde_json::Error;
    fn try_from(
        value: citadel_swarm_services::adoption::SwarmServiceAdoptionDraft,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            draft: CreateSwarmServiceInput {
                name: value.name,
                platform_id: value.source.platform_id,
                description: value.description,
                spec: value.spec.try_into()?,
                tag_ids: vec![],
                duplicate_source: None,
            },
            source: value.source.into(),
            issues: value.issues.into_iter().map(Into::into).collect(),
            preview_fingerprint: value.preview_fingerprint,
        })
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceDuplicateDraftView {
    pub draft: CreateSwarmServiceInput,
    pub warnings: Vec<String>,
}

impl SwarmServiceDuplicateDraftView {
    pub fn from_draft(
        value: citadel_swarm_services::SwarmServiceDuplicateDraft,
        source_id: Uuid,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            draft: CreateSwarmServiceInput {
                name: value.name,
                platform_id: value.platform_id,
                description: value.description,
                spec: value.spec.try_into()?,
                tag_ids: value.tag_ids,
                duplicate_source: Some(DuplicateSourceInput {
                    resource_type: citadel_activities::ActivityResourceType::SwarmService,
                    resource_id: source_id,
                    resource_name: value.source_name,
                }),
            },
            warnings: value.warnings,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize, utoipa::ToSchema)]
pub enum SwarmServiceHealth {
    Unknown,
    Healthy,
    Progressing,
    Degraded,
    Failed,
    Created,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize, utoipa::ToSchema)]
pub enum SwarmServiceSynchronizationState {
    NeverApplied,
    DesiredChangesPending,
    InSync,
    Drifted,
    RuntimeMissing,
    OutcomeUnknown,
    OwnershipConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize, utoipa::ToSchema)]
pub enum SwarmServiceOperationKind {
    Apply,
    Scale,
    ForceUpdate,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize, utoipa::ToSchema)]
pub enum SwarmServiceOperationState {
    Prepared,
    Canceled,
    PendingAcceptance,
    Accepted,
    Rejected,
    NotAccepted,
    OutcomeUnknown,
    Completed,
    OwnershipConflict,
}
