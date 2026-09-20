use super::*;
impl SwarmServiceService {
    pub(super) async fn report_operation_observation(
        &self,
        claim: &ServiceOperationClaim,
        operation: &str,
        message: &str,
        matched: bool,
    ) {
        let Some(alerts) = &self.alerts else {
            return;
        };
        let _ = alerts
            .observe(&AlertObservation {
                alert_type: "SwarmServiceOperationFailed".into(),
                info: serde_json::json!({
                    "HumanMessage": message,
                    "OperationKind": operation,
                    "ServiceName": claim.docker_name,
                    "Reason": message,
                    "OperationId": claim.operation_id,
                }),
                resource_id: claim.id,
                resource_name: claim.docker_name.clone(),
                resource_type: "SwarmService".into(),
                deduplication_component: claim.operation_id.simple().to_string(),
                observed_at: chrono::Utc::now(),
                value: None,
                matched,
            })
            .await;
    }

    pub fn apply(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> mpsc::Receiver<SwarmServiceProgressItem> {
        self.run_operation(
            actor_id,
            administrator,
            id,
            ServiceOperationKind::Apply,
            None,
        )
    }

    pub(super) fn run_operation(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        kind: ServiceOperationKind,
        replicas: Option<i32>,
    ) -> mpsc::Receiver<SwarmServiceProgressItem> {
        self.run_requested_operation(
            actor_id,
            administrator,
            ServiceOperationRequest {
                id,
                kind,
                replicas,
                expected_version: None,
            },
        )
    }

    pub(super) fn run_requested_operation(
        &self,
        actor_id: ActorId,
        administrator: bool,
        request: ServiceOperationRequest,
    ) -> mpsc::Receiver<SwarmServiceProgressItem> {
        let (sender, receiver) = mpsc::channel(32);
        // Reject overload before spawning: a semaphore wait inside each spawned
        // task would retain an unbounded number of requests.
        let Ok(permit) = self.slots.clone().try_acquire_owned() else {
            let _ = sender.try_send(SwarmServiceProgressItem::failed(
                request.id,
                None,
                "Service operations are busy. Try again shortly.",
            ));
            return receiver;
        };
        let service = self.clone();
        let rejected_sender = sender.clone();
        let id = request.id;
        if !self.tasks.spawn(
            "swarm_service.operation",
            Box::pin(async move {
                let _permit = permit;
                service
                    .execute_operation(sender, actor_id, administrator, request)
                    .await
            }),
        ) {
            let _ = rejected_sender.try_send(SwarmServiceProgressItem::failed(
                id,
                None,
                "Server is shutting down.",
            ));
        }
        receiver
    }

    pub(super) async fn execute_operation(
        &self,
        sender: mpsc::Sender<SwarmServiceProgressItem>,
        actor_id: ActorId,
        administrator: bool,
        request: ServiceOperationRequest,
    ) -> Result<(), SwarmServiceError> {
        let ServiceOperationRequest {
            id, kind, replicas, ..
        } = request;
        if kind == ServiceOperationKind::Scale && replicas.is_some_and(|value| value < 0) {
            let _ = sender
                .send(SwarmServiceProgressItem::failed(
                    id,
                    None,
                    "Replica count cannot be negative.",
                ))
                .await;
            return Err(SwarmServiceError::Validation(
                "Replica count cannot be negative.".into(),
            ));
        }
        let claim = match self
            .store
            .claim_operation(actor_id, administrator, request)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                let _ = sender
                    .send(SwarmServiceProgressItem::failed(
                        id,
                        None,
                        error.to_string(),
                    ))
                    .await;
                return Err(error);
            }
        };
        let _ = sender
            .send(SwarmServiceProgressItem::info(
                id,
                Some(claim.operation_id),
                "Prepared",
                format!("{} operation prepared.", kind.as_str()),
            ))
            .await;
        let _ = sender
            .send(SwarmServiceProgressItem::info(
                id,
                Some(claim.operation_id),
                "Bindings",
                "Resolving Service variables and secrets...",
            ))
            .await;
        let (runtime_environment, redaction_values, binding_message) =
            match self.resolve_environment(&claim).await {
                Ok(value) => value,
                Err(error) => {
                    let message = error.to_string();
                    let persisted = self
                        .store
                        .fail_operation(actor_id, &claim, &message, false)
                        .await;
                    self.report_operation_observation(&claim, kind.as_str(), &message, true)
                        .await;
                    let _ = sender
                        .send(SwarmServiceProgressItem::failed(
                            id,
                            Some(claim.operation_id),
                            message,
                        ))
                        .await;
                    persisted?;
                    return Err(error);
                }
            };
        let _ = sender
            .send(SwarmServiceProgressItem::info(
                id,
                Some(claim.operation_id),
                "Bindings",
                binding_message,
            ))
            .await;
        if let Err(error) = self.store.mark_attempted(&claim).await {
            let _ = sender
                .send(SwarmServiceProgressItem::failed(
                    id,
                    Some(claim.operation_id),
                    error.to_string(),
                ))
                .await;
            return Err(error);
        }
        let mut runtime_claim = claim.clone();
        runtime_claim.spec.environment = runtime_environment;
        let cancellation = self.shutdown.child_token();
        let future = match (kind, replicas) {
            (ServiceOperationKind::Apply, _) => self.runtime.apply(&runtime_claim, &cancellation),
            (ServiceOperationKind::Scale, Some(replicas)) => {
                self.runtime.scale(&runtime_claim, replicas, &cancellation)
            }
            (ServiceOperationKind::ForceUpdate, _) => {
                self.runtime.force_update(&runtime_claim, &cancellation)
            }
            _ => unreachable!(),
        };
        let result = tokio::time::timeout(self.operation_timeout, future).await;
        match result {
            Ok(Ok(result)) if result.rollout_complete && result.rollout_error.is_none() => {
                if let Err(error) = self
                    .store
                    .complete_operation(actor_id, &claim, &result)
                    .await
                {
                    let _ = sender
                        .send(SwarmServiceProgressItem::failed(
                            id,
                            Some(claim.operation_id),
                            error.to_string(),
                        ))
                        .await;
                    return Err(error);
                }
                self.notifier.changed(id, "applied");
                self.report_operation_observation(
                    &claim,
                    kind.as_str(),
                    "The Service operation completed successfully.",
                    false,
                )
                .await;
                let _ = sender
                    .send(SwarmServiceProgressItem::completed(
                        id,
                        claim.operation_id,
                        "Service rollout completed.",
                    ))
                    .await;
            }
            Ok(Ok(result)) if result.accepted && result.rollout_error.is_none() => {
                if let Err(error) = self.store.mark_accepted(&claim, &result).await {
                    let _ = sender
                        .send(SwarmServiceProgressItem::failed(
                            id,
                            Some(claim.operation_id),
                            error.to_string(),
                        ))
                        .await;
                    return Err(error);
                }
                self.notifier.changed(id, "accepted");
                let mut item = SwarmServiceProgressItem::completed(
                    id,
                    claim.operation_id,
                    "Docker accepted the Service; rollout observation continues asynchronously.",
                );
                item.is_warning = !result.warnings.is_empty();
                let _ = sender.send(item).await;
            }
            Ok(Ok(result)) => {
                let message = result.rollout_error.as_deref().unwrap_or(
                    "Docker accepted the Service operation, but rollout did not complete.",
                );
                let persisted = self
                    .store
                    .fail_operation(actor_id, &claim, message, false)
                    .await;
                self.report_operation_observation(&claim, kind.as_str(), message, true)
                    .await;
                self.notifier.changed(id, "failed");
                let _ = sender
                    .send(SwarmServiceProgressItem::failed(
                        id,
                        Some(claim.operation_id),
                        message.to_owned(),
                    ))
                    .await;
                persisted?;
                return Err(SwarmServiceError::Runtime(message.to_owned()));
            }
            Ok(Err(error)) => {
                let message = redact(error.to_string(), &redaction_values);
                let unknown = matches!(
                    error,
                    SwarmServiceError::Runtime(_) | SwarmServiceError::Cancelled
                );
                let persisted = self
                    .store
                    .fail_operation(actor_id, &claim, &message, unknown)
                    .await;
                self.report_operation_observation(&claim, kind.as_str(), &message, true)
                    .await;
                self.notifier.changed(id, "failed");
                let _ = sender
                    .send(SwarmServiceProgressItem::failed(
                        id,
                        Some(claim.operation_id),
                        message.to_owned(),
                    ))
                    .await;
                persisted?;
                return Err(SwarmServiceError::Runtime(message.to_owned()));
            }
            Err(_) => {
                let message =
                    "Service operation timed out before its Docker outcome could be confirmed.";
                let persisted = self
                    .store
                    .fail_operation(actor_id, &claim, message, true)
                    .await;
                self.report_operation_observation(&claim, kind.as_str(), message, true)
                    .await;
                self.notifier.changed(id, "outcomeUnknown");
                let _ = sender
                    .send(SwarmServiceProgressItem::failed(
                        id,
                        Some(claim.operation_id),
                        message.to_owned(),
                    ))
                    .await;
                persisted?;
                return Err(SwarmServiceError::Runtime(message.to_owned()));
            }
        }
        Ok(())
    }

    pub async fn reconcile_stale_operations(
        &self,
        older_than: Duration,
        limit: i64,
    ) -> Result<usize, SwarmServiceError> {
        let recovered_checks = self
            .store
            .recover_update_checks(chrono::Utc::now().timestamp() - 60, limit)
            .await?;
        for id in recovered_checks {
            self.notifier.changed(id, "update");
        }
        let cutoff = chrono::Utc::now().timestamp()
            - i64::try_from(older_than.as_secs()).unwrap_or(i64::MAX);
        let claims = self
            .store
            .stale_operation_claims(cutoff, limit.clamp(1, 100))
            .await?;
        let mut reconciled = 0;
        // Deletion outcomes are reconciled from the complete manager snapshot.
        // Never replay an ambiguous daemon mutation from a recovery timer.
        for (actor_id, claim) in claims {
            let cancellation = self.shutdown.child_token();
            match tokio::time::timeout(
                self.operation_timeout.min(Duration::from_secs(30)),
                self.runtime.observe(&claim, &cancellation),
            )
            .await
            {
                Ok(Ok(Some(result)))
                    if result.rollout_complete && result.rollout_error.is_none() =>
                {
                    self.store
                        .complete_operation(actor_id, &claim, &result)
                        .await?;
                    self.report_operation_observation(
                        &claim,
                        "Reconciliation",
                        "The Service rollout completed successfully.",
                        false,
                    )
                    .await;
                    self.notifier.changed(claim.id, "reconciled");
                    reconciled += 1;
                }
                Ok(Ok(Some(result))) if result.rollout_error.is_some() => {
                    self.store
                        .fail_operation(
                            actor_id,
                            &claim,
                            result.rollout_error.as_deref().unwrap(),
                            false,
                        )
                        .await?;
                    self.report_operation_observation(
                        &claim,
                        "Reconciliation",
                        result.rollout_error.as_deref().unwrap(),
                        true,
                    )
                    .await;
                    self.notifier.changed(claim.id, "failed");
                    reconciled += 1;
                }
                _ => {}
            }
        }
        Ok(reconciled)
    }
}
