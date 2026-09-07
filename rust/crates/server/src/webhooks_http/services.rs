use super::*;
use citadel_swarm_services::{ServiceUpdateOutcome, SwarmServiceError};
use tokio_util::sync::CancellationToken;

pub(super) async fn receive(
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
    // A Service follows an image tag, not a Git branch (.NET compatibility).
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
