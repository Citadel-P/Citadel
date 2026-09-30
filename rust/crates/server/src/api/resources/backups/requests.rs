use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryInput {
    pub name: String,
    pub description: Option<String>,
    #[schema(value_type = crate::openapi::compatibility::BackupRepositorySpec)]
    pub spec: Value,
    pub password_secret_id: Uuid,
}

impl From<citadel_backups::BackupRepositoryConfiguration> for BackupRepositoryInput {
    fn from(value: citadel_backups::BackupRepositoryConfiguration) -> Self {
        Self {
            name: value.name,
            description: value.description,
            spec: value.spec,
            password_secret_id: value.password_secret_id,
        }
    }
}

impl From<BackupRepositoryInput> for citadel_backups::BackupRepositoryConfiguration {
    fn from(value: BackupRepositoryInput) -> Self {
        Self {
            name: value.name,
            description: value.description,
            spec: value.spec,
            password_secret_id: value.password_secret_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupPolicyInput {
    pub name: String,
    pub description: Option<String>,
    #[schema(value_type = crate::openapi::compatibility::BackupSourceSpec)]
    pub source: Value,
    pub backup_repository_id: Uuid,
    pub enabled: bool,
    pub cron: Option<String>,
    pub time_zone: Option<String>,
    #[schema(value_type = Option<crate::openapi::compatibility::BackupWebhookConfig>)]
    pub webhook: Option<Value>,
    pub keep_last_successful: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Option<Uuid>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl From<citadel_backups::BackupPolicyConfiguration> for BackupPolicyInput {
    fn from(value: citadel_backups::BackupPolicyConfiguration) -> Self {
        Self {
            name: value.name,
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
            tag_ids: value.tag_ids,
        }
    }
}

impl From<BackupPolicyInput> for citadel_backups::BackupPolicyConfiguration {
    fn from(value: BackupPolicyInput) -> Self {
        Self {
            name: value.name,
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
            tag_ids: value.tag_ids,
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RenameBackupPolicyInput {
    pub id: Uuid,
    pub name: String,
}

impl From<citadel_backups::policies::metadata::RenameBackupPolicyInput>
    for RenameBackupPolicyInput
{
    fn from(value: citadel_backups::policies::metadata::RenameBackupPolicyInput) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<RenameBackupPolicyInput>
    for citadel_backups::policies::metadata::RenameBackupPolicyInput
{
    fn from(value: RenameBackupPolicyInput) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryLocationInput {
    #[schema(value_type = crate::openapi::compatibility::BackupExecutionLocation)]
    pub(crate) location: String,
    pub(crate) platform_id: Option<Uuid>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RestoreInput {
    pub(crate) target_platform_id: Uuid,
    pub(crate) target_volume_name: String,
    pub(crate) overwrite_existing: bool,
    pub(crate) target_docker_node_id: Option<String>,
    pub(crate) source_backup_run_item_id: Option<Uuid>,
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
#[schema(as = server::backups_http::QueueInput)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueueInput {
    pub(crate) trigger: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunFilter {
    pub(crate) policy_id: Option<Uuid>,
    pub(crate) backup_run_id: Option<Uuid>,
    pub(crate) limit: Option<usize>,
}
