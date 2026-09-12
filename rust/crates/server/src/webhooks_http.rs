use std::sync::Arc;

use crate::openapi::router::OpenApiRouterExt;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use chrono::Utc;
use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_automation::{AutomationError, AutomationService};
use citadel_domain::ActorId;
use citadel_git::{GitRepositoryExecutionError, GitRepositoryExecutionService, GitWebhookOutcome};
use citadel_resources::webhooks::{WebhookConfiguration, WebhookError};
use serde::Serialize;
use uuid::Uuid;

mod audit;
mod builds;
mod services;
mod stacks;

const SYSTEM_ACTOR_ID: ActorId = ActorId::new(Uuid::from_u128(1));

#[derive(Clone)]
pub struct WebhooksHttpState {
    pub git: Arc<GitRepositoryExecutionService>,
    pub automation: Arc<AutomationService>,
    pub backups: Option<Arc<citadel_backups::BackupService>>,
    pub builds: Option<Arc<citadel_builds::BuildService>>,
    pub stacks: Option<Arc<citadel_stacks::StackService>>,
    pub services: Option<Arc<citadel_swarm_services::ManagedSwarmServiceService>>,
    pub alerts: Option<Arc<dyn AlertEventSink>>,
    pub audit: Option<Arc<dyn citadel_application::WebhookActivitySink>>,
}

pub fn router(state: WebhooksHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct WebhookResponse {
    accepted: bool,
    status: &'static str,
    request_id: Uuid,
    reason: Option<&'static str>,
}

#[utoipa::path(
    post,
    path = "/listener/{authType}/{resourceType}/{id}/{execution}",
    operation_id = "receiveWebhook",
    summary = "Receive a provider webhook delivery",
    request_body = serde_json::Value,
    responses(
        (status = 202, description = "Success", body = WebhookResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("authType" = String, Path), ("resourceType" = String, Path), ("id" = uuid::Uuid, Path), ("execution" = String, Path)),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(true)))
)]
async fn receive(
    State(state): State<WebhooksHttpState>,
    Path((auth_type, resource_type, id, execution)): Path<(String, String, Uuid, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = Uuid::now_v7();
    if body.len() > 1024 * 1024 {
        return webhook_error(
            StatusCode::BAD_REQUEST,
            request_id,
            "Request body too large.",
        );
    }
    for name in [
        "authorization",
        "x-gitlab-token",
        "x-hub-signature-256",
        "x-gitea-signature",
        "x-forgejo-signature",
        "webhook-id",
        "webhook-timestamp",
        "webhook-signature",
    ] {
        if headers.get_all(name).iter().count() > 1 {
            return webhook_error(
                StatusCode::BAD_REQUEST,
                request_id,
                "Ambiguous webhook authentication headers.",
            );
        }
    }
    let headers = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_owned(), value.to_owned()))
        })
        .collect::<Vec<_>>();
    let result = dispatch(
        &state,
        &resource_type,
        &execution,
        id,
        &auth_type,
        &headers,
        &body,
    )
    .await;
    if let (Some(audit), Some(resource)) = (&state.audit, audit_resource(&resource_type)) {
        let (status, reason) = match &result {
            Ok(outcome) => (
                if outcome.reason.is_some() {
                    "noop"
                } else {
                    "queued"
                },
                outcome.reason,
            ),
            Err((_, reason)) => ("rejected", Some(*reason)),
        };
        let details = citadel_domain::WebhookActivityDetails {
            request_id,
            auth_type: auth_type.chars().take(32).collect(),
            execution: execution.chars().take(32).collect(),
            status,
            reason,
            source: if result.is_ok() {
                audit::source(&headers, &body)
            } else {
                Default::default()
            },
            dispatched_branch: result
                .as_ref()
                .ok()
                .and_then(|outcome| outcome.branch.clone()),
            dispatched_commit_sha: result
                .as_ref()
                .ok()
                .and_then(|outcome| outcome.commit.clone()),
        };
        // Dispatch may already have committed a durable job or Apply claim.
        // An audit outage must not return a failure that asks the sender to
        // replay that mutation. Bound audit latency independently of execution.
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            audit.record_webhook(resource, id, details),
        )
        .await
        {
            Ok(Ok(())) => {}
            _ => tracing::warn!(%request_id, "Webhook audit persistence failed after dispatch"),
        }
    }
    match result {
        Ok(outcome) => {
            let reason = outcome.reason;
            resolve_webhook_alerts(state.alerts.as_ref(), id, request_id).await;
            (
                StatusCode::ACCEPTED,
                Json(WebhookResponse {
                    accepted: true,
                    status: if reason.is_some() { "noop" } else { "queued" },
                    request_id,
                    reason,
                }),
            )
                .into_response()
        }
        Err((status, reason)) => {
            if status != StatusCode::NOT_FOUND {
                let alert_type = if status == StatusCode::UNAUTHORIZED {
                    "WebhookAuthenticationFailed"
                } else {
                    "WebhookDispatchFailed"
                };
                report_webhook_failure(state.alerts.as_ref(), alert_type, id, request_id, reason)
                    .await;
            }
            webhook_error(status, request_id, reason)
        }
    }
}

