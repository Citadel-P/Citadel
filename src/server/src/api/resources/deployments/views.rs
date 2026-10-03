//! Deployment HTTP and realtime representations.
use crate::api::resources::capabilities::ResourceCapabilitiesView;
use crate::api::resources::deployments::spec::{DeploymentSpec, DuplicateWarning};
use crate::api::resources::tags::views::TagSummary;
use crate::api::resources::{
    activities::views::LatestActivityView, common::DuplicateSourceInput,
    deployments::spec::DeploymentStatus,
};
use chrono::{DateTime, Utc};
use citadel_primitives::AuthorizedResource;
use citadel_primitives::AutoUpdateState;
use citadel_primitives::PlatformStatus;
use citadel_primitives::ResourceControlState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentCapabilities {
    pub can_view_logs: bool,
    pub can_inspect: bool,
    pub can_open_terminal: bool,
    pub can_pull: bool,
    pub can_apply: bool,
    pub can_view_resource_bindings: bool,
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentView {
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub platform_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    #[schema(value_type = crate::api::resources::schema_models::deployments::DeploymentStatusSchema)]
    pub status: DeploymentStatus,
    #[schema(value_type = crate::api::resources::schema_models::primitives::ResourceControlStateSchema)]
    pub control_state: ResourceControlState,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<crate::api::resources::schema_models::primitives::AutoUpdateStateSchema>)]
    pub auto_update_state: Option<AutoUpdateState>,
    #[schema(value_type = crate::api::resources::schema_models::deployments::DeploymentSpecSchema)]
    pub spec: DeploymentSpec,
    #[schema(value_type = crate::api::resources::schema_models::primitives::PlatformStatusSchema)]
    pub platform_status: PlatformStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_image_id: Option<String>,
    pub tags: Vec<TagSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_activity_view: Option<LatestActivityView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<DeploymentCapabilities>,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentConfigView {
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[schema(value_type = crate::api::resources::schema_models::deployments::DeploymentSpecSchema)]
    pub spec: DeploymentSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentsView {
    pub deployments: Vec<DeploymentView>,
    pub capabilities: ResourceCapabilitiesView,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentDuplicateDraftView {
    pub draft: CreateDeploymentInputView,
    pub warnings: Vec<DuplicateWarning>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeploymentInputView {
    pub name: String,
    pub platform_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[schema(value_type = crate::api::resources::schema_models::deployments::DeploymentSpecSchema)]
    pub spec: DeploymentSpec,
    pub tag_ids: Vec<Uuid>,
    pub duplicate_source: DuplicateSourceInput,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentStreamItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ImagePullProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DeploymentApplyError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentApplyError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[schema(as = deployments::model::ImagePullProgress)]
#[serde(rename_all = "camelCase")]
pub struct ImagePullProgress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
}

impl TryFrom<AuthorizedResource<citadel_deployments::Deployment>> for DeploymentView {
    type Error = serde_json::Error;

    fn try_from(
        value: AuthorizedResource<citadel_deployments::Deployment>,
    ) -> Result<Self, Self::Error> {
        let permission = value.effective_permission;
        let value = value.resource;
        Ok(Self {
            id: value.id,
            name: value.name,
            description: value.description,
            platform_id: value.platform_id,
            created_at: value.audit.created_at,
            created_by_actor_id: value.audit.created_by_actor_id.value(),
            status: value.status,
            control_state: value.control_state,
            auto_update_state: value.auto_update_state,
            spec: value.spec,
            platform_status: value.platform_status,
            platform_name: value.platform_name,
            image_name: value.image_name,
            image_id: value.image_id,
            container_id: value.container_id,
            docker_container_id: value.docker_container_id,
            docker_image_id: value.docker_image_id,
            tags: value.tags.into_iter().map(Into::into).collect(),
            latest_activity_view: value.latest_activity.map(LatestActivityView::from),
            capabilities: Some(
                crate::api::resources::deployments::capabilities::capabilities(permission),
            ),
        })
    }
}

impl From<citadel_deployments::DeploymentConfig> for DeploymentConfigView {
    fn from(value: citadel_deployments::DeploymentConfig) -> Self {
        Self {
            id: value.id,
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            spec: value.spec,
        }
    }
}

impl TryFrom<citadel_deployments::DeploymentDraft> for CreateDeploymentInputView {
    type Error = serde_json::Error;

    fn try_from(value: citadel_deployments::DeploymentDraft) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            spec: value.spec,
            tag_ids: value.tag_ids,
            duplicate_source: value.duplicate_source.try_into()?,
        })
    }
}

