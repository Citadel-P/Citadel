use crate::*;

#[derive(Debug, Clone)]
pub struct BackupPolicy {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub source: crate::spec::BackupSourceSpec,
    pub backup_repository_id: Uuid,
    pub enabled: bool,
    pub cron: Option<String>,
    pub time_zone: Option<String>,
    pub webhook: Option<citadel_primitives::WebhookConfig>,
    pub keep_last_successful: i32,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
    pub control_state: citadel_primitives::ResourceControlState,
    pub current_run_id: Option<Uuid>,
    pub last_scheduled_run_at: Option<DateTime<Utc>>,
    pub first_successful_run_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
    pub audit: citadel_primitives::AuditMetadata,

    // Related data populated by full resource reads.
    pub tags: Vec<citadel_tags::TagSummary>,
    pub latest_run: Option<crate::BackupRun>,
}
