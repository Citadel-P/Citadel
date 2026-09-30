use crate::api::resources::builds::spec::*;
use citadel_primitives::WebhookConfig;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildProjectInput {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub git_repository_id: Uuid,
    pub branch: Option<String>,
    pub context_path: Option<String>,
    pub dockerfile_path: Option<String>,
    pub target: Option<String>,
    pub build_args: Option<Vec<BuildArgSpec>>,
    pub build_secrets: Option<Vec<BuildSecretSpec>>,
    pub platform_id: Option<Uuid>,
    pub registry_id: Uuid,
    pub image_repository: String,
    pub tag_templates: Option<Vec<String>>,
    #[schema(value_type = Option<crate::api::resources::schema_models::primitives::WebhookConfigSchema>)]
    pub webhook: Option<WebhookConfig>,
    pub timeout_seconds: Option<i32>,
    pub retention_run_count: Option<i32>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
    #[serde(default)]
    pub builder_kind: BuildProjectBuilderKind,
    pub build_agent_pool_id: Option<Uuid>,
}

impl TryFrom<citadel_builds::BuildProjectConfiguration> for BuildProjectInput {
    type Error = serde_json::Error;
    fn try_from(value: citadel_builds::BuildProjectConfiguration) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            description: value.description,
            enabled: value.enabled,
            git_repository_id: value.git_repository_id,
            branch: value.branch,
            context_path: value.context_path,
            dockerfile_path: value.dockerfile_path,
            target: value.target,
            build_args: value
                .build_args
                .map(|value| value.into_iter().map(|value| value.into()).collect()),
            build_secrets: value
                .build_secrets
                .map(|value| value.into_iter().map(|value| value.into()).collect()),
            platform_id: value.platform_id,
            registry_id: value.registry_id,
            image_repository: value.image_repository,
            tag_templates: value.tag_templates,
            webhook: value.webhook,
            timeout_seconds: value.timeout_seconds,
            retention_run_count: value.retention_run_count,
            tag_ids: value.tag_ids,
            builder_kind: serde_json::from_value(value.builder_kind.into())?,
            build_agent_pool_id: value.build_agent_pool_id,
        })
    }
}

impl From<BuildProjectInput> for citadel_builds::BuildProjectConfiguration {
    fn from(value: BuildProjectInput) -> Self {
        Self {
            name: value.name,
            description: value.description,
            enabled: value.enabled,
            git_repository_id: value.git_repository_id,
            branch: value.branch,
            context_path: value.context_path,
            dockerfile_path: value.dockerfile_path,
            target: value.target,
            build_args: value
                .build_args
                .map(|value| value.into_iter().map(|value| value.into()).collect()),
            build_secrets: value
                .build_secrets
                .map(|value| value.into_iter().map(|value| value.into()).collect()),
            platform_id: value.platform_id,
            registry_id: value.registry_id,
            image_repository: value.image_repository,
            tag_templates: value.tag_templates,
            webhook: value.webhook,
            timeout_seconds: value.timeout_seconds,
            retention_run_count: value.retention_run_count,
            tag_ids: value.tag_ids,
            builder_kind: value.builder_kind.as_str().to_owned(),
            build_agent_pool_id: value.build_agent_pool_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildAgentPoolInput {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub provider_spec: BuildAgentPoolProviderSpec,
    pub max_active_builders: Option<i32>,
    pub queue_timeout_seconds: Option<i32>,
    pub provisioning_timeout_seconds: Option<i32>,
    pub registration_timeout_seconds: Option<i32>,
    pub heartbeat_timeout_seconds: Option<i32>,
    pub cleanup_timeout_seconds: Option<i32>,
    pub maximum_instance_lifetime_seconds: Option<i32>,
    pub failure_retention_minutes: Option<i32>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl TryFrom<citadel_builds::BuildAgentPoolConfiguration> for BuildAgentPoolInput {
    type Error = serde_json::Error;
    fn try_from(value: citadel_builds::BuildAgentPoolConfiguration) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            description: value.description,
            enabled: value.enabled,
            provider_spec: serde_json::from_value(value.provider_spec)?,
            max_active_builders: value.max_active_builders,
            queue_timeout_seconds: value.queue_timeout_seconds,
            provisioning_timeout_seconds: value.provisioning_timeout_seconds,
            registration_timeout_seconds: value.registration_timeout_seconds,
            heartbeat_timeout_seconds: value.heartbeat_timeout_seconds,
            cleanup_timeout_seconds: value.cleanup_timeout_seconds,
            maximum_instance_lifetime_seconds: value.maximum_instance_lifetime_seconds,
            failure_retention_minutes: value.failure_retention_minutes,
            tag_ids: value.tag_ids,
        })
    }
}

impl From<BuildAgentPoolInput> for citadel_builds::BuildAgentPoolConfiguration {
    fn from(value: BuildAgentPoolInput) -> Self {
        Self {
            name: value.name,
            description: value.description,
            enabled: value.enabled,
            provider_spec: serde_json::json!(value.provider_spec),
            max_active_builders: value.max_active_builders,
            queue_timeout_seconds: value.queue_timeout_seconds,
            provisioning_timeout_seconds: value.provisioning_timeout_seconds,
            registration_timeout_seconds: value.registration_timeout_seconds,
            heartbeat_timeout_seconds: value.heartbeat_timeout_seconds,
            cleanup_timeout_seconds: value.cleanup_timeout_seconds,
            maximum_instance_lifetime_seconds: value.maximum_instance_lifetime_seconds,
            failure_retention_minutes: value.failure_retention_minutes,
            tag_ids: value.tag_ids,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct RenamePool {
    pub(crate) id: Uuid,
    pub(crate) name: String,
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
#[schema(as = server::builds_http::QueueInput)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueueInput {
    pub(crate) trigger: Option<QueueBuildTrigger>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunFilter {
    pub(crate) project_id: Option<Uuid>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, utoipa::ToSchema)]
pub enum QueueBuildTrigger {
    #[default]
    Manual,
    Webhook,
    Dependency,
}
impl QueueBuildTrigger {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::Webhook => "Webhook",
            Self::Dependency => "Dependency",
        }
    }
}
