use std::sync::Arc;

use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::StackDetails;
use crate::StackError;
use crate::StackService;
use crate::StackSpec;
use crate::StackUpdateBehavior;

pub trait StackEntitlements: Send + Sync {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, StackError>>;
    fn operational_guardrails(&self) -> BoxFuture<'_, Result<bool, StackError>> {
        Box::pin(async { Ok(false) })
    }
}

#[derive(Debug, Clone)]
pub struct StackWebhookJob {
    pub id: Uuid,
    pub stack_id: Uuid,
}

/// Ignore Apply's generated Build provenance, but fence every user-configured field.
pub fn stack_webhook_fingerprint(spec: &StackSpec) -> Result<String, StackError> {
    let mut spec = spec.clone();
    for binding in &mut spec.common_mut().build_image_bindings {
        binding.resolved_image_reference = None;
        binding.resolved_digest = None;
        binding.resolved_build_run_id = None;
        binding.applied_image_reference = None;
        binding.applied_digest = None;
        binding.applied_build_run_id = None;
        binding.applied_at = None;
    }
    let json = serde_json::to_vec(&spec).map_err(|error| StackError::Storage(error.to_string()))?;
    Ok(Sha256::digest(json)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub fn can_queue_stack_webhook(spec: &StackSpec) -> bool {
    matches!(spec, StackSpec::Git { commit_sha, update_behavior: StackUpdateBehavior::StackAutoDeploy,
        webhook: Some(webhook), .. } if commit_sha.as_deref().is_none_or(|sha|sha.trim().is_empty()) && webhook.enabled)
}

pub fn stack_git_path_matches(
    spec: &StackSpec,
    source: &crate::StackReleaseSource,
    changed: &str,
) -> bool {
    let StackSpec::Git {
        watch_paths,
        working_directory,
        compose_paths,
        compose_env_files_from_repo,
        additional_env_file_from_repo,
        ..
    } = spec
    else {
        return false;
    };
    let normalize = |path: &str| {
        path.replace('\\', "/")
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("/")
    };
    let changed = normalize(changed.trim());
    if changed.is_empty() {
        return false;
    }
    let matches = |watch: &str| {
        let watch = normalize(watch.trim());
        if watch == "." || watch == "**" {
            return true;
        }
        let watch = watch.strip_suffix("/**").unwrap_or(&watch);
        !watch.is_empty()
            && (changed == watch
                || changed
                    .strip_prefix(watch)
                    .is_some_and(|suffix| suffix.starts_with('/')))
    };
    let explicit = if watch_paths.is_empty() {
        source.watch_paths.as_deref().unwrap_or_default()
    } else {
        watch_paths
    };
    if !explicit.is_empty() {
        return explicit.iter().any(|path| matches(path));
    }
    let parent = compose_paths
        .first()
        .or_else(|| source.compose_paths.first())
        .and_then(|path| path.rsplit_once('/').map(|(parent, _)| parent));
    let directory = working_directory
        .as_deref()
        .or(source.working_directory.as_deref())
        .or(parent);
    if directory.is_some_and(|path| path != "." && !path.is_empty() && matches(path)) {
        return true;
    }
    let paths = compose_paths
        .iter()
        .chain(&source.compose_paths)
        .chain(compose_env_files_from_repo)
        .chain(additional_env_file_from_repo)
        .chain(
            source
                .compose_env_files_from_repo
                .as_deref()
                .unwrap_or(&source.env_file_paths),
        );
    let mut any = false;
    for path in paths {
        any = true;
        if matches(path) {
            return true;
        }
    }
    !any && directory.is_none_or(|path| path.is_empty() || path == ".")
}

impl StackService {
    pub fn with_entitlements(mut self, entitlements: Arc<dyn StackEntitlements>) -> Self {
        self.entitlements = Some(entitlements);
        self
    }

    pub async fn automated_operations_enabled(&self) -> Result<bool, StackError> {
        match &self.entitlements {
            Some(entitlements) => entitlements.automated_operations().await,
            None => Ok(false),
        }
    }

    pub async fn queue_webhook(
        &self,
        expected: &StackDetails,
        commit: Option<&str>,
    ) -> Result<(), StackError> {
        if !self.automated_operations_enabled().await? {
            return Err(StackError::LicenseRequired("AutomatedOperations"));
        }
        self.store.enqueue_webhook(expected, commit).await
    }

    /// One bounded batch; returns jobs consumed (including discarded jobs).
    /// Stack's existing Apply claim and recovery own Docker execution.
    pub async fn process_webhooks(&self) -> Result<usize, StackError> {
        let jobs = self.store.ready_webhooks(10).await?;
        let mut processed = 0;
        for job in jobs {
            if self.shutdown.is_cancelled() {
                break;
            }
            if !self.automated_operations_enabled().await? {
                self.store.discard_webhook(job.id).await?;
                processed += 1;
                continue;
            }
            match self
                .start_apply(
                    ActorId::new(Uuid::from_u128(1)),
                    true,
                    job.stack_id,
                    None,
                    Some(job.id),
                )
                .await
            {
                Ok(mut output) => {
                    // Dropping HTTP or this worker cannot cancel an already-claimed Apply.
                    // The durable queue is settled by complete_apply/fail_apply in the same transaction.
                    while output.recv().await.is_some() {}
                    processed += 1;
                }
                Err(StackError::NotFound) => {
                    self.store.discard_webhook(job.id).await?;
                    processed += 1;
                }
                Err(StackError::Conflict(_)) => {} // Busy or claimed concurrently; do not spend a retry.
                Err(error) => return Err(error),
            }
        }
        Ok(processed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn spec() -> StackSpec {
        serde_json::from_value(json!({"$type":"Git","gitRepoId":Uuid::from_u128(20),"branch":"main","composePaths":["apps/api/compose.yml"],"updateBehavior":"StackAutoDeploy","webhook":{"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"test-secret"}})).unwrap()
    }
    fn source() -> crate::StackReleaseSource {
        serde_json::from_value(json!({"sourceType":"Git","resolvedCommitSha":"a".repeat(40),"composePaths":["apps/api/compose.yml"],"envFilePaths":["settings/app.env"]})).unwrap()
    }
    #[test]
    fn queue_requires_unpinned_git_auto_deploy_and_enabled_webhook() {
        let mut spec = spec();
        assert!(can_queue_stack_webhook(&spec));
        if let StackSpec::Git { commit_sha, .. } = &mut spec {
            *commit_sha = Some("a".repeat(40));
        }
        assert!(!can_queue_stack_webhook(&spec));
        if let StackSpec::Git {
            commit_sha,
            update_behavior,
            ..
        } = &mut spec
        {
            *commit_sha = None;
            *update_behavior = StackUpdateBehavior::Notify;
        }
        assert!(!can_queue_stack_webhook(&spec));
        if let StackSpec::Git {
            update_behavior,
            webhook,
            ..
        } = &mut spec
        {
            *update_behavior = StackUpdateBehavior::StackAutoDeploy;
            webhook.as_mut().unwrap().enabled = false;
        }
        assert!(!can_queue_stack_webhook(&spec));
    }
    #[test]
    fn git_watch_paths_match_dotnet_without_sibling_prefixes() {
        let mut spec = spec();
        let source = source();
        for path in [
            "apps/api/config.json",
            "apps\\api\\compose.yml",
            "settings/app.env",
        ] {
            assert!(stack_git_path_matches(&spec, &source, path), "{path}");
        }
        for path in ["apps/api-other/compose.yml", "docs/readme.md", ""] {
            assert!(!stack_git_path_matches(&spec, &source, path), "{path}");
        }
        if let StackSpec::Git { watch_paths, .. } = &mut spec {
            *watch_paths = vec!["deployment/**".into()];
        }
        assert!(stack_git_path_matches(&spec, &source, "deployment/app.yml"));
        assert!(!stack_git_path_matches(
            &spec,
            &source,
            "apps/api/compose.yml"
        ));
        assert!(!stack_git_path_matches(
            &spec,
            &source,
            "deployment-other/app.yml"
        ));
    }
    #[test]
    fn fingerprint_ignores_generated_build_provenance_but_not_user_configuration() {
        let mut spec = spec();
        spec.common_mut().build_image_bindings.push(serde_json::from_value(json!({"serviceName":"api","buildProjectId":Uuid::from_u128(21),"redeployOnBuild":true})).unwrap());
        let fingerprint = stack_webhook_fingerprint(&spec).unwrap();
        spec.common_mut().build_image_bindings[0].applied_digest = Some("sha256:abc".into());
        assert_eq!(stack_webhook_fingerprint(&spec).unwrap(), fingerprint);
        spec.common_mut().build_image_bindings[0].redeploy_on_build = false;
        assert_ne!(stack_webhook_fingerprint(&spec).unwrap(), fingerprint);
    }
}
