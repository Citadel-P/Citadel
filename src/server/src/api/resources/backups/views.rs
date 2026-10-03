use super::spec::*;
use crate::api::resources::capabilities::ResourceCapabilitiesView;
use chrono::{DateTime, Utc};
use citadel_primitives::PlatformStatus;
use citadel_primitives::ResourceControlState;
use citadel_primitives::WebhookConfig;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryView {
    #[schema(required = true)]
    pub capabilities: Option<crate::api::resources::capabilities::ResourceCapabilitiesView>,
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    #[schema(required = true)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub repository_type: BackupRepositoryType,
    #[schema(value_type = crate::api::resources::schema_models::backups::BackupRepositorySpecSchema)]
    pub spec: BackupRepositorySpec,
    pub password_secret_id: Uuid,
    #[schema(value_type = crate::api::resources::schema_models::backups::BackupRepositoryStatusSchema)]
    pub status: BackupRepositoryStatus,
    #[schema(value_type = crate::api::resources::schema_models::primitives::ResourceControlStateSchema)]
    pub control_state: ResourceControlState,
    #[schema(required = true)]
    pub current_run_id: Option<Uuid>,
    #[schema(required = true)]
    pub control_started_at: Option<i64>,
    #[schema(required = true)]
    pub last_pruned_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[schema(required = true)]
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}

