use super::*;
use citadel_primitives::ActorId;
use citadel_stacks::{
    StackSpec, StackUpdateScanner, StackUpdateState, build_manual_stack_checks,
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
        stack: &'a citadel_stacks::Stack,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        self.scan_mode(stack, false, cancel)
    }
    fn scan_cached<'a>(
        &'a self,
        stack: &'a citadel_stacks::Stack,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        self.scan_mode(stack, true, cancel)
    }
}
impl StackUpdateRuntime {
    fn scan_mode<'a>(
        &'a self,
        stack: &'a citadel_stacks::Stack,
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
                let (remote, checked_at) = if scheduled {
                    let reference = self
                        .git
                        .synchronized_ref(*git_repo_id, branch)
                        .await
                        .map_err(git_failure)?;
                    // Use the observation's timestamp so a concurrent sync is
                    // still eligible for the next check.
                    (
                        reference.resolved_commit_sha.ok_or_else(|| {
                            git_failure(citadel_git::GitRepositoryExecutionError::NotSynchronized)
                        })?,
                        reference.last_synced_at,
                    )
                } else {
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
                    (remote, chrono::Utc::now())
                };
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
                recreate_stack_on_new_commit_state.last_checked_at = checked_at;
                return Ok(next);
            }
            let checks = build_manual_stack_checks(stack, scheduled)?;
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
                Some(self.runtime.agent_for(&target, cancel).await?)
            };
            let owned_id = stack.id.to_string();
            let project =
                spec.common().project_name.clone().unwrap_or_else(|| {
                    citadel_stacks::normalize_project_name(&stack.name, stack.id)
                });
            let deployed = if stack.platform_type == citadel_platforms::PlatformKind::DockerSwarm {
                use citadel_platforms::SwarmInventoryPort;
                let manager = self
                    .runtime
                    .runtime
                    .swarm(stack.platform_id.ok_or(StackError::NotFound)?, cancel)
                    .await
                    .map_err(runtime_failure)?;
                let services = manager
                    .list_swarm_services(cancel)
                    .await
                    .map_err(runtime_failure)?;
                swarm_deployed_digests(stack.id, &project, &checks, &services)?
            } else {
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
                }
                deployed
            };
            let mut remote = BTreeMap::new();
            for check in &checks {
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

fn swarm_deployed_digests(
    stack_id: Uuid,
    project: &str,
    checks: &[citadel_stacks::ManualStackImageCheck],
    services: &[citadel_platforms::RuntimeSwarmService],
) -> Result<BTreeMap<String, String>, StackError> {
    let owner = stack_id.to_string();
    let mut deployed = BTreeMap::new();
    for check in checks {
        let name = format!("{project}_{}", check.service_name);
        let matching = services
            .iter()
            .filter(|service| {
                service.name == name
                    && service.stack_namespace.as_deref() == Some(project)
                    && !service.labels.contains_key("com.citadel.service-id")
                    && !service.labels.contains_key("com.citadel.deployment-id")
                    && service
                        .labels
                        .get("com.citadel.stack-id")
                        .is_none_or(|id| id == &owner)
            })
            .collect::<Vec<_>>();
        let digest = if matching.len() == 1
            && repository(&matching[0].image) == repository(&check.image_name)
        {
            matching[0]
                .image
                .rsplit_once('@')
                .map(|(_, digest)| digest)
                .filter(|digest| {
                    digest.strip_prefix("sha256:").is_some_and(|hash| {
                        hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
                })
        } else {
            None
        };
        let digest = digest.ok_or_else(|| StackError::Conflict(format!(
            "The deployed digest for Swarm service '{}' is unavailable or ambiguous. Deploy the Stack with registry access before checking again.", check.service_name
        )))?;
        deployed.insert(
            state_key(&check.service_name, &check.image_name),
            digest.to_owned(),
        );
    }
    Ok(deployed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_platforms::RuntimeSwarmService;
    use citadel_stacks::{ManualStackImageCheck, StackImageKey};

    fn check() -> ManualStackImageCheck {
        ManualStackImageCheck {
            service_name: "web".into(),
            image_name: "nginx:alpine".into(),
            key: StackImageKey {
                registry_id: Uuid::nil(),
                repository: "nginx".into(),
                tag: "alpine".into(),
            },
        }
    }
    fn service(owner: Uuid) -> RuntimeSwarmService {
        RuntimeSwarmService {
            name: "demo_web".into(),
            stack_namespace: Some("demo".into()),
            image: format!("docker.io/library/nginx:alpine@sha256:{}", "a".repeat(64)),
            labels: BTreeMap::from([("com.citadel.stack-id".into(), owner.to_string())]),
            ..Default::default()
        }
    }
    #[test]
    fn swarm_digest_comes_from_manager_service_even_without_local_tasks() {
        let owner = Uuid::now_v7();
        let service = service(owner);
        let checks = vec![check()];
        let deployed = swarm_deployed_digests(owner, "demo", &checks, &[service]).unwrap();
        assert_eq!(
            deployed[&state_key("web", "nginx:alpine")],
            format!("sha256:{}", "a".repeat(64))
        );
        let remote =
            BTreeMap::from([(checks[0].key.clone(), format!("sha256:{}", "b".repeat(64)))]);
        let evaluation = evaluate_manual_stack_updates(
            &StackUpdateState::WebEditor {
                recreate_stack_on_new_image_state: Default::default(),
            },
            &checks,
            &remote,
            chrono::Utc::now(),
            Some(&deployed),
        );
        assert_eq!(evaluation.baselines_created, 0);
        assert_eq!(evaluation.available_updates.len(), 1);
    }
    #[test]
    fn swarm_digest_rejects_wrong_ownership_namespace_image_missing_and_ambiguous_services() {
        let owner = Uuid::now_v7();
        let good = service(owner);
        let mut wrong_namespace = good.clone();
        wrong_namespace.stack_namespace = Some("other".into());
        let mut unpinned = good.clone();
        unpinned.image = "nginx:alpine".into();
        let mut malformed = good.clone();
        malformed.image = "nginx:alpine@sha256:bad".into();
        let mut changed_repo = good.clone();
        changed_repo.image = format!("redis@sha256:{}", "a".repeat(64));
        let mut managed_service = good.clone();
        managed_service
            .labels
            .insert("com.citadel.service-id".into(), Uuid::now_v7().to_string());
        for services in [
            vec![],
            vec![service(Uuid::now_v7())],
            vec![wrong_namespace],
            vec![unpinned],
            vec![malformed],
            vec![changed_repo],
            vec![managed_service],
            vec![good.clone(), good],
        ] {
            assert!(swarm_deployed_digests(owner, "demo", &[check()], &services).is_err());
        }
    }
}
