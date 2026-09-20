use super::*;

impl DeploymentService {
    pub async fn apply(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        recreate: bool,
    ) -> Result<mpsc::Receiver<DeploymentProgress>, DeploymentError> {
        self.apply_versioned(actor_id, administrator, id, recreate, None)
            .await
    }

    pub async fn apply_versioned(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        recreate: bool,
        expected_version: Option<i64>,
    ) -> Result<mpsc::Receiver<DeploymentProgress>, DeploymentError> {
        if id.is_nil() {
            return Err(DeploymentError::Validation(
                "A Deployment must be selected.".to_owned(),
            ));
        }
        let permit = tokio::time::timeout(
            Duration::from_secs(30),
            Arc::clone(&self.apply_slots).acquire_owned(),
        )
        .await
        .map_err(|_| {
            DeploymentError::Conflict(
                "Deployment Apply capacity is busy. Try again shortly.".to_owned(),
            )
        })?
        .map_err(|_| DeploymentError::Runtime("Deployment Apply is shutting down.".to_owned()))?;
        let claim = self
            .store
            .claim_apply_versioned(actor_id, administrator, id, expected_version)
            .await?;
        self.notifier.changed(id, "updated");
        let (sender, receiver) = mpsc::channel(32);
        let operation = ApplyOperation {
            actor_id,
            recreate,
            claim: claim.clone(),
            store: Arc::clone(&self.store),
            runtime: Arc::clone(&self.runtime),
            bindings: Arc::clone(&self.bindings),
            notifier: Arc::clone(&self.notifier),
            alerts: self.alerts.clone(),
            sender,
            _permit: permit,
            runtime_applied: false,
        };
        let shutdown = self.shutdown.clone();
        let timeout = self.apply_timeout;
        if !self.tasks.spawn(
            "deployment.apply",
            Box::pin(async move { operation.run(&shutdown, timeout).await }),
        ) {
            self.store
                .fail_apply(
                    actor_id,
                    &claim,
                    "Deployment Apply is shutting down.",
                    None,
                    &[],
                )
                .await?;
            self.notifier.changed(id, "updated");
            return Err(DeploymentError::Cancelled);
        }
        Ok(receiver)
    }

    pub async fn reconcile_stale_applies(
        &self,
        started_before: i64,
        limit: i64,
    ) -> Result<usize, DeploymentError> {
        let claims = self
            .store
            .stale_apply_claims(started_before, limit.clamp(1, 100))
            .await?;
        let mut reconciled = 0;
        for (actor_id, claim) in claims {
            let cancellation = self.shutdown.child_token();
            let observed = self
                .runtime
                .observe_deployment(claim.platform_id, claim.id, &cancellation)
                .await;
            cancellation.cancel();
            match observed {
                Ok(Some(result)) if result.state == RuntimeContainerState::Running => {
                    self.store
                        .complete_apply(actor_id, &claim, &result, None, &[])
                        .await?;
                }
                Ok(Some(result)) => {
                    let message = "Deployment Apply was interrupted and the recovered container is not running.";
                    self.store
                        .fail_apply(actor_id, &claim, message, Some(&result), &[])
                        .await?;
                }
                Err(_) => continue, // An unavailable runtime is not evidence of a failed Apply.
                Ok(None) => {
                    let message = "Deployment Apply was interrupted and its runtime outcome could not be confirmed.";
                    self.store
                        .fail_apply(actor_id, &claim, message, None, &[])
                        .await?;
                }
            }
            self.notifier.changed(claim.id, "updated");
            reconciled += 1;
        }
        Ok(reconciled)
    }
}
struct ApplyOperation {
    actor_id: ActorId,
    recreate: bool,
    claim: ApplyClaim,
    store: Arc<dyn DeploymentRepository>,
    runtime: Arc<dyn DeploymentRuntime>,
    bindings: Arc<dyn DeploymentBindingResolverPort>,
    notifier: Arc<dyn DeploymentChangeNotifier>,
    alerts: Option<Arc<dyn AlertEventSink>>,
    sender: mpsc::Sender<DeploymentProgress>,
    _permit: OwnedSemaphorePermit,
    runtime_applied: bool,
}

