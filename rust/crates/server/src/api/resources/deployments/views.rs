//! Deployment HTTP and realtime representations.
use crate::api::resources::deployments::spec::{
    AutoUpdateState, DeploymentSpec, DuplicateSourceInput, DuplicateWarning, TagSummary,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = deployments::model::ResourceCapabilities)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
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
    pub status: String,
    pub control_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_update_state: Option<AutoUpdateState>,
    pub spec: DeploymentSpec,
    pub platform_status: String,
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
    pub latest_activity_view: Option<Value>,
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
    pub spec: DeploymentSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentsView {
    pub deployments: Vec<DeploymentView>,
    pub capabilities: ResourceCapabilities,
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

impl From<citadel_deployments::DeploymentDetails> for DeploymentView {
    fn from(value: citadel_deployments::DeploymentDetails) -> Self {
        Self {
            id: value.deployment.id,
            name: value.deployment.name,
            description: value.deployment.description,
            platform_id: value.deployment.platform_id,
            created_at: value.deployment.created_at,
            created_by_actor_id: value.deployment.created_by_actor_id,
            status: value.deployment.status,
            control_state: value.deployment.control_state,
            auto_update_state: value.deployment.auto_update_state.map(Into::into),
            spec: value.deployment.spec.into(),
            platform_status: value.platform_status,
            platform_name: value.platform_name,
            image_name: value.image_name,
            image_id: value.image_id,
            container_id: value.container_id,
            docker_container_id: value.docker_container_id,
            docker_image_id: value.docker_image_id,
            tags: value.tags.into_iter().map(Into::into).collect(),
            latest_activity_view:
                crate::api::resources::activities::presentation::public_latest_activity(
                    value.latest_activity,
                ),
            capabilities: Some(
                crate::api::resources::deployments::capabilities::capabilities(
                    value.effective_permission,
                ),
            ),
        }
    }
}

impl From<citadel_deployments::DeploymentConfig> for DeploymentConfigView {
    fn from(value: citadel_deployments::DeploymentConfig) -> Self {
        Self {
            id: value.id,
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            spec: value.spec.into(),
        }
    }
}

impl From<citadel_deployments::DeploymentDraft> for CreateDeploymentInputView {
    fn from(value: citadel_deployments::DeploymentDraft) -> Self {
        Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            spec: value.spec.into(),
            tag_ids: value.tag_ids,
            duplicate_source: value.duplicate_source.into(),
        }
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

impl From<citadel_deployments::DeploymentDuplicateDraft> for DeploymentDuplicateDraftView {
    fn from(value: citadel_deployments::DeploymentDuplicateDraft) -> Self {
        Self {
            draft: value.draft.into(),
            warnings: value.warnings.into_iter().map(Into::into).collect(),
        }
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
            status: "Created".to_owned(),
            control_state: "Idle".to_owned(),
            auto_update_state: Some(AutoUpdateState {
                last_checked_at: Utc::now(),
                status: "Unknown".to_owned(),
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
            platform_status: "Online".to_owned(),
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