type DispatchResult = Result<WebhookDispatch, (StatusCode, &'static str)>;

#[derive(Default)]
struct WebhookDispatch {
    reason: Option<&'static str>,
    branch: Option<String>,
    commit: Option<String>,
}
impl WebhookDispatch {
    fn noop(reason: &'static str) -> Self {
        Self {
            reason: Some(reason),
            ..Self::default()
        }
    }
    fn queued(branch: &str, commit: Option<&str>) -> Self {
        Self {
            reason: None,
            branch: Some(branch.into()),
            commit: commit.map(str::to_owned),
        }
    }
}

fn audit_resource(resource: &str) -> Option<citadel_domain::ActivityResourceType> {
    use citadel_domain::ActivityResourceType;
    if resource.eq_ignore_ascii_case("repo") {
        Some(ActivityResourceType::GitRepository)
    } else if resource.eq_ignore_ascii_case("stack") {
        Some(ActivityResourceType::Stack)
    } else if resource.eq_ignore_ascii_case("swarm-service") {
        Some(ActivityResourceType::SwarmService)
    } else if ["build", "build-project", "buildProject"]
        .iter()
        .any(|name| resource.eq_ignore_ascii_case(name))
    {
        Some(ActivityResourceType::Build)
    } else {
        None
    }
}

fn changed_paths(payload: &serde_json::Value) -> Vec<&str> {
    use serde_json::Value;
    payload
        .get("changedPaths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .chain(
            payload
                .get("commits")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .flat_map(|commit| {
                    ["added", "modified", "removed"]
                        .into_iter()
                        .flat_map(move |field| {
                            commit
                                .get(field)
                                .and_then(Value::as_array)
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                        })
                }),
        )
        .collect()
}

async fn dispatch(
    state: &WebhooksHttpState,
    resource: &str,
    execution: &str,
    id: Uuid,
    auth_type: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> DispatchResult {
    if resource.eq_ignore_ascii_case("repo") && execution.eq_ignore_ascii_case("pull") {
        return match state
            .git
            .receive_webhook(SYSTEM_ACTOR_ID, id, auth_type, headers, body)
            .await
        {
            Ok(GitWebhookOutcome::Queued { branch }) => Ok(WebhookDispatch::queued(&branch, None)),
            Ok(GitWebhookOutcome::Ignored) => {
                Ok(WebhookDispatch::noop("Branch filter did not match"))
            }
            Err(GitRepositoryExecutionError::NotFound) => {
                Err((StatusCode::NOT_FOUND, "Webhook not found."))
            }
            Err(GitRepositoryExecutionError::Authentication) => {
                Err((StatusCode::UNAUTHORIZED, "Webhook authentication failed."))
            }
            Err(GitRepositoryExecutionError::Conflict) => Ok(WebhookDispatch::noop(
                "Repository webhook configuration changed during dispatch.",
            )),
            Err(GitRepositoryExecutionError::Validation(_)) => Err((
                StatusCode::BAD_REQUEST,
                "Webhook payload or configuration is invalid.",
            )),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Webhook dispatch failed.",
            )),
        };
    }
    if (resource.eq_ignore_ascii_case("action")
        || resource.eq_ignore_ascii_case("automation-action"))
        && execution.eq_ignore_ascii_case("run")
    {
        let action = state
            .automation
            .store()
            .get(id)
            .await
            .map_err(automation_error)?;
        let webhook = WebhookConfiguration::from_value(action.webhook.as_ref())
            .map_err(webhook_auth_error)?
            .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
        if let Some(reason) = webhook
            .evaluate(auth_type, headers, body)
            .map_err(webhook_auth_error)?
        {
            return Ok(WebhookDispatch::noop(reason));
        }
        let args = normalize_payload(body);
        // Queue durably, using the configured run-as Actor, not the caller's token.
        // A short HTTP acknowledgement must not own or cancel the script process.
        return match state
            .automation
            .queue_webhook(id, action.webhook.as_ref().unwrap(), &args)
            .await
        {
            Ok(()) => Ok(WebhookDispatch::default()),
            Err(AutomationError::Conflict(_)) => Ok(WebhookDispatch::noop(
                "Action is disabled, busy, or its webhook configuration changed.",
            )),
            Err(AutomationError::LicenseRequired) => Ok(WebhookDispatch::noop(
                "Automated operations require an active license entitlement.",
            )),
            Err(error) => Err(automation_error(error)),
        };
    }
    if (resource.eq_ignore_ascii_case("backup-policy")
        || resource.eq_ignore_ascii_case("backupPolicy"))
        && execution.eq_ignore_ascii_case("run")
    {
        let backups = state
            .backups
            .as_ref()
            .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
        let policy = backups.store().get_policy(id).await.map_err(backup_error)?;
        let webhook = WebhookConfiguration::from_value(policy.webhook.as_ref())
            .map_err(webhook_auth_error)?
            .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
        if let Some(reason) = webhook
            .evaluate(auth_type, headers, body)
            .map_err(webhook_auth_error)?
        {
            return Ok(WebhookDispatch::noop(reason));
        }
        return match backups
            .queue_webhook(id, policy.webhook.as_ref().unwrap())
            .await
        {
            Ok(()) => Ok(WebhookDispatch::default()),
            Err(citadel_backups::BackupError::Conflict(_)) => Ok(WebhookDispatch::noop(
                "Backup Policy is disabled, busy, or its webhook configuration changed.",
            )),
            Err(citadel_backups::BackupError::LicenseRequired) => Ok(WebhookDispatch::noop(
                "Automated operations require an active license entitlement.",
            )),
            Err(error) => Err(backup_error(error)),
        };
    }
    if ["build", "build-project", "buildProject"]
        .iter()
        .any(|name| resource.eq_ignore_ascii_case(name))
        && execution.eq_ignore_ascii_case("run")
    {
        return builds::receive(state, id, auth_type, headers, body).await;
    }
    if resource.eq_ignore_ascii_case("stack") && execution.eq_ignore_ascii_case("deploy") {
        return stacks::receive(state, id, auth_type, headers, body).await;
    }
    if resource.eq_ignore_ascii_case("swarm-service") && execution.eq_ignore_ascii_case("update") {
        return services::receive(state, id, auth_type, headers, body).await;
    }
    Err((StatusCode::BAD_REQUEST, "Unsupported webhook target."))
}