impl ApplyOperation {
    async fn run(
        mut self,
        shutdown: &CancellationToken,
        timeout: Duration,
    ) -> Result<(), DeploymentError> {
        let cancellation = shutdown.child_token();
        let result = tokio::select! {
            biased;
            () = shutdown.cancelled() => Err(DeploymentError::Cancelled),
            result = tokio::time::timeout(timeout, self.execute_inner(&cancellation)) => {
                result.unwrap_or_else(|_| Err(DeploymentError::Runtime(
                    "Deployment Apply timed out.".to_owned(),
                )))
            }
        };
        cancellation.cancel();
        if let Err(error) = result {
            if self.runtime_applied {
                self.send(DeploymentProgress::failure(
                    deployment_error_code(&error),
                    format!("Container is running, but saving the deployment result failed. Recovery will check the running container: {error}"),
                ));
                self.notifier.changed(self.claim.id, "updated");
                // Keep successful Docker work out of the failure path. The
                // existing recovery worker owns the still-pending claim.
                return Err(error);
            }
            if matches!(error, DeploymentError::Cancelled)
                || matches!(&error, DeploymentError::Runtime(message) if message == "Deployment Apply timed out.")
            {
                self.send(DeploymentProgress::failure(
                    deployment_error_code(&error),
                    error.to_string(),
                ));
                self.notifier.changed(self.claim.id, "updated");
                return Err(error);
            }
            let message = error.to_string();
            let persisted = self
                .store
                .fail_apply(self.actor_id, &self.claim, &message, None, &[])
                .await;
            let final_error = persisted.err().unwrap_or(error);
            self.send(DeploymentProgress::failure(
                deployment_error_code(&final_error),
                final_error.to_string(),
            ));
            self.notifier.changed(self.claim.id, "updated");
            return Err(final_error);
        }
        Ok(())
    }

    async fn execute_inner(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<(), DeploymentError> {
        match &self.claim.spec.image {
            DeploymentImageInfo::External { image_tag, .. } => {
                self.send(DeploymentProgress::info(format!(
                    "Pulling image {image_tag}"
                )));
            }
            DeploymentImageInfo::Local { .. } | DeploymentImageInfo::Build { .. } => {}
        }
        let image = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
            image = self.runtime.prepare_image(
                self.claim.platform_id,
                &self.claim.spec.image,
                cancellation,
            ) => image?,
        };
        apply_resolved_build(&mut self.claim.spec.image, &image)?;

        if self.recreate
            && let Some(container_id) = self.claim.existing_docker_container_id.as_deref()
        {
            self.runtime
                .delete_container(self.claim.platform_id, container_id, cancellation)
                .await?;
            self.send(DeploymentProgress::info(format!(
                "Container deleted: {container_id}"
            )));
        }

        self.send(DeploymentProgress::info(
            "Resolving deployment variables and secrets...",
        ));
        let configured_environment = self
            .claim
            .spec
            .environment_variables
            .as_deref()
            .unwrap_or_default();
        let referenced = match referenced_binding_names(configured_environment) {
            Ok(referenced) => referenced,
            Err(error) => {
                self.report_configuration_failure(&error.to_string()).await;
                return Err(error);
            }
        };
        let resolved = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
            resolved = self.bindings.resolve(self.claim.id, &referenced) => match resolved {
                Ok(resolved) => resolved,
                Err(error) => {
                    self.report_configuration_failure(&error.to_string()).await;
                    return Err(error);
                }
            },
        };
        let environment = match build_environment(configured_environment, &referenced, &resolved) {
            Ok(environment) => environment,
            Err(error) => {
                self.report_configuration_failure(&error.to_string()).await;
                return Err(error);
            }
        };
        self.send(DeploymentProgress::info(binding_message(&resolved)));
        self.send(DeploymentProgress::info(format!(
            "Applying deployment to {}...",
            self.claim.platform_address
        )));

