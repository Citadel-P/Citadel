use super::*;
impl StackService {
    pub async fn apply(
        &self,
        actor: ActorId,
        administrator: bool,
        input: ApplyStack,
    ) -> Result<mpsc::Receiver<StackProgressItem>, StackError> {
        self.start_apply(actor, administrator, input.id, None, None)
            .await
    }

    pub async fn rollback(
        &self,
        actor: ActorId,
        administrator: bool,
        input: RollbackStack,
    ) -> Result<mpsc::Receiver<StackProgressItem>, StackError> {
        self.start_apply(
            actor,
            administrator,
            input.stack_id,
            Some(input.release_id),
            None,
        )
        .await
    }

    pub(crate) async fn start_apply(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        rollback: Option<Uuid>,
        webhook_job_id: Option<Uuid>,
    ) -> Result<mpsc::Receiver<StackProgressItem>, StackError> {
        self.start_apply_versioned(
            actor,
            administrator,
            id,
            rollback,
            webhook_job_id,
            Default::default(),
        )
        .await
    }

    pub async fn apply_versioned(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        expected_version: i64,
    ) -> Result<mpsc::Receiver<StackProgressItem>, StackError> {
        self.apply_selected(
            actor,
            administrator,
            id,
            crate::StackApplyOptions {
                expected_version: Some(expected_version),
                service_names: Vec::new(),
            },
        )
        .await
    }

    pub async fn apply_selected(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        options: crate::StackApplyOptions,
    ) -> Result<mpsc::Receiver<StackProgressItem>, StackError> {
        self.start_apply_versioned(actor, administrator, id, None, None, options)
            .await
    }

    pub(super) async fn start_apply_versioned(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        rollback: Option<Uuid>,
        webhook_job_id: Option<Uuid>,
        options: crate::StackApplyOptions,
    ) -> Result<mpsc::Receiver<StackProgressItem>, StackError> {
        let permit = self.operations.clone().try_acquire_owned().map_err(|_| {
            StackError::Conflict("Too many Stack operations are already running.".to_owned())
        })?;
        let claim = self
            .store
            .claim_apply_versioned(actor, administrator, id, rollback, webhook_job_id, options)
            .await?;
        let runtime = Arc::clone(&self.runtime);
        let source_materializer = self.source_materializer.clone();
        let store = Arc::clone(&self.store);
        let bindings = Arc::clone(&self.bindings);
        let build_images = self.build_images.clone();
        let notifier = Arc::clone(&self.notifier);
        let alerts = self.alerts.clone();
        let cancellation = self.shutdown.child_token();
        let timeout = self.timeout;
        let (sender, receiver) = mpsc::channel(32);
        let rejected_claim = claim.clone();
        if !self.tasks.spawn(
            "stack.apply",
            Box::pin(async move {
                let _permit = permit;
                execute_apply(
                    runtime,
                    source_materializer,
                    store,
                    bindings,
                    build_images,
                    notifier,
                    alerts,
                    actor,
                    claim,
                    cancellation,
                    timeout,
                    sender,
                )
                .await
            }),
        ) {
            self.store
                .fail_apply(actor, &rejected_claim, "Server is shutting down.", false)
                .await?;
            self.notifier.changed(id, "failed");
            return Err(StackError::Cancelled);
        }
        Ok(receiver)
    }

