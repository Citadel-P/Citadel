use super::spec::{BuildArgSpec, BuildSecretSpec};

use chrono::{DateTime, Utc};

use serde::Serialize;

use serde_json::Value;

use uuid::Uuid;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildProjectView {
    pub tags: Vec<crate::api::tags::dto::TagSummary>,
    pub latest_run: Option<BuildRunView>,
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub git_repository_id: Uuid,
    pub branch: String,
    pub context_path: String,
    pub dockerfile_path: String,
    pub target: Option<String>,
    pub build_args: Vec<BuildArgSpec>,
    pub build_secrets: Vec<BuildSecretSpec>,
    pub builder_kind: String,
    pub platform_id: Option<Uuid>,
    pub build_agent_pool_id: Option<Uuid>,
    pub registry_id: Uuid,
    pub image_repository: String,
    pub tag_templates: Vec<String>,
    pub webhook: Option<Value>,
    pub timeout_seconds: i32,
    pub retention_run_count: i32,
    pub current_run_id: Option<Uuid>,
    pub control_state: String,
    pub control_started_at: Option<i64>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}

impl From<citadel_builds::BuildProject> for BuildProjectView {
    fn from(value: citadel_builds::BuildProject) -> Self {
        Self {
            tags: value.tags.into_iter().map(Into::into).collect(),
            latest_run: value.latest_run.map(|value| value.into()),
            id: value.id,
            name: value.name,
            normalized_name: value.normalized_name,
            description: value.description,
            enabled: value.enabled,
            git_repository_id: value.git_repository_id,
            branch: value.branch,
            context_path: value.context_path,
            dockerfile_path: value.dockerfile_path,
            target: value.target,
            build_args: value
                .build_args
                .into_iter()
                .map(|value| value.into())
                .collect(),
            build_secrets: value
                .build_secrets
                .into_iter()
                .map(|value| value.into())
                .collect(),
            builder_kind: value.builder_kind,
            platform_id: value.platform_id,
            build_agent_pool_id: value.build_agent_pool_id,
            registry_id: value.registry_id,
            image_repository: value.image_repository,
            tag_templates: value.tag_templates,
            webhook: value.webhook,
            timeout_seconds: value.timeout_seconds,
            retention_run_count: value.retention_run_count,
            current_run_id: value.current_run_id,
            control_state: value.control_state,
            control_started_at: value.control_started_at,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
            row_version: value.row_version,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildRunView {
    pub id: Uuid,
    pub build_project_id: Uuid,
    pub project_name_snapshot: String,
    pub git_repository_id: Uuid,
    pub branch: String,
    pub resolved_commit_sha: Option<String>,
    pub context_path: String,
    pub dockerfile_path: String,
    pub target: Option<String>,
    pub registry_id: Uuid,
    pub registry_host: String,
    pub image_repository: String,
    pub image_references: Vec<String>,
    pub trigger: String,
    pub status: String,
    pub image_digest: Option<String>,
    pub timeout_seconds: i32,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
}

impl From<citadel_builds::BuildRun> for BuildRunView {
    fn from(value: citadel_builds::BuildRun) -> Self {
        Self {
            id: value.id,
            build_project_id: value.build_project_id,
            project_name_snapshot: value.project_name_snapshot,
            git_repository_id: value.git_repository_id,
            branch: value.branch,
            resolved_commit_sha: value.resolved_commit_sha,
            context_path: value.context_path,
            dockerfile_path: value.dockerfile_path,
            target: value.target,
            registry_id: value.registry_id,
            registry_host: value.registry_host,
            image_repository: value.image_repository,
            image_references: value.image_references,
            trigger: value.trigger,
            status: value.status,
            image_digest: value.image_digest,
            timeout_seconds: value.timeout_seconds,
            queued_at: value.queued_at,
            started_at: value.started_at,
            completed_at: value.completed_at,
            exit_code: value.exit_code,
            error_code: value.error_code,
            error_message: value.error_message,
            triggered_by_actor_id: value.triggered_by_actor_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildAgentPoolView {
    pub tags: Vec<crate::api::tags::dto::TagSummary>,
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub provider: String,
    pub provider_spec: Value,
    pub max_active_builders: i32,
    pub queue_timeout_seconds: i32,
    pub provisioning_timeout_seconds: i32,
    pub registration_timeout_seconds: i32,
    pub heartbeat_timeout_seconds: i32,
    pub cleanup_timeout_seconds: i32,
    pub maximum_instance_lifetime_seconds: i32,
    pub failure_retention_minutes: i32,
    pub last_validation_status: String,
    pub last_validation_message: Option<String>,
    pub last_validated_at: Option<DateTime<Utc>>,
    pub control_state: String,
    pub control_triggered_by: Option<Uuid>,
    pub control_started_at: Option<i64>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}

impl From<citadel_builds::BuildAgentPool> for BuildAgentPoolView {
    fn from(value: citadel_builds::BuildAgentPool) -> Self {
        Self {
            tags: value.tags.into_iter().map(Into::into).collect(),
            id: value.id,
            name: value.name,
            normalized_name: value.normalized_name,
            description: value.description,
            enabled: value.enabled,
            provider: value.provider,
            provider_spec: value.provider_spec,
            max_active_builders: value.max_active_builders,
            queue_timeout_seconds: value.queue_timeout_seconds,
            provisioning_timeout_seconds: value.provisioning_timeout_seconds,
            registration_timeout_seconds: value.registration_timeout_seconds,
            heartbeat_timeout_seconds: value.heartbeat_timeout_seconds,
            cleanup_timeout_seconds: value.cleanup_timeout_seconds,
            maximum_instance_lifetime_seconds: value.maximum_instance_lifetime_seconds,
            failure_retention_minutes: value.failure_retention_minutes,
            last_validation_status: value.last_validation_status,
            last_validation_message: value.last_validation_message,
            last_validated_at: value.last_validated_at,
            control_state: value.control_state,
            control_triggered_by: value.control_triggered_by,
            control_started_at: value.control_started_at,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
            row_version: value.row_version,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildLog {
    pub stream: String,
    pub message: String,
}

impl From<citadel_builds::BuildLog> for BuildLog {
    fn from(value: citadel_builds::BuildLog) -> Self {
        Self {
            stream: value.stream,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildLogEntry {
    pub id: Uuid,
    pub build_run_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub stream: String,
    pub message: String,
}

impl From<citadel_builds::BuildLogEntry> for BuildLogEntry {
    fn from(value: citadel_builds::BuildLogEntry) -> Self {
        Self {
            id: value.id,
            build_run_id: value.build_run_id,
            created_at: value.created_at,
            stream: value.stream,
            message: value.message,
        }
    }
}