impl From<citadel_deployments::DeploymentApplyError> for DeploymentApplyError {
    fn from(value: citadel_deployments::DeploymentApplyError) -> Self {
        Self {
            code: value.code,
            message: value.message,
        }
    }
}

impl From<citadel_deployments::ImagePullProgress> for ImagePullProgress {
    fn from(value: citadel_deployments::ImagePullProgress) -> Self {
        Self {
            current: value.current,
            total: value.total,
            start: value.start,
            units: value.units,
        }
    }
}

impl TryFrom<citadel_deployments::DeploymentDuplicateDraft> for DeploymentDuplicateDraftView {
    type Error = serde_json::Error;

    fn try_from(value: citadel_deployments::DeploymentDuplicateDraft) -> Result<Self, Self::Error> {
        Ok(Self {
            draft: value.draft.try_into()?,
            warnings: value.warnings.into_iter().map(Into::into).collect(),
        })
    }
}

impl From<citadel_deployments::DeploymentProgress> for DeploymentStreamItem {
    fn from(value: citadel_deployments::DeploymentProgress) -> Self {
        Self {
            id: value.id,
            status: value.status,
            stream: value.stream,
            progress_message: value.progress_message,
            error_message: value.error_message,
            progress: value.progress.map(Into::into),
            error: value.error.map(Into::into),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::api::resources::deployments::{
        spec::{DeploymentImageInfo, UpdateBehavior},
        views::*,
    };

    #[test]
    fn api_serialization_omits_null_properties_like_the_dotnet_contract() {
        let view = DeploymentView {
            id: Uuid::now_v7(),
            name: "web".to_owned(),
            description: None,
            platform_id: Uuid::now_v7(),
            created_at: Utc::now(),
            created_by_actor_id: Uuid::now_v7(),
            status: DeploymentStatus::Created,
            control_state: ResourceControlState::Idle,
            auto_update_state: Some(AutoUpdateState {
                last_checked_at: Utc::now(),
                status: citadel_primitives::AutoUpdateStatus::Unknown,
                current_digest: None,
                remote_digest: None,
                last_error: None,
            }),
            spec: DeploymentSpec {
                image: DeploymentImageInfo::External {
                    registry_id: Uuid::from_u128(0x100),
                    image_tag: "nginx:latest".to_owned(),
                    resolved_digest: None,
                },
                update_behavior: UpdateBehavior::Notify,
                life_cycle_spec: None,
                resource_spec: None,
                labels: None,
                ports: None,
                volumes: None,
                networks: None,
                command: None,
                environment_variables: None,
            },
            platform_status: PlatformStatus::Online,
            platform_name: None,
            image_name: None,
            image_id: None,
            container_id: None,
            docker_container_id: None,
            docker_image_id: None,
            tags: Vec::new(),
            latest_activity_view: None,
            capabilities: None,
        };

        let value = serde_json::to_value(view).unwrap();

        assert_eq!(value["spec"]["image"]["$type"], "External");
        assert!(value.get("description").is_none());
        assert!(value.get("rowVersion").is_none());
        assert!(value.get("platformName").is_none());
        assert!(value["spec"].get("lifeCycleSpec").is_none());
        assert!(value["spec"]["image"].get("resolvedDigest").is_none());
        assert!(value["autoUpdateState"].get("lastError").is_none());
    }
}
