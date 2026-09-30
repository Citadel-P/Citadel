use crate::*;
use chrono::{DateTime, Utc};
use citadel_primitives::WebhookConfig;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AutomationAction {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub default_args_json: String,
    pub enabled: bool,
    pub schedule_enabled: bool,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: String,
    pub webhook: Option<WebhookConfig>,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
    pub control_state: citadel_primitives::ResourceControlState,
    pub current_run_id: Option<Uuid>,
    pub row_version: i64,
    pub updated_at: DateTime<Utc>,
    pub last_scheduled_run_at: Option<DateTime<Utc>>,
    pub tags: Vec<citadel_tags::TagSummary>,
    pub latest_run: Option<AutomationRun>,
    pub audit: citadel_primitives::AuditMetadata,
}

impl AutomationAction {
    pub fn snapshot(&self) -> citadel_activities::AutomationActionActivitySnapshot {
        citadel_activities::AutomationActionActivitySnapshot {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            code: self.code.clone(),
            default_args_json: self.default_args_json.clone(),
            enabled: self.enabled,
            schedule_enabled: self.schedule_enabled,
            schedule_cron: self.schedule_cron.clone(),
            schedule_time_zone: self.schedule_time_zone.clone(),
            webhook: self.webhook.as_ref().map(WebhookConfig::redacted),
            timeout_seconds: self.timeout_seconds,
            alert_on_failure: self.alert_on_failure,
            run_as_actor_id: self.run_as_actor_id,
        }
    }
}