impl TryFrom<citadel_backups::BackupRepository> for BackupRepositoryView {
    type Error = serde_json::Error;
    fn try_from(value: citadel_backups::BackupRepository) -> Result<Self, Self::Error> {
        Ok(Self {
            capabilities: None,
            id: value.id,
            name: value.name,
            normalized_name: value.normalized_name,
            description: value.description,
            repository_type: serde_json::from_value(value.repository_type.into())?,
            spec: value.spec,
            password_secret_id: value.password_secret_id,
            status: value.status,
            control_state: value.control_state,
            current_run_id: value.current_run_id,
            control_started_at: value.control_started_at,
            last_pruned_at: value.last_pruned_at,
            last_checked_at: value.last_checked_at,
            created_by_actor_id: value.audit.created_by_actor_id.value(),
            created_at: value.audit.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
            row_version: value.row_version,
        })
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupPolicyView {
    pub tags: Vec<crate::api::resources::tags::views::TagSummary>,
    #[schema(required = true)]
    pub latest_run: Option<BackupRunView>,
    #[schema(required = true)]
    pub capabilities: Option<crate::api::resources::capabilities::ResourceCapabilitiesView>,
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    #[schema(required = true)]
    pub description: Option<String>,
    #[schema(value_type = crate::api::resources::schema_models::backups::BackupSourceSpecSchema)]
    pub source: BackupSourceSpec,
    pub backup_repository_id: Uuid,
    pub enabled: bool,
    #[schema(required = true)]
    pub cron: Option<String>,
    #[schema(required = true)]
    pub time_zone: Option<String>,
    #[schema(required = true, value_type = Option<crate::api::resources::schema_models::primitives::WebhookConfigSchema>)]
    pub webhook: Option<WebhookConfig>,
    pub keep_last_successful: i32,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
    #[schema(value_type = crate::api::resources::schema_models::primitives::ResourceControlStateSchema)]
    pub control_state: ResourceControlState,
    #[schema(required = true)]
    pub current_run_id: Option<Uuid>,
    #[schema(required = true)]
    pub last_scheduled_run_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub first_successful_run_at: Option<DateTime<Utc>>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[schema(required = true)]
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
}

impl TryFrom<citadel_backups::BackupPolicy> for BackupPolicyView {
    type Error = serde_json::Error;
    fn try_from(value: citadel_backups::BackupPolicy) -> Result<Self, Self::Error> {
        Ok(Self {
            tags: value.tags.into_iter().map(Into::into).collect(),
            latest_run: value.latest_run.map(BackupRunView::try_from).transpose()?,
            capabilities: None,
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
            created_by_actor_id: value.audit.created_by_actor_id.value(),
            created_at: value.audit.created_at,
            updated_at: value.updated_at,
            archived_at: value.archived_at,
            row_version: value.row_version,
        })
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRunView {
    pub id: Uuid,
    pub backup_policy_id: Uuid,
    pub policy_name_snapshot: String,
    pub backup_repository_id: Uuid,
    pub repository_type_snapshot: BackupRepositoryType,
    #[schema(value_type = crate::api::resources::schema_models::backups::BackupSourceSpecSchema)]
    pub source_snapshot: BackupSourceSpec,
    pub trigger: BackupRunTrigger,
    #[schema(value_type = crate::api::resources::schema_models::backups::BackupRunStatusSchema)]
    pub status: BackupRunStatus,
    #[schema(value_type = crate::api::resources::schema_models::backups::BackupSnapshotAvailabilitySchema)]
    pub snapshot_availability: BackupSnapshotAvailability,
    #[schema(required = true)]
    pub restic_snapshot_id: Option<String>,
    #[schema(required = true)]
    pub parent_snapshot_id: Option<String>,
    #[schema(required = true)]
    pub files_processed: Option<i64>,
    #[schema(required = true)]
    pub bytes_processed: Option<i64>,
    #[schema(required = true)]
    pub bytes_added: Option<i64>,
    pub warnings: Vec<String>,
    pub queued_at: DateTime<Utc>,
    #[schema(required = true)]
    pub started_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub completed_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub exit_code: Option<i32>,
    #[schema(required = true)]
    pub error_code: Option<String>,
    #[schema(required = true)]
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
    pub items: Vec<BackupRunItemView>,
}

impl TryFrom<citadel_backups::BackupRun> for BackupRunView {
    type Error = serde_json::Error;
    fn try_from(value: citadel_backups::BackupRun) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            backup_policy_id: value.backup_policy_id,
            policy_name_snapshot: value.policy_name_snapshot,
            backup_repository_id: value.backup_repository_id,
            repository_type_snapshot: serde_json::from_value(
                value.repository_type_snapshot.into(),
            )?,
            source_snapshot: value.source_snapshot,
            trigger: serde_json::from_value(value.trigger.into())?,
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
            items: value
                .items
                .into_iter()
                .map(BackupRunItemView::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRunItemView {
    pub id: Uuid,
    pub backup_run_id: Uuid,
    pub platform_id: Uuid,
    pub volume_name: String,
    #[schema(required = true)]
    pub docker_node_id: Option<String>,
    #[schema(required = true)]
    pub node_hostname: Option<String>,
    #[schema(value_type = crate::api::resources::schema_models::backups::BackupRunItemStatusSchema)]
    pub status: BackupRunItemStatus,
    #[schema(required = true)]
    pub restic_snapshot_id: Option<String>,
    #[schema(required = true)]
    pub parent_snapshot_id: Option<String>,
    #[schema(required = true)]
    pub files_processed: Option<i64>,
    #[schema(required = true)]
    pub bytes_processed: Option<i64>,
    #[schema(required = true)]
    pub bytes_added: Option<i64>,
    #[schema(required = true)]
    pub started_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub completed_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub exit_code: Option<i32>,
    #[schema(required = true)]
    pub error_code: Option<String>,
    #[schema(required = true)]
    pub error_message: Option<String>,
}

impl TryFrom<citadel_backups::BackupRunItem> for BackupRunItemView {
    type Error = serde_json::Error;
    fn try_from(value: citadel_backups::BackupRunItem) -> Result<Self, Self::Error> {
        Ok(Self {
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
        })
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreRunView {
    pub id: Uuid,
    pub backup_run_id: Uuid,
    pub backup_repository_id: Uuid,
    #[schema(required = true)]
    pub source_backup_run_item_id: Option<Uuid>,
    pub target_platform_id: Uuid,
    #[schema(required = true)]
    pub target_docker_node_id: Option<String>,
    pub target_volume_name: String,
    pub overwrite_existing: bool,
    #[schema(value_type = crate::api::resources::schema_models::backups::BackupRestoreStatusSchema)]
    pub status: BackupRestoreStatus,
    pub queued_at: DateTime<Utc>,
    #[schema(required = true)]
    pub started_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub completed_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub exit_code: Option<i32>,
    #[schema(required = true)]
    pub error_code: Option<String>,
    #[schema(required = true)]
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
}

impl TryFrom<citadel_backups::BackupRestoreRun> for BackupRestoreRunView {
    type Error = serde_json::Error;
    fn try_from(value: citadel_backups::BackupRestoreRun) -> Result<Self, Self::Error> {
        Ok(Self {
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
        })
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryValidationView {
    pub id: Uuid,
    pub backup_repository_id: Uuid,
    #[schema(value_type = crate::api::resources::schema_models::backups::BackupExecutionLocationSchema)]
    pub location: BackupExecutionLocation,
    #[schema(required = true)]
    pub platform_id: Option<Uuid>,
    #[schema(value_type = crate::api::resources::schema_models::backups::BackupRepositoryValidationStatusSchema)]
    pub status: BackupRepositoryValidationStatus,
    pub last_validated_at: DateTime<Utc>,
    #[schema(required = true)]
    pub last_error_code: Option<String>,
    #[schema(required = true)]
    pub last_error_message: Option<String>,
}

impl TryFrom<citadel_backups::BackupRepositoryValidation> for BackupRepositoryValidationView {
    type Error = serde_json::Error;
    fn try_from(value: citadel_backups::BackupRepositoryValidation) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            backup_repository_id: value.backup_repository_id,
            location: serde_json::from_value(value.location.into())?,
            platform_id: value.platform_id,
            status: value.status,
            last_validated_at: value.last_validated_at,
            last_error_code: value.last_error_code,
            last_error_message: value.last_error_message,
        })
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupSourcePreview {
    #[serde(flatten)]
    pub resource: BackupPreviewResource,
    pub platform_id: Uuid,
    pub platform_name: String,
    #[schema(value_type = crate::api::resources::schema_models::primitives::PlatformStatusSchema)]
    pub platform_status: PlatformStatus,
    pub volumes: Vec<BackupVolumePreview>,
    pub warnings: Vec<String>,
}

impl TryFrom<citadel_backups::policies::read_models::BackupSourcePreview> for BackupSourcePreview {
    type Error = serde_json::Error;
    fn try_from(
        value: citadel_backups::policies::read_models::BackupSourcePreview,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
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
        })
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
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
    #[schema(value_type = Option<crate::api::resources::schema_models::backups::BackupRunStatusSchema>)]
    pub last_run_status: Option<BackupRunStatus>,
    pub last_run_at: Option<DateTime<Utc>>,
}

impl TryFrom<citadel_backups::runs::read_models::PlatformBackupSummary> for PlatformBackupSummary {
    type Error = serde_json::Error;
    fn try_from(
        value: citadel_backups::runs::read_models::PlatformBackupSummary,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
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
        })
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Repositories {
    pub(crate) repositories: Vec<BackupRepositoryView>,
    pub(crate) capabilities: ResourceCapabilitiesView,
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
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PlatformBackupSummaries {
    pub platforms: Vec<PlatformBackupSummary>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRunStreamItem {
    pub run_id: Uuid,
    #[schema(value_type = Option<crate::api::resources::schema_models::backups::BackupRunStatusSchema>)]
    pub status: Option<BackupRunStatus>,
    pub message: Option<String>,
    pub stream: Option<String>,
    pub exit_code: Option<i32>,
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreRunStreamItem {
    pub restore_run_id: Uuid,
    #[schema(value_type = Option<crate::api::resources::schema_models::backups::BackupRestoreStatusSchema>)]
    pub status: Option<BackupRestoreStatus>,
    pub message: Option<String>,
    pub stream: Option<String>,
    pub exit_code: Option<i32>,
}
