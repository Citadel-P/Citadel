//! Webhooks HTTP routes, authorization and local request handling.
use crate::{api::resources::webhooks::views::WebhookResponse, openapi::router::OpenApiRouterExt};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use chrono::Utc;

use citadel_activities::WebhookActivitySource;

use citadel_alerts::{AlertEventSink, AlertObservation};

use citadel_automation::{AutomationError, AutomationService};

use citadel_builds::BuildError;

use citadel_git::{
    GitRepositoryExecutionError, GitRepositoryExecutionService, GitWebhookOutcome,
    repositories::webhooks::{
        WebhookConfiguration, WebhookError, repository_matches, webhook_branch,
    },
};

use citadel_primitives::ActorId;

use citadel_stacks::{StackError, StackSpec, StackUpdateBehavior, stack_git_path_matches};

use citadel_swarm_services::{ServiceUpdateOutcome, SwarmServiceError};

use serde_json::Value;

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

const SYSTEM_ACTOR_ID: ActorId = ActorId::new(Uuid::from_u128(1));

#[derive(Clone)]
pub struct WebhooksHttpState {
    pub git: Arc<GitRepositoryExecutionService>,
    pub automation: Arc<AutomationService>,
    pub backups: Option<Arc<citadel_backups::BackupService>>,
    pub builds: Option<Arc<citadel_builds::BuildService>>,
    pub stacks: Option<Arc<citadel_stacks::StackService>>,
    pub services: Option<Arc<citadel_swarm_services::SwarmServiceService>>,
    pub alerts: Option<Arc<dyn AlertEventSink>>,
    pub audit: Option<Arc<dyn citadel_activities::WebhookActivitySink>>,
}

pub fn router(state: WebhooksHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<WebhooksHttpState> {
    utoipa_axum::router::OpenApiRouter::new().normalized_routes(utoipa_axum::routes!(receive))
}

#[utoipa::path(
    post,
    path = "/listener/{authType}/{resourceType}/{id}/{execution}",
    operation_id = "receiveWebhook",
    tag = "WebhookListener",
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
        let details = citadel_activities::WebhookActivityDetails {
            request_id,
            auth_type: auth_type.chars().take(32).collect(),
            execution: execution.chars().take(32).collect(),
            status,
            reason,
            source: if result.is_ok() {
                source(&headers, &body)
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

fn audit_resource(resource: &str) -> Option<citadel_activities::ActivityResourceType> {
    use citadel_activities::ActivityResourceType;
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
        let config = action
            .webhook
            .as_ref()
            .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
        let webhook = config
            .configuration()
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
        return match state.automation.queue_webhook(id, config, &args).await {
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
        return receive_build_webhook(state, id, auth_type, headers, body).await;
    }
    if resource.eq_ignore_ascii_case("stack") && execution.eq_ignore_ascii_case("deploy") {
        return receive_stack_webhook(state, id, auth_type, headers, body).await;
    }
    if resource.eq_ignore_ascii_case("swarm-service") && execution.eq_ignore_ascii_case("update") {
        return receive_service_webhook(state, id, auth_type, headers, body).await;
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

// Only authenticated, bounded identifiers are kept. Never persist clone URLs,
// arbitrary payloads or authentication headers as activity metadata.
fn source(headers: &[(String, String)], body: &[u8]) -> WebhookActivitySource {
    let payload: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let header = |names: &[&str]| {
        names.iter().find_map(|name| {
            headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
        })
    };
    let commit = string(
        payload
            .get("after")
            .or_else(|| payload.get("checkout_sha"))
            .or_else(|| payload.get("commitSha"))
            .or_else(|| payload.get("commit")),
    )
    .filter(|sha| matches!(sha.len(), 40 | 64) && sha.bytes().all(|b| b.is_ascii_hexdigit()));
    WebhookActivitySource {
        delivery_id: identifier(header(&[
            "x-github-delivery",
            "x-gitlab-event-uuid",
            "x-gitea-delivery",
            "x-forgejo-delivery",
            "webhook-id",
        ])),
        event_type: identifier(header(&[
            "x-github-event",
            "x-gitlab-event",
            "x-gitea-event",
            "x-forgejo-event",
        ])),
        branch: identifier(
            string(payload.get("ref").or_else(|| payload.get("branch")))
                .map(|branch| branch.strip_prefix("refs/heads/").unwrap_or(branch)),
        ),
        commit_sha: commit.map(str::to_owned),
        repository_full_name: identifier(string(
            payload
                .pointer("/repository/full_name")
                .or_else(|| payload.pointer("/project/path_with_namespace")),
        )),
    }
}

fn string(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str)
}

fn identifier(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 256
                && value
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | ' '))
        })
        .map(str::to_owned)
}

