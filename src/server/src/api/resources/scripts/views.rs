use super::spec::{ScriptRunStatus, ScriptRunTrigger};
use crate::api::resources::webhooks::views::WebhookView;
use chrono::{DateTime, Utc};
use citadel_primitives::ResourceControlState;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptView {
    pub id: Uuid,
    pub name: String,
    #[schema(required = true)]
    pub description: Option<String>,
    pub code: String,
    pub default_args_json: String,
    pub enabled: bool,
    pub schedule_enabled: bool,
    #[schema(required = true)]
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: String,
    #[schema(required = true, value_type = Option<crate::api::resources::webhooks::views::WebhookView>)]
    pub webhook: Option<WebhookView>,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    #[schema(value_type = super::spec::ScriptExecutorSchema)]
    pub executor: citadel_scripts::ScriptExecutor,
    pub run_as_actor_id: Uuid,
    #[schema(value_type = crate::api::resources::schema_models::primitives::ResourceControlStateSchema)]
    pub control_state: ResourceControlState,
    #[schema(required = true)]
    pub current_run_id: Option<Uuid>,
    pub row_version: i64,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[schema(required = true)]
    pub last_scheduled_run_at: Option<DateTime<Utc>>,
    pub tags: Vec<crate::api::resources::tags::views::TagSummary>,
    #[schema(required = true)]
    pub latest_run: Option<ScriptRunView>,
}

impl TryFrom<citadel_scripts::Script> for ScriptView {
    type Error = serde_json::Error;
    fn try_from(value: citadel_scripts::Script) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
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
            executor: value.executor,
            run_as_actor_id: value.run_as_actor_id,
            control_state: value.control_state,
            current_run_id: value.current_run_id,
            row_version: value.row_version,
            created_by_actor_id: value.audit.created_by_actor_id.value(),
            created_at: value.audit.created_at,
            updated_at: value.updated_at,
            last_scheduled_run_at: value.last_scheduled_run_at,
            tags: value.tags.into_iter().map(Into::into).collect(),
            latest_run: value.latest_run.map(ScriptRunView::try_from).transpose()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptRunView {
    pub id: Uuid,
    #[schema(required = true)]
    pub executor_id: Option<Uuid>,
    #[schema(required = true)]
    pub attempt_id: Option<Uuid>,
    pub cleanup_pending: bool,
    pub script_id: Uuid,
    pub script_name: String,
    pub trigger: ScriptRunTrigger,
    #[schema(value_type = crate::api::resources::schema_models::scripts::ScriptRunStatusSchema)]
    pub status: ScriptRunStatus,
    pub run_as_actor_id: Uuid,
    #[schema(required = true)]
    pub triggered_by_actor_id: Option<Uuid>,
    pub args_json: String,
    #[schema(required = true)]
    pub code_snapshot: Option<String>,
    pub code_hash: String,
    pub timeout_seconds: i32,
    pub queued_at: DateTime<Utc>,
    #[schema(required = true)]
    pub started_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub finished_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub duration_ms: Option<i64>,
    #[schema(required = true)]
    pub exit_code: Option<i32>,
    #[schema(required = true)]
    pub logs: Option<String>,
    #[schema(required = true)]
    pub error_message: Option<String>,
}

impl TryFrom<citadel_scripts::ScriptRun> for ScriptRunView {
    type Error = serde_json::Error;
    fn try_from(value: citadel_scripts::ScriptRun) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            executor_id: value.executor_id,
            attempt_id: value.attempt_id,
            cleanup_pending: value.cleanup_pending,
            script_id: value.script_id,
            script_name: value.script_name,
            trigger: serde_json::from_value(value.trigger.into())?,
            status: value.status,
            run_as_actor_id: value.run_as_actor_id,
            triggered_by_actor_id: value.triggered_by_actor_id,
            args_json: value.args_json,
            code_snapshot: value.code_snapshot,
            code_hash: value.code_hash,
            timeout_seconds: value.timeout_seconds,
            queued_at: value.queued_at,
            started_at: value.started_at,
            finished_at: value.finished_at,
            duration_ms: value.duration_ms,
            exit_code: value.exit_code,
            logs: value.logs,
            error_message: value.error_message,
        })
    }
}

#[derive(Debug, Default, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptProgress {
    #[schema(required = true)]
    pub run_id: Option<Uuid>,
    #[schema(required = true, value_type = Option<crate::api::resources::schema_models::scripts::ScriptRunStatusSchema>)]
    pub status: Option<ScriptRunStatus>,
    #[schema(required = true)]
    pub stream: Option<String>,
    #[schema(required = true)]
    pub progress_message: Option<String>,
    #[schema(required = true)]
    pub error_message: Option<String>,
    #[schema(required = true)]
    pub error: Option<ScriptProgressError>,
}

impl TryFrom<citadel_scripts::ScriptProgress> for ScriptProgress {
    type Error = serde_json::Error;
    fn try_from(value: citadel_scripts::ScriptProgress) -> Result<Self, Self::Error> {
        Ok(Self {
            run_id: value.run_id,
            status: value.status,
            stream: value.stream,
            progress_message: value.progress_message,
            error_message: value.error_message,
            error: value.error.map(Into::into),
        })
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ScriptProgressError {
    pub code: i32,
    pub message: String,
}

impl From<citadel_scripts::ScriptProgressError> for ScriptProgressError {
    fn from(value: citadel_scripts::ScriptProgressError) -> Self {
        Self {
            code: value.code,
            message: value.message,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScriptList {
    pub(crate) scripts: Vec<AuthorizedScript>,
    pub(crate) capabilities: crate::api::resources::capabilities::ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct AuthorizedScript {
    #[serde(flatten)]
    pub(crate) script: ScriptView,
    pub(crate) capabilities: crate::api::resources::capabilities::ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunList {
    pub(crate) runs: Vec<ScriptRunView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScriptRunLogsView {
    pub(crate) run_id: Uuid,
    pub(crate) logs: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptDuplicateDraftView {
    pub source_name: String,
    pub draft: super::requests::ScriptInput,
}