fn backup_error(error: citadel_backups::BackupError) -> (StatusCode, &'static str) {
    match error {
        citadel_backups::BackupError::NotFound => (StatusCode::NOT_FOUND, "Webhook not found."),
        citadel_backups::BackupError::Validation(_) => {
            (StatusCode::BAD_REQUEST, "Webhook payload is invalid.")
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Webhook dispatch failed.",
        ),
    }
}

fn automation_error(error: AutomationError) -> (StatusCode, &'static str) {
    match error {
        AutomationError::NotFound => (StatusCode::NOT_FOUND, "Webhook not found."),
        AutomationError::Validation(_) => (StatusCode::BAD_REQUEST, "Webhook payload is invalid."),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Webhook dispatch failed.",
        ),
    }
}

fn webhook_auth_error(error: WebhookError) -> (StatusCode, &'static str) {
    match error {
        WebhookError::Authentication => {
            (StatusCode::UNAUTHORIZED, "Webhook authentication failed.")
        }
        WebhookError::Validation(_) => (
            StatusCode::BAD_REQUEST,
            "Webhook payload or configuration is invalid.",
        ),
    }
}

fn normalize_payload(body: &[u8]) -> serde_json::Value {
    if body.iter().all(u8::is_ascii_whitespace) {
        return serde_json::json!({});
    }
    match serde_json::from_slice::<serde_json::Value>(body) {
        Ok(value) if value.is_object() => value,
        Ok(value) => serde_json::json!({"payload":value}),
        Err(_) => serde_json::json!({"payload":String::from_utf8_lossy(body)}),
    }
}

async fn report_webhook_failure(
    alerts: Option<&Arc<dyn AlertEventSink>>,
    alert_type: &str,
    resource_id: Uuid,
    request_id: Uuid,
    message: &str,
) {
    let Some(alerts) = alerts else { return };
    let observation = webhook_observation(alert_type, resource_id, request_id, message, true);
    if let Err(error) = alerts.observe(&observation).await {
        tracing::warn!(%error, %request_id, alert_type, "Webhook Alert evaluation failed");
    }
}

async fn resolve_webhook_alerts(
    alerts: Option<&Arc<dyn AlertEventSink>>,
    resource_id: Uuid,
    request_id: Uuid,
) {
    let Some(alerts) = alerts else { return };
    for alert_type in ["WebhookAuthenticationFailed", "WebhookDispatchFailed"] {
        let observation = webhook_observation(
            alert_type,
            resource_id,
            request_id,
            "Webhook request was accepted.",
            false,
        );
        if let Err(error) = alerts.observe(&observation).await {
            tracing::warn!(%error, %request_id, alert_type, "Webhook Alert resolution failed");
        }
    }
}

fn webhook_observation(
    alert_type: &str,
    resource_id: Uuid,
    request_id: Uuid,
    message: &str,
    matched: bool,
) -> AlertObservation {
    AlertObservation {
        alert_type: alert_type.to_owned(),
        info: serde_json::json!({
            "RequestId": request_id,
            "ResourceId": resource_id,
            "Reason": message,
            "HumanMessage": message,
        }),
        resource_id,
        resource_name: resource_id.to_string(),
        resource_type: "Webhook".to_owned(),
        deduplication_component: "listener".to_owned(),
        observed_at: Utc::now(),
        value: None,
        matched,
    }
}

fn webhook_error(status: StatusCode, request_id: Uuid, reason: &'static str) -> Response {
    (
        status,
        Json(WebhookResponse {
            accepted: false,
            status: "rejected",
            request_id,
            reason: Some(reason),
        }),
    )
        .into_response()
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<WebhooksHttpState> {
    utoipa_axum::router::OpenApiRouter::new().normalized_routes(utoipa_axum::routes!(receive))
}
