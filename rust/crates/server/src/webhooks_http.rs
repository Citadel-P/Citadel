use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_domain::ActorId;
use citadel_git::{GitRepositoryExecutionError, GitRepositoryExecutionService, GitWebhookOutcome};
use serde::Serialize;
use uuid::Uuid;

const SYSTEM_ACTOR_ID: ActorId = ActorId::new(Uuid::from_u128(1));

#[derive(Clone)]
pub struct WebhooksHttpState {
    pub git: Arc<GitRepositoryExecutionService>,
    pub alerts: Option<Arc<dyn AlertEventSink>>,
}

pub fn router(state: WebhooksHttpState) -> Router {
    Router::new()
        .route(
            "/listener/{auth_type}/{resource_type}/{id}/{execution}",
            post(receive),
        )
        .with_state(state)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WebhookResponse {
    accepted: bool,
    status: &'static str,
    request_id: Uuid,
    reason: Option<&'static str>,
}

async fn receive(
    State(state): State<WebhooksHttpState>,
    Path((auth_type, resource_type, id, execution)): Path<(String, String, Uuid, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = Uuid::now_v7();
    if resource_type != "repo" || execution != "pull" {
        return webhook_error(
            StatusCode::BAD_REQUEST,
            request_id,
            "Unsupported webhook target.",
        );
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
    match state
        .git
        .receive_webhook(SYSTEM_ACTOR_ID, id, &auth_type, &headers, &body)
        .await
    {
        Ok(GitWebhookOutcome::Queued) => {
            resolve_webhook_alerts(state.alerts.as_ref(), id, request_id).await;
            (
                StatusCode::ACCEPTED,
                Json(WebhookResponse {
                    accepted: true,
                    status: "queued",
                    request_id,
                    reason: None,
                }),
            )
                .into_response()
        }
        Ok(GitWebhookOutcome::Ignored) => {
            resolve_webhook_alerts(state.alerts.as_ref(), id, request_id).await;
            (
                StatusCode::ACCEPTED,
                Json(WebhookResponse {
                    accepted: true,
                    status: "noop",
                    request_id,
                    reason: Some("Branch filter did not match"),
                }),
            )
                .into_response()
        }
        Err(GitRepositoryExecutionError::NotFound) => {
            webhook_error(StatusCode::NOT_FOUND, request_id, "Webhook not found.")
        }
        Err(GitRepositoryExecutionError::Authentication) => {
            report_webhook_failure(
                state.alerts.as_ref(),
                "WebhookAuthenticationFailed",
                id,
                request_id,
                "Webhook authentication failed.",
            )
            .await;
            webhook_error(
                StatusCode::UNAUTHORIZED,
                request_id,
                "Webhook authentication failed.",
            )
        }
        Err(GitRepositoryExecutionError::Validation(_)) => {
            report_webhook_failure(
                state.alerts.as_ref(),
                "WebhookDispatchFailed",
                id,
                request_id,
                "Webhook payload is invalid.",
            )
            .await;
            webhook_error(
                StatusCode::BAD_REQUEST,
                request_id,
                "Webhook payload is invalid.",
            )
        }
        Err(_) => {
            report_webhook_failure(
                state.alerts.as_ref(),
                "WebhookDispatchFailed",
                id,
                request_id,
                "Webhook dispatch failed.",
            )
            .await;
            webhook_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                request_id,
                "Webhook dispatch failed.",
            )
        }
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