    pub async fn reconcile_stale_operations(
        &self,
        older_than: Duration,
        limit: i64,
    ) -> Result<usize, StackError> {
        let cutoff = chrono::Utc::now().timestamp()
            - i64::try_from(older_than.as_secs()).unwrap_or(i64::MAX);
        let claims = self
            .store
            .stale_apply_claims(cutoff, limit.clamp(1, 100))
            .await?;
        let mut count = 0;
        for (actor, claim) in claims {
            if claim.platform_type == citadel_platforms::PlatformKind::DockerSwarm {
                // Namespace/task counts cannot prove that every Service in this
                // release was accepted. Release the expired claim as recoverable;
                // the authoritative Swarm snapshot owns convergence and metadata checks.
                self.store
                    .fail_apply(
                        actor,
                        &claim,
                        "Stack deployment was interrupted before its outcome could be confirmed.",
                        true,
                    )
                    .await?;
                self.notifier.changed(claim.stack_id, "reconciled");
                count += 1;
                continue;
            }
            let result = tokio::time::timeout(
                Duration::from_secs(30),
                self.runtime.observe(&claim, &self.shutdown.child_token()),
            )
            .await;
            match result {
                Ok(Ok(Some(result))) if result.status == StackReleaseStatus::Healthy => {
                    self.store
                        .complete_apply(actor, &claim, &result, &[], None)
                        .await?;
                    self.notifier.changed(claim.stack_id, "reconciled");
                    count += 1;
                }
                Ok(Ok(Some(result)))
                    if matches!(
                        result.status,
                        StackReleaseStatus::Failed | StackReleaseStatus::TimedOut
                    ) =>
                {
                    let message = result
                        .messages
                        .last()
                        .and_then(|item| item.message.as_deref())
                        .unwrap_or("Stack runtime reported a failed deployment.");
                    self.store.fail_apply(actor, &claim, message, false).await?;
                    self.notifier.changed(claim.stack_id, "failed");
                    count += 1;
                }
                _ => {}
            }
        }
        let remaining = limit
            .clamp(1, 100)
            .saturating_sub(i64::try_from(count).unwrap_or(limit));
        let mut remaining = remaining;
        if remaining > 0 {
            for (actor, claim) in self.store.stale_state_claims(cutoff, remaining).await? {
                let snapshot = self
                    .runtime
                    .runtime_snapshot(
                        claim.platform_id,
                        &claim.project_name,
                        orchestration(&claim.platform_type),
                        &self.shutdown.child_token(),
                    )
                    .await?;
                if let Some(status) = stable_runtime_status(&snapshot) {
                    let container_ids = snapshot
                        .containers
                        .iter()
                        .map(|container| container.docker_container_id.clone())
                        .collect::<Vec<_>>();
                    self.store
                        .complete_state(actor, &claim, status, &container_ids)
                        .await?;
                    self.notifier.changed(claim.stack_id, "stateReconciled");
                    count += 1;
                }
            }
            remaining = limit
                .clamp(1, 100)
                .saturating_sub(i64::try_from(count).unwrap_or(limit));
        }
        if remaining > 0 {
            for (actor, claim) in self.store.stale_delete_claims(cutoff, remaining).await? {
                let orchestration = orchestration(&claim.platform_type);
                let snapshot = self
                    .runtime
                    .runtime_snapshot(
                        claim.platform_id,
                        &claim.project_name,
                        orchestration,
                        &self.shutdown.child_token(),
                    )
                    .await?;
                if snapshot.containers.is_empty() && snapshot.services.is_empty() {
                    self.store
                        .complete_delete(actor, std::slice::from_ref(&claim))
                        .await?;
                    self.notifier.changed(claim.stack_id, "deleted");
                    count += 1;
                } else {
                    self.store.release_delete(&[claim.stack_id]).await?;
                    self.notifier.changed(claim.stack_id, "deleteRecovered");
                    count += 1;
                }
            }
        }
        Ok(count)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_apply(
    runtime: Arc<dyn StackRuntime>,
    source_materializer: Option<Arc<dyn StackSourceMaterializerPort>>,
    store: Arc<dyn StackRepository>,
    bindings: Arc<dyn StackBindingResolverPort>,
    build_images: Option<Arc<dyn StackBuildImageResolverPort>>,
    notifier: Arc<dyn StackChangeNotifier>,
    alerts: Option<Arc<dyn AlertEventSink>>,
    actor: ActorId,
    mut claim: StackOperationClaim,
    cancellation: CancellationToken,
    timeout: Duration,
    sender: mpsc::Sender<StackProgressItem>,
) -> Result<(), StackError> {
    let _ = sender
        .send(StackProgressItem::system(
            "Resolving Stack variables and secrets...",
        ))
        .await;
    let mut source = match &claim.spec {
        StackSpec::WebEditor { compose_file, .. } => crate::StackApplySource {
            files: vec![StackSourceFile {
                relative_path: "compose.yml".to_owned(),
                content: compose_file.as_bytes().to_vec(),
            }],
            compose_paths: vec!["compose.yml".to_owned()],
            env_file_paths: Vec::new(),
            working_directory: ".".to_owned(),
            labels_override_path: None,
            resolved_commit_sha: None,
        },
        StackSpec::Git { .. } => {
            let Some(materializer) = source_materializer else {
                return fail_apply_before_runtime(
                    &store,
                    actor,
                    &claim,
                    &sender,
                    "Git Stack source materialization is unavailable.",
                )
                .await;
            };
            match materializer.materialize(&claim, &cancellation).await {
                Ok(source) => source,
                Err(error) => {
                    return fail_apply_before_runtime(
                        &store,
                        actor,
                        &claim,
                        &sender,
                        &error.to_string(),
                    )
                    .await;
                }
            }
        }
    };
    let compose_files = match source.compose_contents() {
        Ok(files) => files,
        Err(error) => {
            return fail_apply_before_runtime(&store, actor, &claim, &sender, &error.to_string())
                .await;
        }
    };
    let resolved_build_images = if claim.spec.common().build_image_bindings.is_empty() {
        Vec::new()
    } else {
        let Some(resolver) = build_images else {
            return fail_apply_before_runtime(
                &store,
                actor,
                &claim,
                &sender,
                "Stack Build image resolution is unavailable.",
            )
            .await;
        };
        match resolver
            .resolve(&claim.spec.common().build_image_bindings)
            .await
        {
            Ok(resolved) => resolved,
            Err(error) => {
                return fail_apply_before_runtime(
                    &store,
                    actor,
                    &claim,
                    &sender,
                    &error.to_string(),
                )
                .await;
            }
        }
    };
    if !resolved_build_images.is_empty() {
        for resolved in &resolved_build_images {
            let _ = sender
                .send(StackProgressItem::system(format!(
                    "Resolved service '{}' from Build{}.",
                    resolved.service_name,
                    resolved
                        .build_run_id
                        .map(|id| format!(" Run {id}"))
                        .unwrap_or_default()
                )))
                .await;
        }
        let override_path = ".citadel/citadel.build-images.yml".to_owned();
        source.files.push(StackSourceFile {
            relative_path: override_path.clone(),
            content: build_image_override(&resolved_build_images).into_bytes(),
        });
        source.compose_paths.push(override_path);
    }
    let model = match parse_compose(&compose_files) {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            report_stack_configuration_failure(alerts.as_ref(), &claim, &message).await;
            let persisted = store.fail_apply(actor, &claim, &message, false).await;
            let _ = sender
                .send(StackProgressItem::completed(
                    StackReleaseStatus::Failed,
                    message,
                ))
                .await;
            persisted?;
            return Err(error);
        }
    };
    if claim
        .service_names
        .iter()
        .any(|name| !model.services.iter().any(|service| service.name == *name))
    {
        return fail_apply_before_runtime(
            &store,
            actor,
            &claim,
            &sender,
            "A selected Service does not exist in the materialized Stack configuration.",
        )
        .await;
    }
    let release_source = match (&claim.spec, source.resolved_commit_sha.as_deref()) {
        (
            StackSpec::Git {
                git_repo_id,
                branch,
                commit_sha,
                compose_paths,
                working_directory,
                compose_env_files_from_repo,
                watch_paths,
                additional_env_file_from_repo,
                ..
            },
            Some(resolved_commit_sha),
        ) => Some(crate::StackReleaseSource {
            source_type: crate::StackSource::Git,
            git_repository_id: Some(*git_repo_id),
            git_repository_name: None,
            branch: Some(branch.clone()),
            requested_commit_sha: commit_sha.clone(),
            resolved_commit_sha: resolved_commit_sha.to_owned(),
            compose_paths: compose_paths.clone(),
            env_file_paths: if compose_env_files_from_repo.is_empty() {
                additional_env_file_from_repo.clone()
            } else {
                compose_env_files_from_repo.clone()
            },
            git_repository_url: None,
            working_directory: working_directory.clone().or_else(|| {
                compose_paths[0]
                    .rsplit_once('/')
                    .map(|(parent, _)| parent.to_owned())
            }),
            watch_paths: Some(watch_paths.clone()),
            compose_env_files_from_repo: Some(if compose_env_files_from_repo.is_empty() {
                additional_env_file_from_repo.clone()
            } else {
                compose_env_files_from_repo.clone()
            }),
            compose_digest: Some(compose_digest(&compose_files)),
        }),
        _ => None,
    };
    let names = model.variables.iter().cloned().collect::<Vec<_>>();
    let resolved = match bindings.resolve(claim.stack_id, &names).await {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            report_stack_configuration_failure(alerts.as_ref(), &claim, &message).await;
            let persisted = store.fail_apply(actor, &claim, &message, false).await;
            let _ = sender
                .send(StackProgressItem::completed(
                    StackReleaseStatus::Failed,
                    message,
                ))
                .await;
            persisted?;
            return Err(error);
        }
    };
    let (environment, redactions) = match resolve_environment(&model, &resolved) {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            report_stack_configuration_failure(alerts.as_ref(), &claim, &message).await;
            let persisted = store.fail_apply(actor, &claim, &message, false).await;
            let _ = sender
                .send(StackProgressItem::completed(
                    StackReleaseStatus::Failed,
                    message,
                ))
                .await;
            persisted?;
            return Err(error);
        }
    };
    let _ = sender
        .send(StackProgressItem::system(
            resolution_message::compose_resolution_message(
                &model,
                &resolved,
                source.env_file_paths.len(),
                &redactions,
            ),
        ))
        .await;
    let labels_override = match create_ownership_labels_override(
        &compose_files,
        claim.stack_id,
        claim.release_id,
        claim.platform_type == citadel_platforms::PlatformKind::DockerSwarm,
    ) {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            let persisted = store.fail_apply(actor, &claim, &message, false).await;
            let _ = sender
                .send(StackProgressItem::completed(
                    StackReleaseStatus::Failed,
                    message,
                ))
                .await;
            persisted?;
            return Err(error);
        }
    };
    let labels_override_path = ".citadel/citadel.labels.yml".to_owned();
    source.files.push(StackSourceFile {
        relative_path: labels_override_path.clone(),
        content: labels_override.into_bytes(),
    });
    source.labels_override_path = Some(labels_override_path);
    let _ = sender
        .send(StackProgressItem::system(
            "Submitting Stack deployment to Docker...",
        ))
        .await;
    if let Some(source) = release_source.as_ref()
        && let Err(error) = store.record_apply_source(&claim, source).await
    {
        return fail_apply_before_runtime(&store, actor, &claim, &sender, &error.to_string()).await;
    }
    let progress = StackProgress::new(sender.clone(), redactions.clone(), cancellation.clone());
    match tokio::time::timeout(
        timeout,
        runtime.apply(
            &claim,
            &source,
            &environment,
            &cancellation,
            Some(&progress),
        ),
    )
    .await
    {
        Ok(Ok(mut result)) => {
            redact_runtime_messages(&mut result, &redactions);
            let persistence = if result.status == StackReleaseStatus::Healthy {
                let applied_at = chrono::Utc::now();
                for binding in &mut claim.spec.common_mut().build_image_bindings {
                    if !claim.service_names.is_empty()
                        && !claim
                            .service_names
                            .iter()
                            .any(|name| name == &binding.service_name)
                    {
                        continue;
                    }
                    if let Some(resolved) = resolved_build_images.iter().find(|resolved| {
                        resolved
                            .service_name
                            .eq_ignore_ascii_case(&binding.service_name)
                    }) {
                        binding.record_applied(resolved, applied_at);
                    }
                }
                let snapshots = resolved
                    .entries
                    .iter()
                    .map(|entry| entry.snapshot.clone())
                    .collect::<Vec<_>>();
                // Retry only completion persistence, preserving the source,
                // bindings and build provenance of the successful Docker run.
                for attempt in 0..3 {
                    match store
                        .complete_apply(actor, &claim, &result, &snapshots, release_source.as_ref())
                        .await
                    {
                        Ok(()) => break,
                        Err(StackError::Storage(_)) if attempt < 2 => {
                            tokio::time::sleep(Duration::from_millis(100 * (attempt + 1))).await;
                        }
                        Err(error) => {
                            let _ = sender.send(StackProgressItem::completed(
                                StackReleaseStatus::Unknown,
                                format!("Docker completed the Stack deployment, but saving its result failed. Recovery will check the deployed Stack: {error}"),
                            )).await;
                            notifier.changed(claim.stack_id, "outcomeUnknown");
                            return Err(error);
                        }
                    }
                }
                notifier.changed(claim.stack_id, "applied");
                Ok(())
            } else {
                let message = result
                    .messages
                    .last()
                    .and_then(|item| item.message.as_deref())
                    .unwrap_or("Stack deployment failed.");
                let persistence = store.fail_apply(actor, &claim, message, false).await;
                notifier.changed(claim.stack_id, "failed");
                persistence
            };
            let _ = sender
                .send(StackProgressItem::completed(
                    result.status,
                    if result.status == StackReleaseStatus::Healthy {
                        "Stack deployment completed."
                    } else {
                        "Stack deployment failed."
                    },
                ))
                .await;
            persistence?;
            if result.status != StackReleaseStatus::Healthy {
                return Err(StackError::Runtime("Stack deployment failed.".to_owned()));
            }
        }
        Ok(Err(error)) => {
            let unknown = matches!(error, StackError::Runtime(_) | StackError::Cancelled);
            let message = redact(error.to_string(), &redactions);
            let persisted = store.fail_apply(actor, &claim, &message, unknown).await;
            notifier.changed(
                claim.stack_id,
                if unknown { "outcomeUnknown" } else { "failed" },
            );
            let _ = sender
                .send(StackProgressItem::completed(
                    StackReleaseStatus::Failed,
                    message.clone(),
                ))
                .await;
            persisted?;
            return Err(StackError::Runtime(message));
        }
        Err(_) => {
            let message = "Stack operation timed out before its Docker outcome could be confirmed.";
            let persisted = store.fail_apply(actor, &claim, message, true).await;
            notifier.changed(claim.stack_id, "outcomeUnknown");
            let _ = sender
                .send(StackProgressItem::completed(
                    StackReleaseStatus::TimedOut,
                    message,
                ))
                .await;
            persisted?;
            return Err(StackError::Runtime(message.to_owned()));
        }
    }
    Ok(())
}

