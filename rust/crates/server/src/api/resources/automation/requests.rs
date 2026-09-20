use crate::api::resources::git_repositories::webhook::RepoWebhookConfig;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationActionInput {
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub default_args_json: Option<String>,
    pub enabled: bool,
    pub schedule_enabled: bool,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: Option<String>,
    pub webhook: Option<RepoWebhookConfig>,
    pub timeout_seconds: Option<i32>,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Option<Uuid>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl From<AutomationActionInput> for citadel_automation::AutomationActionConfiguration {
    fn from(value: AutomationActionInput) -> Self {
        Self {
            name: value.name,
            description: value.description,
            code: value.code,
            default_args_json: value.default_args_json,
            enabled: value.enabled,
            schedule_enabled: value.schedule_enabled,
            schedule_cron: value.schedule_cron,
            schedule_time_zone: value.schedule_time_zone,
            webhook: value.webhook.map(Into::into),
            timeout_seconds: value.timeout_seconds,
            alert_on_failure: value.alert_on_failure,
            run_as_actor_id: value.run_as_actor_id,
            tag_ids: value.tag_ids,
        }
    }
}

impl From<citadel_automation::AutomationActionConfiguration> for AutomationActionInput {
    fn from(value: citadel_automation::AutomationActionConfiguration) -> Self {
        Self {
            name: value.name,
            description: value.description,
            code: value.code,
            default_args_json: value.default_args_json,
            enabled: value.enabled,
            schedule_enabled: value.schedule_enabled,
            schedule_cron: value.schedule_cron,
            schedule_time_zone: value.schedule_time_zone,
            webhook: value.webhook.map(Into::into),
            timeout_seconds: value.timeout_seconds,
            alert_on_failure: value.alert_on_failure,
            run_as_actor_id: value.run_as_actor_id,
            tag_ids: value.tag_ids,
        }
    }
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunInput {
    pub(crate) args_json: Option<Value>,
    pub(crate) timeout_seconds: Option<i32>,
    pub(crate) code: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenameInput {
    pub(crate) id: Uuid,
    pub(crate) name: String,
}

#[derive(Deserialize, Default)]
pub(crate) struct LimitQuery {
    pub(crate) limit: Option<usize>,
}
