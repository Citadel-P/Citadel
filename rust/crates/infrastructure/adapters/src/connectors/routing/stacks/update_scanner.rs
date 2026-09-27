use super::*;
use citadel_primitives::ActorId;
use citadel_stacks::{
    StackDetails, StackSpec, StackUpdateScanner, StackUpdateState, build_manual_stack_checks,
    evaluate_manual_stack_updates, stack_git_path_matches, state_key,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub struct StackUpdateRuntime {
    runtime: StackRuntimeRouter,
    git: Arc<citadel_git::GitRepositoryExecutionService>,
}
impl StackUpdateRuntime {
    pub fn new(
        runtime: StackRuntimeRouter,
        git: Arc<citadel_git::GitRepositoryExecutionService>,
    ) -> Self {
        Self { runtime, git }
    }
}
impl StackUpdateScanner for StackUpdateRuntime {
    fn scan<'a>(
        &'a self,
        stack: &'a StackDetails,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        self.scan_mode(stack, false, cancel)
    }
    fn scan_cached<'a>(
        &'a self,
        stack: &'a StackDetails,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        self.scan_mode(stack, true, cancel)
    }
}
impl StackUpdateRuntime {
    fn scan_mode<'a>(
        &'a self,
        stack: &'a StackDetails,
        scheduled: bool,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        Box::pin(async move {
            let spec = stack.spec.as_ref().ok_or(StackError::NotFound)?;
            if let StackSpec::Git {
                git_repo_id,
                branch,
                commit_sha,
                ..
            } = spec
            {
                if commit_sha
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    return Err(StackError::Validation(
                        "The Stack is pinned to a Git commit.".into(),
                    ));
                }
                let source = stack
                    .source
                    .as_ref()
                    .filter(|source| {
                        source.source_type == citadel_stacks::StackSource::Git
                            && source.git_repository_id == Some(*git_repo_id)
                            && source.branch.as_deref() == Some(branch)
                            && !source.resolved_commit_sha.is_empty()
                    })
                    .ok_or_else(|| {
                        StackError::Conflict(
                            "The Stack has no applied Git commit to compare.".into(),
                        )
                    })?;
                let remote = self
                    .git
                    .synchronize_commit(
                        ActorId::new(Uuid::from_u128(1)),
                        *git_repo_id,
                        branch,
                        cancel,
                    )
                    .await
                    .map_err(git_failure)?;
                let relevant = if remote.eq_ignore_ascii_case(&source.resolved_commit_sha) {
                    false
                } else {
                    let changes = self
                        .git
                        .compare(*git_repo_id, &source.resolved_commit_sha, &remote, cancel)
                        .await
                        .map_err(git_failure)?;
                    changes.is_truncated
                        || changes.files.iter().any(|file| {
                            stack_git_path_matches(spec, source, &file.path)
                                || file
                                    .previous_path
                                    .as_deref()
                                    .is_some_and(|path| stack_git_path_matches(spec, source, path))
                        })
                };
                let mut next = stack.stack_update_state.clone();
                let StackUpdateState::Git {
                    recreate_stack_on_new_commit_state,
                    ..
                } = &mut next
                else {
                    return Err(StackError::Storage(
                        "The Stack update state is inconsistent.".into(),
                    ));
                };
                recreate_stack_on_new_commit_state.current_commit_sha =
                    source.resolved_commit_sha.clone();
                recreate_stack_on_new_commit_state.remote_commit_sha = relevant.then_some(remote);
                recreate_stack_on_new_commit_state.last_checked_at = chrono::Utc::now();
                return Ok(next);
            }
            if stack.platform_type == citadel_platforms::PlatformKind::DockerSwarm {
                return Err(StackError::Validation(
                    "Image update checks for native Swarm Stacks are unavailable.".into(),
                ));
            }
            let checks = build_manual_stack_checks(stack, false)?;
            if checks.len() > 256 {
                return Err(StackError::Validation(
                    "Too many Stack images to check in one operation.".into(),
                ));
            }
            let target = self
                .runtime
                .platform(stack.platform_id.ok_or(StackError::NotFound)?)
                .await?;
            let peer = if target.connector == citadel_platforms::ConnectorKind::Local {
                None
            } else {
                Some(self.runtime.agent_for(&target)?)
            };
            let (containers, images) = match &peer {
                Some(peer) => (
                    peer.list_containers(cancel)
                        .await
                        .map_err(runtime_failure)?,
                    peer.list_images(cancel).await.map_err(runtime_failure)?,
                ),
                None => (
                    citadel_platforms::ContainerInventoryPort::list_containers(
                        &self.runtime.docker,
                        cancel,
                    )
                    .await
                    .map_err(runtime_failure)?,
                    citadel_platforms::ImageInventoryPort::list_images(
                        &self.runtime.docker,
                        cancel,
                    )
                    .await
                    .map_err(runtime_failure)?,
                ),
            };
            let owned_id = stack.id.to_string();
            let project =
                spec.common().project_name.clone().unwrap_or_else(|| {
                    citadel_stacks::normalize_project_name(&stack.name, stack.id)
                });
            let owned = containers
                .iter()
                .filter(
                    |container| match container.labels.get("com.citadel.stack-id") {
                        Some(id) => id == &owned_id,
                        None => {
                            !container.labels.contains_key("com.citadel.deployment-id")
                                && container.labels.get("com.docker.compose.project")
                                    == Some(&project)
                        }
                    },
                )
                .collect::<Vec<_>>();
            let images = images
                .iter()
                .map(|image| (image.id.as_str(), image))
                .collect::<BTreeMap<_, _>>();
            let mut deployed = BTreeMap::new();
            let mut remote = BTreeMap::new();
            for check in &checks {
                let mut digests = BTreeSet::new();
                for container in &owned {
                    if container
                        .labels
                        .get("com.docker.compose.service")
                        .is_none_or(|service| service != &check.service_name)
                    {
                        continue;
                    }
                    let Some(image) = images.get(container.image_id.as_str()) else {
                        continue;
                    };
                    for digest in &image.repo_digests {
                        if repository(digest) == repository(&check.image_name)
                            && let Some((_, digest)) = digest.rsplit_once('@')
                        {
                            digests.insert(digest.to_owned());
                        }
                    }
                }
                if digests.len() != 1 {
                    return Err(StackError::Conflict(format!(
                        "The deployed digest for service '{}' is unavailable or ambiguous. Apply with image pulling enabled before checking again.",
                        check.service_name
                    )));
                }
                deployed.insert(
                    state_key(&check.service_name, &check.image_name),
                    digests.into_iter().next().unwrap(),
                );
                if !remote.contains_key(&check.key) {
                    let digest = if scheduled {
                        if !self.runtime.image_cache.wait_ready(cancel).await {
                            return Err(StackError::Cancelled);
                        }
                        self.runtime
                            .image_cache
                            .get(check.key.registry_id, &check.image_name)
                            .ok_or_else(|| {
                                StackError::Conflict(
                                    "No recent registry observation is available.".into(),
                                )
                            })?
                            .digest
                    } else {
                        crate::connectors::registries::digest::inspect(
                            &self.runtime.pool,
                            &self.runtime.docker,
                            peer.clone(),
                            check.key.registry_id,
                            &check.image_name,
                            cancel,
                        )
                        .await
                        .map_err(|_| {
                            StackError::Runtime(
                            "Registry update check failed. Verify connectivity and credentials."
                                .into(),
                        )
                        })?
                    };
                    remote.insert(check.key.clone(), digest);
                }
            }
            Ok(evaluate_manual_stack_updates(
                &stack.stack_update_state,
                &checks,
                &remote,
                chrono::Utc::now(),
                Some(&deployed),
            )
            .state)
        })
    }
}
fn runtime_failure(_: citadel_platforms::RuntimeCapabilityError) -> StackError {
    StackError::Runtime("Could not inspect the Stack's deployed images.".into())
}
fn git_failure(_: citadel_git::GitRepositoryExecutionError) -> StackError {
    StackError::Runtime(
        "Git update check failed. Verify repository connectivity and credentials.".into(),
    )
}
fn repository(reference: &str) -> String {
    let reference = reference.split('@').next().unwrap_or(reference);
    let reference = match reference.rfind(':') {
        Some(index) if reference.rfind('/').is_none_or(|slash| index > slash) => {
            &reference[..index]
        }
        _ => reference,
    };
    let reference = reference
        .strip_prefix("docker.io/")
        .or_else(|| reference.strip_prefix("index.docker.io/"))
        .unwrap_or(reference);
    reference
        .strip_prefix("library/")
        .unwrap_or(reference)
        .to_ascii_lowercase()
}
