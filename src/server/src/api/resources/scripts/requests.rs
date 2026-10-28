use citadel_primitives::WebhookConfig;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptInput {
    #[serde(default)]
    pub duplicate_source_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub default_args_json: Option<String>,
    pub enabled: bool,
    pub schedule_enabled: bool,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: Option<String>,
    #[schema(value_type = Option<crate::api::resources::schema_models::primitives::WebhookConfigSchema>)]
    #[serde(serialize_with = "crate::api::resources::webhooks::views::serialize_optional")]
    pub webhook: Option<WebhookConfig>,
    pub timeout_seconds: Option<i32>,
    pub alert_on_failure: bool,
    #[serde(default)]
    #[schema(value_type = super::spec::ScriptExecutorSchema)]
    pub executor: citadel_scripts::ScriptExecutor,
    pub run_as_actor_id: Option<Uuid>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl From<ScriptInput> for citadel_scripts::ScriptConfiguration {
    fn from(value: ScriptInput) -> Self {
        Self {
            duplicate_source_id: value.duplicate_source_id,
            name: value.name,
            description: value.description,
            code: value.code,
            default_args_json: value.default_args_json,
            enabled: value.enabled,
            schedule_enabled: value.schedule_enabled,
            schedule_cron: value.schedule_cron,
            schedule_time_zone: value.schedule_time_zone,
            webhook: value.webhook,
            timeout_seconds: value.timeout_seconds,
            alert_on_failure: value.alert_on_failure,
            executor: value.executor,
            run_as_actor_id: value.run_as_actor_id,
            tag_ids: value.tag_ids,
        }
    }
}

impl From<citadel_scripts::ScriptConfiguration> for ScriptInput {
    fn from(value: citadel_scripts::ScriptConfiguration) -> Self {
        Self {
            duplicate_source_id: value.duplicate_source_id,
            name: value.name,
            description: value.description,
            code: value.code,
            default_args_json: value.default_args_json,
            enabled: value.enabled,
            schedule_enabled: value.schedule_enabled,
            schedule_cron: value.schedule_cron,
            schedule_time_zone: value.schedule_time_zone,
            webhook: value.webhook,
            timeout_seconds: value.timeout_seconds,
            alert_on_failure: value.alert_on_failure,
            executor: value.executor,
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

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveScriptCleanupInput {
    pub attempt_id: uuid::Uuid,
    pub reason: String,
    pub acknowledged: bool,
}
