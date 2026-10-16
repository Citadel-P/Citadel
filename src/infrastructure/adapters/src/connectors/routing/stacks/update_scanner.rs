use super::*;
use citadel_platforms::SwarmInventoryPort;
use citadel_primitives::ActorId;
use citadel_stacks::{
    StackSpec, StackUpdateScanner, StackUpdateState, build_stack_image_checks,
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
    fn scan_images<'a>(
        &'a self,
        stack: &'a citadel_stacks::Stack,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        self.scan_images(stack, false, cancel)
    }
    fn scan<'a>(
        &'a self,
        stack: &'a citadel_stacks::Stack,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        Box::pin(async move {
            let mut checked = stack.clone();
            if matches!(&stack.spec, Some(StackSpec::Git { commit_sha, .. }) if commit_sha.as_deref().is_none_or(|sha| sha.trim().is_empty()))
            {
                checked.stack_update_state = self.scan_mode(stack, false, cancel).await?;
            }
            if checked.spec.as_ref().is_some_and(|spec| {
                matches!(spec, StackSpec::WebEditor { .. }) || spec.common().registry_id.is_some()
            }) {
                self.scan_images(&checked, false, cancel).await
            } else if matches!(&stack.spec, Some(StackSpec::Git { commit_sha, .. }) if commit_sha.as_deref().is_some_and(|sha| !sha.trim().is_empty()))
            {
                Err(StackError::Validation("The Stack is pinned to a Git commit and has no Registry configured for image checks.".into()))
            } else {
                Ok(checked.stack_update_state)
            }
        })
    }
    fn scan_cached<'a>(
        &'a self,
        stack: &'a citadel_stacks::Stack,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        self.scan_mode(stack, true, cancel)
    }
    fn scan_images_cached<'a>(
        &'a self,
        stack: &'a citadel_stacks::Stack,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        self.scan_images(stack, true, cancel)
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
            self.scan_images(stack, scheduled, cancel).await
        })
    }

    fn scan_images<'a>(
        &'a self,
        stack: &'a citadel_stacks::Stack,
        scheduled: bool,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackUpdateState, StackError>> {
        Box::pin(async move {
            let spec = stack.spec.as_ref().ok_or(StackError::NotFound)?;
            let checks = stack_image_checks(&self.git, stack, scheduled, cancel).await?;
            if checks.is_empty() {
                return Ok(evaluate_manual_stack_updates(
                    &stack.stack_update_state,
                    &checks,
                    &BTreeMap::new(),
                    chrono::Utc::now(),
                    None,
                    None,
                )
                .state);
            }
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
            let mut swarm_service_ids = BTreeMap::new();
            let mut deployed_image_ids = BTreeMap::new();
            let deployed = if stack.platform_type == citadel_platforms::PlatformKind::DockerSwarm {
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
                let deployed = swarm_deployed_digests(stack.id, &project, &checks, &services)?;
                deployed
                    .into_iter()
                    .map(|(key, (digest, id))| {
                        swarm_service_ids.insert(key.clone(), id);
                        (key, digest)
                    })
                    .collect()
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
                    let mut image_ids = BTreeSet::new();
                    for container in &owned {
                        if container
                            .labels
                            .get("com.docker.compose.service")
                            .is_none_or(|service| service != &check.service_name)
                        {
                            continue;
                        }
                        let image = images.get(container.image_id.as_str()).ok_or_else(|| {
                            StackError::Conflict(format!(
                                "The deployed image for service '{}' is unavailable.",
                                check.service_name
                            ))
                        })?;
                        let mut identified = false;
                        for digest in &image.repo_digests {
                            if repository(digest) == repository(&check.image_name)
                                && let Some((_, digest)) = digest.rsplit_once('@')
                            {
                                digests.insert(digest.to_owned());
                                image_ids.insert(image.id.clone());
                                identified = true;
                            }
                        }
                        if !identified {
                            return Err(StackError::Conflict(format!(
                                "A deployed image for service '{}' has no comparable repository digest.",
                                check.service_name
                            )));
                        }
                    }
                    if digests.len() != 1 {
                        return Err(StackError::Conflict(format!(
                            "The deployed digest for service '{}' is unavailable or ambiguous. Apply with image pulling enabled before checking again.",
                            check.service_name
                        )));
                    }
                    deployed_image_ids
                        .insert(state_key(&check.service_name, &check.image_name), image_ids);
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
            let adapter = crate::connectors::registries::image_updates::ImageUpdateAdapter {
                pool: self.runtime.pool.clone(),
                runtime: self.runtime.runtime.clone(),
                cache: self.runtime.image_cache.clone(),
            };
            let platform = stack.platform_id.ok_or(StackError::NotFound)?;
            let mut content_changes = BTreeMap::new();
            let mut swarm_inventory = None;
            for check in &checks {
                let key = state_key(&check.service_name, &check.image_name);
                let current = &deployed[&key];
                let latest = &remote[&check.key];
                if citadel_primitives::image_digests_equal(current, latest) {
                    content_changes.insert(key, false);
                    continue;
                }
                let comparison =
                    crate::connectors::registries::workload_updates::WorkloadImageCheck {
                        platform,
                        registry: check.key.registry_id,
                        reference: &check.image_name,
                        current,
                        remote: latest,
                    };
                let targets = if let Some(id) = swarm_service_ids.get(&key) {
                    if swarm_inventory.is_none() {
                        let manager = self
                            .runtime
                            .runtime
                            .swarm(platform, cancel)
                            .await
                            .map_err(runtime_failure)?;
                        swarm_inventory = Some(
                            tokio::try_join!(
                                manager.list_swarm_nodes(cancel),
                                manager.list_swarm_tasks(cancel)
                            )
                            .map_err(runtime_failure)?,
                        );
                    }
                    let (nodes, tasks) = swarm_inventory.as_ref().unwrap();
                    crate::connectors::registries::workload_updates::swarm_targets(id, nodes, tasks)
                        .map_err(runtime_failure)?
                } else {
                    let ids = &deployed_image_ids[&key];
                    if ids.len() > 16 {
                        return Err(StackError::Conflict(
                            "Too many deployed image variants to compare.".into(),
                        ));
                    }
                    let mut targets = BTreeSet::new();
                    for id in ids {
                        targets.insert(
                            adapter
                                .deployed_image_target(&comparison, Some(id), cancel)
                                .await
                                .map_err(runtime_failure)?,
                        );
                    }
                    targets.into_iter().collect()
                };
                let changed = adapter
                    .workload_content_changed(&comparison, &targets, cancel)
                    .await
                    .map_err(runtime_failure)?;
                content_changes.insert(key, changed);
            }
            Ok(evaluate_manual_stack_updates(
                &stack.stack_update_state,
                &checks,
                &remote,
                chrono::Utc::now(),
                Some(&deployed),
                Some(&content_changes),
            )
            .state)
        })
    }
}
/// Reads only the applied revision from the local Git cache. This never synchronizes a branch.
pub(crate) async fn stack_image_checks(
    git: &citadel_git::GitRepositoryExecutionService,
    stack: &citadel_stacks::Stack,
    scheduled: bool,
    cancel: &CancellationToken,
) -> Result<Vec<citadel_stacks::ManualStackImageCheck>, StackError> {
    let spec = stack.spec.as_ref().ok_or(StackError::NotFound)?;
    let (StackSpec::Git {
        update_behavior, ..
    }
    | StackSpec::WebEditor {
        update_behavior, ..
    }) = spec;
    if scheduled && *update_behavior == citadel_stacks::StackUpdateBehavior::Disabled {
        return Ok(Vec::new());
    }
    if spec.common().registry_id.is_none_or(|id| id.is_nil()) {
        return Err(StackError::Validation(
            "The Stack has no Registry configured.".into(),
        ));
    }
    let files = match spec {
        StackSpec::WebEditor { compose_file, .. } => vec![compose_file.clone()],
        StackSpec::Git {
            git_repo_id,
            branch,
            compose_paths,
            ..
        } => {
            let source = stack
                .source
                .as_ref()
                .filter(|source| {
                    source.source_type == citadel_stacks::StackSource::Git
                        && source.git_repository_id == Some(*git_repo_id)
                        && source.branch.as_deref() == Some(branch)
                        && source.compose_paths == *compose_paths
                        && !source.resolved_commit_sha.is_empty()
                })
                .ok_or_else(|| {
                    StackError::Conflict("Apply the Stack before checking its images.".into())
                })?;
            let snapshot = git
                .stack_snapshot_files(
                    *git_repo_id,
                    Some(&source.resolved_commit_sha),
                    Some(&source.compose_paths),
                    cancel,
                )
                .await
                .map_err(git_failure)?;
            source
                .compose_paths
                .iter()
                .map(|path| {
                    let file = snapshot
                        .files
                        .iter()
                        .find(|file| file.relative_path == *path)
                        .ok_or_else(|| {
                            StackError::Conflict("An applied Compose file is unavailable.".into())
                        })?;
                    String::from_utf8(file.content.clone()).map_err(|_| {
                        StackError::Validation("The Compose file is not UTF-8.".into())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    build_stack_image_checks(stack, scheduled, &files)
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
) -> Result<BTreeMap<String, (String, String)>, StackError> {
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
            (digest.to_owned(), matching[0].id.clone()),
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
        let deployed = swarm_deployed_digests(owner, "demo", &checks, &[service])
            .unwrap()
            .into_iter()
            .map(|(key, (digest, _))| (key, digest))
            .collect::<BTreeMap<_, _>>();
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
            None,
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
