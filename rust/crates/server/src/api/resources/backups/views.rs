use crate::api::resources::capabilities::ResourceCapabilities;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryView {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub repository_type: String,
    pub spec: Value,
    pub password_secret_id: Uuid,
    pub status: String,
    pub control_state: String,
    pub current_run_id: Option<Uuid>,
    pub control_started_at: Option<i64>,
    pub last_pruned_at: Option<DateTime<Utc>>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}

impl From<citadel_backups::BackupRepository> for BackupRepositoryView {
    fn from(value: citadel_backups::BackupRepository) -> Self {
        Self {
            id: value.id,
            name: value.name,
            normalized_name: value.normalized_name,
            description: value.description,
            repository_type: value.repository_type,
            spec: value.spec,
            password_secret_id: value.password_secret_id,
            status: value.status,
            control_state: value.control_state,
            current_run_id: value.current_run_id,
            control_started_at: value.control_started_at,
            last_pruned_at: value.last_pruned_at,
            last_checked_at: value.last_checked_at,
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
pub struct BackupPolicyView {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub source: Value,
    pub backup_repository_id: Uuid,
    pub enabled: bool,
    pub cron: Option<String>,
    pub time_zone: Option<String>,
    pub webhook: Option<Value>,
    pub keep_last_successful: i32,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
    pub control_state: String,
    pub current_run_id: Option<Uuid>,
    pub last_scheduled_run_at: Option<DateTime<Utc>>,
    pub first_successful_run_at: Option<DateTime<Utc>>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}

impl From<citadel_backups::BackupPolicy> for BackupPolicyView {
    fn from(value: citadel_backups::BackupPolicy) -> Self {
        Self {
            id: value.id,
            name: value.name,
            normalized_name: value.normalized_name,
            description: value.description,
            source: value.source,
            backup_repository_id: value.backup_repository_id,
            enabled: value.enabled,
            cron: value.cron,
            time_zone: value.time_zone,
            webhook: value.webhook,
            keep_last_successful: value.keep_last_successful,
            timeout_seconds: value.timeout_seconds,
            alert_on_failure: value.alert_on_failure,
            run_as_actor_id: value.run_as_actor_id,
            control_state: value.control_state,
            current_run_id: value.current_run_id,
            last_scheduled_run_at: value.last_scheduled_run_at,
            first_successful_run_at: value.first_successful_run_at,
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
pub struct BackupRunView {
    pub id: Uuid,
    pub backup_policy_id: Uuid,
    pub policy_name_snapshot: String,
    pub backup_repository_id: Uuid,
    pub repository_type_snapshot: String,
    pub source_snapshot: Value,
    pub trigger: String,
    pub status: String,
    pub snapshot_availability: String,
    pub restic_snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub files_processed: Option<i64>,
    pub bytes_processed: Option<i64>,
    pub bytes_added: Option<i64>,
    pub warnings: Vec<String>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
    pub items: Vec<BackupRunItemView>,
}

impl From<citadel_backups::BackupRun> for BackupRunView {
    fn from(value: citadel_backups::BackupRun) -> Self {
        Self {
            id: value.id,
            backup_policy_id: value.backup_policy_id,
            policy_name_snapshot: value.policy_name_snapshot,
            backup_repository_id: value.backup_repository_id,
            repository_type_snapshot: value.repository_type_snapshot,
            source_snapshot: value.source_snapshot,
            trigger: value.trigger,
            status: value.status,
            snapshot_availability: value.snapshot_availability,
            restic_snapshot_id: value.restic_snapshot_id,
            parent_snapshot_id: value.parent_snapshot_id,
            files_processed: value.files_processed,
            bytes_processed: value.bytes_processed,
            bytes_added: value.bytes_added,
            warnings: value.warnings,
            queued_at: value.queued_at,
            started_at: value.started_at,
            completed_at: value.completed_at,
            exit_code: value.exit_code,
            error_code: value.error_code,
            error_message: value.error_message,
            triggered_by_actor_id: value.triggered_by_actor_id,
            items: value.items.into_iter().map(|value| value.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRunItemView {
    pub id: Uuid,
    pub backup_run_id: Uuid,
    pub platform_id: Uuid,
    pub volume_name: String,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
    pub status: String,
    pub restic_snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub files_processed: Option<i64>,
    pub bytes_processed: Option<i64>,
    pub bytes_added: Option<i64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl From<citadel_backups::BackupRunItem> for BackupRunItemView {
    fn from(value: citadel_backups::BackupRunItem) -> Self {
        Self {
            id: value.id,
            backup_run_id: value.backup_run_id,
            platform_id: value.platform_id,
            volume_name: value.volume_name,
            docker_node_id: value.docker_node_id,
            node_hostname: value.node_hostname,
            status: value.status,
            restic_snapshot_id: value.restic_snapshot_id,
            parent_snapshot_id: value.parent_snapshot_id,
            files_processed: value.files_processed,
            bytes_processed: value.bytes_processed,
            bytes_added: value.bytes_added,
            started_at: value.started_at,
            completed_at: value.completed_at,
            exit_code: value.exit_code,
            error_code: value.error_code,
            error_message: value.error_message,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreRunView {
    pub id: Uuid,
    pub backup_run_id: Uuid,
    pub backup_repository_id: Uuid,
    pub source_backup_run_item_id: Option<Uuid>,
    pub target_platform_id: Uuid,
    pub target_docker_node_id: Option<String>,
    pub target_volume_name: String,
    pub overwrite_existing: bool,
    pub status: String,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
}

impl From<citadel_backups::BackupRestoreRun> for BackupRestoreRunView {
    fn from(value: citadel_backups::BackupRestoreRun) -> Self {
        Self {
            id: value.id,
            backup_run_id: value.backup_run_id,
            backup_repository_id: value.backup_repository_id,
            source_backup_run_item_id: value.source_backup_run_item_id,
            target_platform_id: value.target_platform_id,
            target_docker_node_id: value.target_docker_node_id,
            target_volume_name: value.target_volume_name,
            overwrite_existing: value.overwrite_existing,
            status: value.status,
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
pub struct BackupRepositoryValidationView {
    pub id: Uuid,
    pub backup_repository_id: Uuid,
    pub location: String,
    pub platform_id: Option<Uuid>,
    pub status: String,
    pub last_validated_at: DateTime<Utc>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}

impl From<citadel_backups::BackupRepositoryValidation> for BackupRepositoryValidationView {
    fn from(value: citadel_backups::BackupRepositoryValidation) -> Self {
        Self {
            id: value.id,
            backup_repository_id: value.backup_repository_id,
            location: value.location,
            platform_id: value.platform_id,
            status: value.status,
            last_validated_at: value.last_validated_at,
            last_error_code: value.last_error_code,
            last_error_message: value.last_error_message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupLog {
    pub stream: String,
    pub message: String,
}

impl From<citadel_backups::BackupLog> for BackupLog {
    fn from(value: citadel_backups::BackupLog) -> Self {
        Self {
            stream: value.stream,
            message: value.message,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BackupPreviewResource {
    Deployment {
        #[serde(rename = "deploymentId")]
        id: Uuid,
        #[serde(rename = "deploymentName")]
        name: String,
    },
    Stack {
        #[serde(rename = "stackId")]
        id: Uuid,
        #[serde(rename = "stackName")]
        name: String,
    },
    SwarmService {
        #[serde(rename = "swarmServiceId")]
        id: Uuid,
        #[serde(rename = "swarmServiceName")]
        name: String,
    },
}

impl From<citadel_backups::policies::read_models::BackupPreviewResource> for BackupPreviewResource {
    fn from(value: citadel_backups::policies::read_models::BackupPreviewResource) -> Self {
        match value {
            citadel_backups::policies::read_models::BackupPreviewResource::Deployment {
                id,
                name,
            } => Self::Deployment { id, name },
            citadel_backups::policies::read_models::BackupPreviewResource::Stack { id, name } => {
                Self::Stack { id, name }
            }
            citadel_backups::policies::read_models::BackupPreviewResource::SwarmService {
                id,
                name,
            } => Self::SwarmService { id, name },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSourcePreview {
    #[serde(flatten)]
    pub resource: BackupPreviewResource,
    pub platform_id: Uuid,
    pub platform_name: String,
    pub platform_status: String,
    pub volumes: Vec<BackupVolumePreview>,
    pub warnings: Vec<String>,
}

impl From<citadel_backups::policies::read_models::BackupSourcePreview> for BackupSourcePreview {
    fn from(value: citadel_backups::policies::read_models::BackupSourcePreview) -> Self {
        Self {
            resource: value.resource.into(),
            platform_id: value.platform_id,
            platform_name: value.platform_name,
            platform_status: value.platform_status,
            volumes: value
                .volumes
                .into_iter()
                .map(|value| value.into())
                .collect(),
            warnings: value.warnings,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupVolumePreview {
    pub name: String,
    pub kind: String,
    pub is_external: bool,
    pub is_shared: bool,
    pub has_backup_coverage: bool,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
}

impl From<citadel_backups::policies::read_models::BackupVolumePreview> for BackupVolumePreview {
    fn from(value: citadel_backups::policies::read_models::BackupVolumePreview) -> Self {
        Self {
            name: value.name,
            kind: value.kind,
            is_external: value.is_external,
            is_shared: value.is_shared,
            has_backup_coverage: value.has_backup_coverage,
            docker_node_id: value.docker_node_id,
            node_hostname: value.node_hostname,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformBackupSummary {
    pub platform_id: Uuid,
    pub policy_count: i32,
    pub enabled_policy_count: i32,
    pub docker_volume_policy_count: i32,
    pub stack_policy_count: i32,
    pub deployment_policy_count: i32,
    pub swarm_service_policy_count: i32,
    pub attention_policy_count: i32,
    pub last_run_status: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
}

impl From<citadel_backups::runs::read_models::PlatformBackupSummary> for PlatformBackupSummary {
    fn from(value: citadel_backups::runs::read_models::PlatformBackupSummary) -> Self {
        Self {
            platform_id: value.platform_id,
            policy_count: value.policy_count,
            enabled_policy_count: value.enabled_policy_count,
            docker_volume_policy_count: value.docker_volume_policy_count,
            stack_policy_count: value.stack_policy_count,
            deployment_policy_count: value.deployment_policy_count,
            swarm_service_policy_count: value.swarm_service_policy_count,
            attention_policy_count: value.attention_policy_count,
            last_run_status: value.last_run_status,
            last_run_at: value.last_run_at,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Repositories {
    pub(crate) repositories: Vec<BackupRepositoryView>,
    pub(crate) capabilities: ResourceCapabilities,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Restores {
    pub(crate) runs: Vec<BackupRestoreRunView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::backups_http::Runs)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Runs {
    pub(crate) runs: Vec<BackupRunView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::backups_http::Logs)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Logs {
    pub(crate) run_id: Uuid,
    pub(crate) logs: String,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::backups_http::Events)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Events {
    pub(crate) run_id: Uuid,
    pub(crate) events: Vec<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Policies {
    pub(crate) policies: Vec<BackupPolicyView>,
    pub(crate) capabilities: ResourceCapabilities,
}
