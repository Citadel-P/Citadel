use super::*;
use citadel_resources::webhooks::{repository_matches, webhook_branch};
use citadel_stacks::{StackError, StackSpec, StackUpdateBehavior, stack_git_path_matches};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

pub(super) async fn receive(
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
        let paths = super::changed_paths(&payload);
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