async fn receive_build_webhook(
    state: &WebhooksHttpState,
    id: Uuid,
    auth_type: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> DispatchResult {
    let builds = state
        .builds
        .as_ref()
        .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
    let project = builds.store().get(id).await.map_err(build_error)?;
    let mut webhook = WebhookConfiguration::from_value(project.webhook.as_ref())
        .map_err(webhook_auth_error)?
        .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
    if webhook
        .branch_filter
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        webhook.branch_filter = Some(project.branch.clone());
    }
    if let Some(reason) = webhook
        .evaluate(auth_type, headers, body)
        .map_err(webhook_auth_error)?
    {
        return Ok(WebhookDispatch::noop(reason));
    }
    if !project.enabled {
        return Ok(WebhookDispatch::noop("Build Project is disabled."));
    }
    if project.control_state != "Idle" {
        return Ok(WebhookDispatch::noop(
            "Build Project already has an active run.",
        ));
    }
    if let Err(error) = builds
        .ensure_execution_entitlements(&project, "Webhook")
        .await
    {
        return match error {
            BuildError::LicenseRequired(_) => Ok(WebhookDispatch::noop(
                "Build webhook requires an active license entitlement.",
            )),
            error => Err(build_error(error)),
        };
    }
    let payload: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let source = state
        .git
        .source(project.git_repository_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Linked Git repository is unavailable.",
            )
        })?;
    if !repository_matches(&source.url, &payload) {
        return Ok(WebhookDispatch::noop("Repository identity mismatch"));
    }
    let (_, payload_branch) =
        citadel_git::repositories::webhooks::webhook_branch(&webhook.provider, headers, body)
            .map_err(webhook_auth_error)?;
    let branch = payload_branch
        .as_deref()
        .or(webhook.branch_filter.as_deref())
        .unwrap_or(&project.branch);
    let mut commit = payload
        .get("after")
        .or_else(|| payload.get("checkout_sha"))
        .or_else(|| payload.get("commitSha"))
        .or_else(|| payload.get("commit"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    if commit.as_ref().is_some_and(|value| {
        !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Webhook commit must be a full commit ID.",
        ));
    }
    let paths = crate::api::routes::webhooks::changed_paths(&payload);
    if !paths.is_empty() {
        if !paths
            .iter()
            .any(|path| relevant_path(&project.context_path, &project.dockerfile_path, path))
        {
            return Ok(WebhookDispatch::noop("No relevant path changes"));
        }
    } else if let Some(previous) = &project.latest_run
        && previous.status == "Succeeded"
        && let Some(base) = previous.resolved_commit_sha.as_deref()
    {
        let cancellation = CancellationToken::new();
        let head = state
            .git
            .synchronize_commit(
                SYSTEM_ACTOR_ID,
                project.git_repository_id,
                branch,
                &cancellation,
            )
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Repository synchronization failed.",
                )
            })?;
        if head == base {
            return Ok(WebhookDispatch::noop("No new commit"));
        }
        if let Ok(changes) = state
            .git
            .compare(project.git_repository_id, base, &head, &cancellation)
            .await
            && !changes.is_truncated
            && !changes.files.iter().any(|file| {
                relevant_path(&project.context_path, &project.dockerfile_path, &file.path)
                    || file.previous_path.as_deref().is_some_and(|path| {
                        relevant_path(&project.context_path, &project.dockerfile_path, path)
                    })
            })
        {
            return Ok(WebhookDispatch::noop("No relevant path changes"));
        }
        commit = Some(head);
    }
    match builds
        .queue_webhook(&project, branch, commit.as_deref())
        .await
    {
        Ok(()) => Ok(WebhookDispatch::queued(branch, commit.as_deref())),
        Err(BuildError::Conflict(_)) => Ok(WebhookDispatch::noop(
            "Build Project is busy or its configuration changed.",
        )),
        Err(BuildError::LicenseRequired(_)) => Ok(WebhookDispatch::noop(
            "Build webhook requires an active license entitlement.",
        )),
        Err(error) => Err(build_error(error)),
    }
}

fn build_error(error: BuildError) -> (StatusCode, &'static str) {
    match error {
        BuildError::NotFound => (StatusCode::NOT_FOUND, "Webhook not found."),
        BuildError::Validation(_) => (
            StatusCode::BAD_REQUEST,
            "Build webhook configuration is invalid.",
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Build webhook dispatch failed.",
        ),
    }
}