pub(super) fn build_image_override(bindings: &[ResolvedStackBuildImageBinding]) -> String {
    let mut output = String::from("services:\n");
    for binding in bindings {
        output.push_str("  ");
        output.push_str(&yaml_quote(&binding.service_name));
        output.push_str(":\n    image: ");
        output.push_str(&yaml_quote(&binding.image_reference));
        output.push('\n');
    }
    output
}

pub(super) fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

pub(super) async fn fail_apply_before_runtime(
    store: &Arc<dyn StackRepository>,
    actor: ActorId,
    claim: &StackOperationClaim,
    sender: &mpsc::Sender<StackProgressItem>,
    message: &str,
) -> Result<(), StackError> {
    let persisted = store.fail_apply(actor, claim, message, false).await;
    let _ = sender
        .send(StackProgressItem::completed(
            StackReleaseStatus::Failed,
            message,
        ))
        .await;
    persisted?;
    Err(StackError::Runtime(message.to_owned()))
}

pub(super) async fn report_stack_configuration_failure(
    alerts: Option<&Arc<dyn AlertEventSink>>,
    claim: &StackOperationClaim,
    message: &str,
) {
    let Some(alerts) = alerts else { return };
    let observation = AlertObservation {
        alert_type: "StackConfigurationResolutionFailed".to_owned(),
        info: serde_json::json!({
            "StackId": claim.stack_id,
            "StackName": claim.name,
            "Reason": message,
            "HumanMessage": message,
        }),
        resource_id: claim.stack_id,
        resource_name: claim.name.clone(),
        resource_type: "Stack".to_owned(),
        deduplication_component: claim.release_id.to_string(),
        observed_at: chrono::Utc::now(),
        value: None,
        matched: true,
    };
    if let Err(error) = alerts.observe(&observation).await {
        tracing::warn!(%error, stack_id=%claim.stack_id, "Stack configuration Alert evaluation failed");
    }
}
