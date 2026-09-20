use super::*;
use citadel_builds::BuildError;
use citadel_git::repositories::webhooks::repository_matches;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

pub(super) async fn receive(
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
    let paths = super::changed_paths(&payload);
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

#[cfg(test)]
mod tests {
    use super::*;

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
