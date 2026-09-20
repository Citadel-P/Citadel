use crate::api::git::repositories::webhook::RepoWebhookConfig;

use chrono::{DateTime, Utc};

use serde::Serialize;

use uuid::Uuid;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationActionView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub default_args_json: String,
    pub enabled: bool,
    pub schedule_enabled: bool,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: String,
    pub webhook: Option<RepoWebhookConfig>,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
    pub control_state: String,
    pub current_run_id: Option<Uuid>,
    pub row_version: i64,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_scheduled_run_at: Option<DateTime<Utc>>,
    pub tags: Vec<crate::api::tags::dto::TagSummary>,
    pub latest_run: Option<AutomationRunView>,
}

impl From<citadel_automation::AutomationAction> for AutomationActionView {
    fn from(value: citadel_automation::AutomationAction) -> Self {
        Self {
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
            run_as_actor_id: value.run_as_actor_id,
            control_state: value.control_state,
            current_run_id: value.current_run_id,
            row_version: value.row_version,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            last_scheduled_run_at: value.last_scheduled_run_at,
            tags: value.tags.into_iter().map(Into::into).collect(),
            latest_run: value.latest_run.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRunView {
    pub id: Uuid,
    pub action_id: Uuid,
    pub action_name: String,
    pub trigger: String,
    pub status: String,
    pub run_as_actor_id: Uuid,
    pub triggered_by_actor_id: Option<Uuid>,
    pub args_json: String,
    pub code_snapshot: Option<String>,
    pub code_hash: String,
    pub timeout_seconds: i32,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub logs: Option<String>,
    pub error_message: Option<String>,
}

impl From<citadel_automation::AutomationRun> for AutomationRunView {
    fn from(value: citadel_automation::AutomationRun) -> Self {
        Self {
            id: value.id,
            action_id: value.action_id,
            action_name: value.action_name,
            trigger: value.trigger,
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
        }
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationProgress {
    pub run_id: Option<Uuid>,
    pub status: Option<String>,
    pub stream: Option<String>,
    pub progress_message: Option<String>,
    pub error_message: Option<String>,
    pub error: Option<AutomationProgressError>,
}

impl From<citadel_automation::AutomationProgress> for AutomationProgress {
    fn from(value: citadel_automation::AutomationProgress) -> Self {
        Self {
            run_id: value.run_id,
            status: value.status,
            stream: value.stream,
            progress_message: value.progress_message,
            error_message: value.error_message,
            error: value.error.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AutomationProgressError {
    pub code: i32,
    pub message: String,
}

impl From<citadel_automation::AutomationProgressError> for AutomationProgressError {
    fn from(value: citadel_automation::AutomationProgressError) -> Self {
        Self {
            code: value.code,
            message: value.message,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct ActionList {
    pub(super) actions: Vec<AuthorizedAction>,
    pub(super) capabilities: crate::capabilities::ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct AuthorizedAction {
    #[serde(flatten)]
    pub(super) action: AutomationActionView,
    pub(super) capabilities: crate::capabilities::ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunList {
    pub(super) runs: Vec<AutomationRunView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct AutomationActionRunLogsView {
    pub(super) run_id: Uuid,
    pub(super) logs: String,
}
