use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BackupPolicyActivitySnapshot {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub source_type: String,
    pub source_key: String,
    pub backup_repository_id: Uuid,
    pub enabled: bool,
    pub cron: Option<String>,
    pub time_zone: Option<String>,
    pub webhook_enabled: bool,
    pub keep_last_successful: i32,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
}