fn relevant_path(context: &str, dockerfile: &str, path: &str) -> bool {
    let path = path.replace('\\', "/");
    let path = path.trim().trim_matches('/');
    let context = context.trim_matches('/');
    context.is_empty()
        || context == "."
        || path == context
        || path == dockerfile
        || path
            .strip_prefix(context)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

async fn receive_service_webhook(
    state: &WebhooksHttpState,
    id: Uuid,
    auth_type: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> DispatchResult {
    let services = state
        .services
        .as_ref()
        .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
    let snapshot = services
        .get(SYSTEM_ACTOR_ID, true, id)
        .await
        .map_err(service_error)?;
    let config = snapshot
        .spec
        .webhook
        .as_ref()
        .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
    let value = serde_json::to_value(config).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Webhook configuration is invalid.",
        )
    })?;
    let mut webhook = WebhookConfiguration::from_value(Some(&value))
        .map_err(webhook_auth_error)?
        .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
    // A Service follows an image tag, not a Git branch.
    webhook.branch_filter = None;
    if let Some(reason) = webhook
        .evaluate(auth_type, headers, body)
        .map_err(webhook_auth_error)?
    {
        return Ok(WebhookDispatch::noop(reason));
    }
    let cancellation = CancellationToken::new();
    let _guard = cancellation.clone().drop_guard();
    match services
        .check_automated_updates(snapshot, &cancellation)
        .await
    {
        Ok(ServiceUpdateOutcome::UpdateAvailable | ServiceUpdateOutcome::ApplyStarted) => {
            Ok(WebhookDispatch::default())
        }
        Ok(ServiceUpdateOutcome::Noop(reason)) => Ok(WebhookDispatch::noop(reason)),
        Err(SwarmServiceError::Conflict(_)) => Ok(WebhookDispatch::noop(
            "Service is busy, unavailable, or changed during dispatch.",
        )),
        Err(error) => Err(service_error(error)),
    }
}

fn service_error(error: SwarmServiceError) -> (StatusCode, &'static str) {
    match error {
        SwarmServiceError::NotFound => (StatusCode::NOT_FOUND, "Webhook not found."),
        SwarmServiceError::Validation(_) => (
            StatusCode::BAD_REQUEST,
            "Service update configuration is invalid.",
        ),
        SwarmServiceError::Runtime(_) => (
            StatusCode::BAD_GATEWAY,
            "Registry update check failed. Verify registry connectivity and credentials.",
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Service webhook dispatch failed.",
        ),
    }
}

async fn receive_stack_webhook(
    state: &WebhooksHttpState,
    id: Uuid,
    auth_type: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> DispatchResult {
    let stacks = state
        .stacks
        .as_ref()
        .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
    let stack = stacks
        .get(SYSTEM_ACTOR_ID, true, id)
        .await
        .map_err(stack_error)?;
    let Some(StackSpec::Git {
        git_repo_id,
        branch,
        commit_sha,
        update_behavior,
        webhook: Some(config),
        ..
    }) = stack.spec.as_ref()
    else {
        return Err((StatusCode::NOT_FOUND, "Webhook not found."));
    };
    let value = serde_json::to_value(config).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Webhook configuration is invalid.",
        )
    })?;
    let mut webhook = WebhookConfiguration::from_value(Some(&value))
        .map_err(webhook_auth_error)?
        .ok_or((StatusCode::NOT_FOUND, "Webhook not found."))?;
    if webhook
        .branch_filter
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        webhook.branch_filter = Some(branch.clone());
    }
    if let Some(reason) = webhook
        .evaluate(auth_type, headers, body)
        .map_err(webhook_auth_error)?
    {
        return Ok(WebhookDispatch::noop(reason));
    }
    if commit_sha
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Ok(WebhookDispatch::noop("Stack is pinned to a commit."));
    }
    if *update_behavior == StackUpdateBehavior::Disabled {
        return Ok(WebhookDispatch::noop("Stack Git updates are disabled."));
    }
    let payload: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let source = state.git.source(*git_repo_id).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Linked Git repository is unavailable.",
        )
    })?;
    if !repository_matches(&source.url, &payload) {
        return Ok(WebhookDispatch::noop("Repository identity mismatch"));
    }
    let (_, payload_branch) =
        webhook_branch(&webhook.provider, headers, body).map_err(webhook_auth_error)?;
    if payload_branch
        .as_deref()
        .or(webhook.branch_filter.as_deref())
        != Some(branch)
    {
        return Ok(WebhookDispatch::noop("Branch mismatch"));
    }
    let mut commit = payload
        .get("after")
        .or_else(|| payload.get("checkout_sha"))
        .or_else(|| payload.get("commitSha"))
        .or_else(|| payload.get("commit"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    if commit.as_ref().is_some_and(|sha| {
        !matches!(sha.len(), 40 | 64) || !sha.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Webhook commit must be a full commit ID.",
        ));
    }
    if !config.force_deploy
        && let Some(applied) = &stack.source
    {
        let paths = crate::api::routes::webhooks::changed_paths(&payload);
        if !paths.is_empty() {
            if !paths
                .iter()
                .any(|path| stack_git_path_matches(stack.spec.as_ref().unwrap(), applied, path))
            {
                return Ok(WebhookDispatch::noop("No relevant path changes"));
            }
        } else if !applied.resolved_commit_sha.is_empty() {
            let cancellation = CancellationToken::new();
            let head = state
                .git
                .synchronize_commit(SYSTEM_ACTOR_ID, *git_repo_id, branch, &cancellation)
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Repository synchronization failed.",
                    )
                })?;
            if head.eq_ignore_ascii_case(&applied.resolved_commit_sha) {
                return Ok(WebhookDispatch::noop("No new commit"));
            }
            if let Ok(changes) = state
                .git
                .compare(
                    *git_repo_id,
                    &applied.resolved_commit_sha,
                    &head,
                    &cancellation,
                )
                .await
                && !changes.is_truncated
                && !changes.files.iter().any(|file| {
                    stack_git_path_matches(stack.spec.as_ref().unwrap(), applied, &file.path)
                        || file.previous_path.as_deref().is_some_and(|path| {
                            stack_git_path_matches(stack.spec.as_ref().unwrap(), applied, path)
                        })
                })
            {
                return Ok(WebhookDispatch::noop("No relevant path changes"));
            }
            commit = Some(head);
        }
    }
    if *update_behavior == StackUpdateBehavior::Notify {
        state
            .git
            .request_sync(SYSTEM_ACTOR_ID, *git_repo_id, Some(branch))
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Repository synchronization failed.",
                )
            })?;
        return Ok(WebhookDispatch::noop("Stack update notification queued."));
    }
    match stacks.queue_webhook(&stack, commit.as_deref()).await {
        Ok(()) => Ok(WebhookDispatch::queued(branch, commit.as_deref())),
        Err(StackError::LicenseRequired(_)) => Ok(WebhookDispatch::noop(
            "Automated operations require an active license entitlement.",
        )),
        Err(StackError::Conflict(_)) => Ok(WebhookDispatch::noop(
            "Stack webhook configuration changed during dispatch.",
        )),
        Err(error) => Err(stack_error(error)),
    }
}

