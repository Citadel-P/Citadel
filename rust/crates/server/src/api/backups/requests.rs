use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryInput {
    pub name: String,
    pub description: Option<String>,
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
    pub source: Value,
    pub backup_repository_id: Uuid,
    pub enabled: bool,
    pub cron: Option<String>,
    pub time_zone: Option<String>,
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