        let command = RuntimeDeploymentCommand {
            deployment_id: self.claim.id,
            name: self.claim.name.clone(),
            image_id: image.docker_image_id.clone(),
            spec: self.claim.spec.clone(),
            environment_variables: environment.values,
        };
        let runtime_result = self
            .runtime
            .apply_container(self.claim.platform_id, &command, cancellation)
            .await
            .map_err(|error| redact_error(error, &environment.redaction_values))?;
        self.send(DeploymentProgress::info(format!(
            "Container created: {}",
            runtime_result.docker_container_id
        )));
        if runtime_result.state != RuntimeContainerState::Running {
            let message = format!(
                "Deployment failed: container did not start successfully - Container state: {:?}",
                runtime_result.state
            );
            self.store
                .fail_apply(
                    self.actor_id,
                    &self.claim,
                    &message,
                    Some(&runtime_result),
                    &environment.snapshots,
                )
                .await?;
            self.send(DeploymentProgress::failure(422, message));
            self.notifier.changed(self.claim.id, "updated");
            return Ok(());
        }
        self.runtime_applied = true;
        // Retry only the database transaction, with the original result, digest
        // and binding snapshots. Docker has already succeeded.
        for attempt in 0..3 {
            match self
                .store
                .complete_apply(
                    self.actor_id,
                    &self.claim,
                    &runtime_result,
                    image.digest.as_deref(),
                    &environment.snapshots,
                )
                .await
            {
                Ok(()) => break,
                Err(DeploymentError::Storage(_)) if attempt < 2 => {
                    tokio::time::sleep(Duration::from_millis(100 * (attempt + 1))).await;
                }
                Err(error) => return Err(error),
            }
        }
        self.notifier.changed(self.claim.id, "updated");
        self.send(DeploymentProgress::info("Deployment is now running."));
        Ok(())
    }

    async fn report_configuration_failure(&self, message: &str) {
        let Some(alerts) = &self.alerts else { return };
        let observation = AlertObservation {
            alert_type: "DeploymentConfigurationResolutionFailed".to_owned(),
            info: serde_json::json!({
                "DeploymentId": self.claim.id,
                "DeploymentName": &self.claim.name,
                "Reason": message,
                "HumanMessage": message,
            }),
            resource_id: self.claim.id,
            resource_name: self.claim.name.clone(),
            resource_type: "Deployment".to_owned(),
            deduplication_component: self.claim.row_version.to_string(),
            observed_at: chrono::Utc::now(),
            value: None,
            matched: true,
        };
        let _ = alerts.observe(&observation).await;
    }

    fn send(&self, item: DeploymentProgress) {
        let _ = self.sender.try_send(item);
    }
}

pub(super) fn apply_resolved_build(
    image: &mut DeploymentImageInfo,
    prepared: &crate::PreparedDeploymentImage,
) -> Result<(), DeploymentError> {
    match (&*image, prepared.resolved_build.as_ref()) {
        (
            DeploymentImageInfo::Build {
                build_project_id,
                redeploy_on_build,
                ..
            },
            Some(build),
        ) => {
            *image = DeploymentImageInfo::Build {
                build_project_id: *build_project_id,
                redeploy_on_build: *redeploy_on_build,
                resolved_image_reference: Some(build.image_reference.clone()),
                resolved_digest: build.digest.clone(),
                resolved_build_run_id: Some(build.build_run_id),
                applied_image_reference: Some(build.image_reference.clone()),
                applied_digest: prepared.digest.clone().or_else(|| build.digest.clone()),
                applied_build_run_id: Some(build.build_run_id),
                applied_at: Some(chrono::Utc::now()),
            };
            Ok(())
        }
        (DeploymentImageInfo::Build { .. }, None) => Err(DeploymentError::Runtime(
            "Build-backed Deployment image preparation returned no Build provenance.".to_owned(),
        )),
        (_, Some(_)) => Err(DeploymentError::Runtime(
            "Docker returned Build provenance for a non-Build Deployment.".to_owned(),
        )),
        (_, None) => Ok(()),
    }
}

pub(super) fn deployment_error_code(error: &DeploymentError) -> i64 {
    match error {
        DeploymentError::Validation(_) => 400,
        DeploymentError::NotFound => 404,
        DeploymentError::Forbidden => 403,
        DeploymentError::Conflict(_) => 409,
        DeploymentError::LicenseRequired(_) => 403,
        DeploymentError::Runtime(_) | DeploymentError::Storage(_) | DeploymentError::Cancelled => {
            500
        }
    }
}