fn stack_error(error: StackError) -> (StatusCode, &'static str) {
    match error {
        StackError::NotFound => (StatusCode::NOT_FOUND, "Webhook not found."),
        StackError::Validation(_) => (
            StatusCode::BAD_REQUEST,
            "Stack webhook configuration is invalid.",
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Stack webhook dispatch failed.",
        ),
    }
}

#[cfg(test)]
mod audit_tests {
    use crate::api::routes::webhooks::*;

    #[test]
    fn provider_metadata_is_bounded_and_never_uses_clone_urls_or_credentials() {
        let payload = serde_json::json!({"ref":"refs/heads/main","after":"a".repeat(40),"repository":{"full_name":"team/project","clone_url":"https://user:secret@host/project"}});
        let source = source(
            &[
                ("x-forgejo-delivery".into(), "delivery-1".into()),
                ("authorization".into(), "Bearer secret".into()),
            ],
            &serde_json::to_vec(&payload).unwrap(),
        );
        assert_eq!(source.branch.as_deref(), Some("main"));
        assert_eq!(source.repository_full_name.as_deref(), Some("team/project"));
        assert_eq!(source.delivery_id.as_deref(), Some("delivery-1"));
        assert!(!serde_json::to_string(&source).unwrap().contains("secret"));
        assert!(identifier(Some(&"x".repeat(257))).is_none());
        assert!(identifier(Some("https://user:secret@host")).is_none());
    }
}

#[cfg(test)]
mod builds_tests {
    use crate::api::routes::webhooks::*;

    #[test]
    fn build_context_matches_dotnet_change_matcher_without_sibling_prefixes() {
        assert!(relevant_path(".", "Dockerfile", "docs/readme.md"));
        assert!(relevant_path(
            "apps/api",
            "Dockerfile",
            "apps/api/src/main.rs"
        ));
        assert!(relevant_path("apps/api", "Dockerfile", "Dockerfile"));
        assert!(!relevant_path(
            "apps/api",
            "Dockerfile",
            "apps/api-other/main.rs"
        ));
        assert!(!relevant_path("apps/api", "Dockerfile", "docs/readme.md"));
        assert!(relevant_path(
            "apps/api",
            "Dockerfile",
            "apps\\api\\main.rs"
        ));
    }

    #[test]
    fn supplied_repository_identity_cannot_target_an_unrelated_repository() {
        assert!(repository_matches(
            "https://example.test/team/repo.git",
            &serde_json::json!({"repository":{"full_name":"team/repo"}})
        ));
        assert!(!repository_matches(
            "https://example.test/other-team/repo.git",
            &serde_json::json!({"repository":{"full_name":"team/repo"}})
        ));
        assert!(!repository_matches(
            "https://example.test/team/repo.git",
            &serde_json::json!({"repository":"https://example.test/team/other.git"})
        ));
    }
}
