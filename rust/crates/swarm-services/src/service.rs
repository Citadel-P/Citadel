use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use tokio::sync::{Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::{
    CreateSwarmServiceInput, ManagedSwarmServiceView, ManagedSwarmServicesView,
    RenameSwarmServiceInput, ResourceCapabilities, RuntimeServiceResult, ServiceDeletionClaim,
    ServiceOperationClaim, ServiceOperationKind, SwarmServiceError, SwarmServiceFilter,
    SwarmServiceProgressItem, UpdateSwarmServiceInput,
};

#[path = "updates.rs"]
mod updates;
pub use updates::*;
#[path = "jobs/image_updates.rs"]
mod image_updates;

/// The optional version fences an automated action to the configuration that
/// was authenticated and checked, without changing ordinary interactive Apply.
#[derive(Clone, Copy)]
pub struct ServiceOperationRequest {
    pub id: Uuid,
    pub kind: ServiceOperationKind,
    pub replicas: Option<i32>,
    pub expected_version: Option<i64>,
}

pub trait SwarmServiceStore: Send + Sync {
    fn update_check_candidates(
        &self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, SwarmServiceError>>;
    fn begin_update_check<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a ManagedSwarmServiceView,
    ) -> BoxFuture<'a, Result<ServiceUpdateCheck, SwarmServiceError>>;
    fn complete_update_check<'a>(
        &'a self,
        claim: &'a ServiceUpdateCheck,
        state: Option<&'a crate::AutoUpdateState>,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn recover_update_checks(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, SwarmServiceError>>;
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        filter: &'a SwarmServiceFilter,
    ) -> BoxFuture<'a, Result<Vec<ManagedSwarmServiceView>, SwarmServiceError>>;
    fn get_authorized(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<ManagedSwarmServiceView, SwarmServiceError>>;
    fn create<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a CreateSwarmServiceInput,
    ) -> BoxFuture<'a, Result<ManagedSwarmServiceView, SwarmServiceError>>;
    fn update<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: &'a UpdateSwarmServiceInput,
    ) -> BoxFuture<'a, Result<ManagedSwarmServiceView, SwarmServiceError>>;
    fn rename<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a RenameSwarmServiceInput,
    ) -> BoxFuture<'a, Result<ManagedSwarmServiceView, SwarmServiceError>>;
    fn delete<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<ServiceDeletionClaim>, SwarmServiceError>>;
    fn complete_delete<'a>(
        &'a self,
        actor_id: ActorId,
        deleted: &'a [ServiceDeletionClaim],
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn release_delete<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn stale_deletion_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceDeletionClaim)>, SwarmServiceError>>;
    fn claim_operation(
        &self,
        actor_id: ActorId,
        administrator: bool,
        request: ServiceOperationRequest,
    ) -> BoxFuture<'_, Result<ServiceOperationClaim, SwarmServiceError>>;
    fn mark_attempted<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn mark_accepted<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        result: &'a RuntimeServiceResult,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn complete_operation<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ServiceOperationClaim,
        result: &'a RuntimeServiceResult,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn fail_operation<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ServiceOperationClaim,
        message: &'a str,
        outcome_unknown: bool,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn stale_operation_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceOperationClaim)>, SwarmServiceError>>;
}

pub trait SwarmServiceRuntimePort: Send + Sync {
    fn apply<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>>;
    fn scale<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        replicas: i32,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>>;
    fn force_update<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>>;
    fn delete<'a>(
        &'a self,
        platform_id: Uuid,
        docker_service_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn observe<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<RuntimeServiceResult>, SwarmServiceError>>;
}

#[derive(Debug)]
pub struct ResolvedSwarmServiceBinding {
    pub name: String,
    pub value: Zeroizing<String>,
    pub secret: bool,
}

#[derive(Debug, Default)]
pub struct ResolvedSwarmServiceBindings {
    pub entries: Vec<ResolvedSwarmServiceBinding>,
}

pub trait SwarmServiceBindingResolverPort: Send + Sync {
    fn resolve<'a>(
        &'a self,
        service_id: Uuid,
        referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedSwarmServiceBindings, SwarmServiceError>>;
}

#[derive(Default)]
pub struct EmptySwarmServiceBindingResolver;
impl SwarmServiceBindingResolverPort for EmptySwarmServiceBindingResolver {
    fn resolve<'a>(
        &'a self,
        _service_id: Uuid,
        _referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedSwarmServiceBindings, SwarmServiceError>> {
        Box::pin(async { Ok(ResolvedSwarmServiceBindings::default()) })
    }
}

pub trait SwarmServiceChangeNotifier: Send + Sync {
    fn changed(&self, service_id: Uuid, event: &'static str);
}

#[derive(Default)]
pub struct NoopSwarmServiceChangeNotifier;
impl SwarmServiceChangeNotifier for NoopSwarmServiceChangeNotifier {
    fn changed(&self, _service_id: Uuid, _event: &'static str) {}
}

#[derive(Clone)]
pub struct ManagedSwarmServiceService {
    store: Arc<dyn SwarmServiceStore>,
    runtime: Arc<dyn SwarmServiceRuntimePort>,
    notifier: Arc<dyn SwarmServiceChangeNotifier>,
    bindings: Arc<dyn SwarmServiceBindingResolverPort>,
    shutdown: CancellationToken,
    operation_timeout: Duration,
    slots: Arc<Semaphore>,
    alerts: Option<Arc<dyn AlertEventSink>>,
    image_digests: Option<Arc<dyn ServiceImageDigestPort>>,
    entitlements: Option<Arc<dyn ServiceAutomationEntitlements>>,
}

impl ManagedSwarmServiceService {
    #[must_use]
    pub fn new(
        store: Arc<dyn SwarmServiceStore>,
        runtime: Arc<dyn SwarmServiceRuntimePort>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            store,
            runtime,
            notifier: Arc::new(NoopSwarmServiceChangeNotifier),
            bindings: Arc::new(EmptySwarmServiceBindingResolver),
            shutdown,
            operation_timeout: Duration::from_secs(10 * 60),
            slots: Arc::new(Semaphore::new(4)),
            alerts: None,
            image_digests: None,
            entitlements: None,
        }
    }

    #[must_use]
    pub fn with_alerts(mut self, alerts: Arc<dyn AlertEventSink>) -> Self {
        self.alerts = Some(alerts);
        self
    }

    async fn report_operation_observation(
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
                    "Operation": operation,
                    "OperationId": claim.operation_id,
                }),
                resource_id: claim.id,
                resource_name: claim.docker_name.clone(),
                resource_type: "SwarmService".into(),
                deduplication_component: "operation".into(),
                observed_at: chrono::Utc::now(),
                value: None,
                matched,
            })
            .await;
    }

    #[must_use]
    pub fn with_notifier(mut self, notifier: Arc<dyn SwarmServiceChangeNotifier>) -> Self {
        self.notifier = notifier;
        self
    }

    #[must_use]
    pub fn with_binding_resolver(
        mut self,
        bindings: Arc<dyn SwarmServiceBindingResolverPort>,
    ) -> Self {
        self.bindings = bindings;
        self
    }

    #[must_use]
    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    pub async fn list(
        &self,
        actor_id: ActorId,
        administrator: bool,
        filter: &SwarmServiceFilter,
        capabilities: ResourceCapabilities,
    ) -> Result<ManagedSwarmServicesView, SwarmServiceError> {
        Ok(ManagedSwarmServicesView {
            swarm_services: self
                .store
                .list_authorized(actor_id, administrator, filter)
                .await?,
            capabilities,
        })
    }

    pub async fn get(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<ManagedSwarmServiceView, SwarmServiceError> {
        self.store.get_authorized(actor_id, administrator, id).await
    }

    pub async fn create(
        &self,
        actor_id: ActorId,
        administrator: bool,
        mut input: CreateSwarmServiceInput,
    ) -> Result<ManagedSwarmServiceView, SwarmServiceError> {
        if input.duplicate_source.is_some() {
            return Err(validation(
                "Managed Swarm Service duplication has not migrated to Rust yet.",
            ));
        }
        normalize_name(&mut input.name)?;
        normalize_description(&mut input.description)?;
        if input.platform_id.is_nil() {
            return Err(validation("A Platform must be selected."));
        }
        if input.tag_ids.len() > 100 {
            return Err(validation("A Service cannot contain more than 100 Tags."));
        }
        input.tag_ids = unique_ids(&input.tag_ids);
        input.spec = input.spec.for_create();
        input.spec.validate()?;
        let created = self.store.create(actor_id, administrator, &input).await?;
        self.notifier.changed(created.id, "created");
        Ok(created)
    }

    pub async fn update(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: UpdateSwarmServiceInput,
    ) -> Result<ManagedSwarmServiceView, SwarmServiceError> {
        input.spec.validate()?;
        let current = self
            .store
            .get_authorized(actor_id, administrator, id)
            .await?;
        if current.spec.scheduling_mode != input.spec.scheduling_mode
            && current.applied_image_digest.is_some()
        {
            return Err(validation(
                "Service scheduling mode cannot change after the first successful Apply.",
            ));
        }
        let updated = self
            .store
            .update(actor_id, administrator, id, &input)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn rename(
        &self,
        actor_id: ActorId,
        administrator: bool,
        mut input: RenameSwarmServiceInput,
    ) -> Result<ManagedSwarmServiceView, SwarmServiceError> {
        normalize_name(&mut input.name)?;
        let updated = self.store.rename(actor_id, administrator, &input).await?;
        self.notifier.changed(input.id, "renamed");
        Ok(updated)
    }

    pub async fn delete(
        &self,
        actor_id: ActorId,
        administrator: bool,
        ids: &[Uuid],
    ) -> Result<(), SwarmServiceError> {
        let ids = unique_ids(ids);
        if ids.is_empty() {
            return Err(validation("Ids must not be empty."));
        }
        let claims = self.store.delete(actor_id, administrator, &ids).await?;
        let cancellation = self.shutdown.child_token();
        for claim in &claims {
            if let Some(docker_id) = &claim.docker_service_id
                && let Err(error) = self
                    .runtime
                    .delete(claim.platform_id, docker_id, &cancellation)
                    .await
            {
                if !matches!(
                    error,
                    SwarmServiceError::Runtime(_) | SwarmServiceError::Cancelled
                ) {
                    self.store.release_delete(&ids).await?;
                }
                return Err(error);
            }
        }
        self.store.complete_delete(actor_id, &claims).await?;
        for id in ids {
            self.notifier.changed(id, "deleted");
        }
        Ok(())
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
    pub fn scale(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        replicas: i32,
    ) -> mpsc::Receiver<SwarmServiceProgressItem> {
        self.run_operation(
            actor_id,
            administrator,
            id,
            ServiceOperationKind::Scale,
            Some(replicas),
        )
    }
    pub fn force_update(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> mpsc::Receiver<SwarmServiceProgressItem> {
        self.run_operation(
            actor_id,
            administrator,
            id,
            ServiceOperationKind::ForceUpdate,
            None,
        )
    }

    fn run_operation(
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

    fn run_requested_operation(
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
        tokio::spawn(async move {
            let _permit = permit;
            service
                .execute_operation(sender, actor_id, administrator, request)
                .await;
        });
        receiver
    }

    async fn execute_operation(
        &self,
        sender: mpsc::Sender<SwarmServiceProgressItem>,
        actor_id: ActorId,
        administrator: bool,
        request: ServiceOperationRequest,
    ) {
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
            return;
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
                return;
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
                    let _ = self
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
                    return;
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
            return;
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
                    return;
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
                    return;
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
                let _ = self
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
                        message,
                    ))
                    .await;
            }
            Ok(Err(error)) => {
                let message = redact(error.to_string(), &redaction_values);
                let unknown = matches!(
                    error,
                    SwarmServiceError::Runtime(_) | SwarmServiceError::Cancelled
                );
                let _ = self
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
                        message,
                    ))
                    .await;
            }
            Err(_) => {
                let message =
                    "Service operation timed out before its Docker outcome could be confirmed.";
                let _ = self
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
                        message,
                    ))
                    .await;
            }
        }
    }

    async fn resolve_environment(
        &self,
        claim: &ServiceOperationClaim,
    ) -> Result<(Vec<String>, Vec<String>, String), SwarmServiceError> {
        let referenced = referenced_binding_names(&claim.spec.environment)?;
        let resolved = self.bindings.resolve(claim.id, &referenced).await?;
        let selected = resolved
            .entries
            .iter()
            .filter(|entry| {
                referenced
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(&entry.name))
            })
            .collect::<Vec<_>>();
        for name in &referenced {
            if !selected
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case(name))
            {
                return Err(validation(&format!(
                    "Service references undefined Citadel variable or secret '{name}'."
                )));
            }
        }
        let mut environment = Vec::with_capacity(claim.spec.environment.len());
        for configured in &claim.spec.environment {
            if !configured.contains('=') && valid_binding_name(configured) {
                let binding = selected
                    .iter()
                    .find(|entry| entry.name.eq_ignore_ascii_case(configured))
                    .expect("referenced binding was validated");
                environment.push(format!("{configured}={}", binding.value.as_str()));
                continue;
            }
            let mut value = configured.clone();
            for binding in &selected {
                value = value.replace(&format!("${{{}}}", binding.name), binding.value.as_str());
            }
            environment.push(value);
        }
        let redaction_values = selected
            .iter()
            .filter(|entry| entry.secret)
            .map(|entry| entry.value.to_string())
            .collect::<Vec<_>>();
        let variables = selected.iter().filter(|entry| !entry.secret).count();
        let secrets = selected.iter().filter(|entry| entry.secret).count();
        let message = if variables == 0 && secrets == 0 {
            "No Citadel variables or secrets were referenced by this Service.".to_owned()
        } else {
            format!("Injected {variables} Citadel variable(s) and {secrets} secret(s).")
        };
        Ok((environment, redaction_values, message))
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
        for (actor_id, deletion) in self
            .store
            .stale_deletion_claims(cutoff, limit.clamp(1, 100))
            .await?
        {
            let cancellation = self.shutdown.child_token();
            let result = match deletion.docker_service_id.as_deref() {
                Some(docker_id) => tokio::time::timeout(
                    self.operation_timeout.min(Duration::from_secs(30)),
                    self.runtime
                        .delete(deletion.platform_id, docker_id, &cancellation),
                )
                .await
                .map_err(|_| {
                    SwarmServiceError::Runtime(
                        "Timed out while reconciling Service deletion.".to_owned(),
                    )
                })?,
                None => Ok(()),
            };
            match result {
                Ok(()) => {
                    self.store
                        .complete_delete(actor_id, std::slice::from_ref(&deletion))
                        .await?;
                    self.notifier.changed(deletion.id, "deleted");
                    reconciled += 1;
                }
                Err(SwarmServiceError::RuntimeRejected(_)) => {
                    self.store.release_delete(&[deletion.id]).await?;
                    reconciled += 1;
                }
                Err(_) => {}
            }
        }
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

fn normalize_name(value: &mut String) -> Result<(), SwarmServiceError> {
    *value = value.trim().to_owned();
    if value.is_empty() || value.len() > 100 {
        return Err(validation(
            "Service name must contain between 1 and 100 characters.",
        ));
    }
    Ok(())
}
fn normalize_description(value: &mut Option<String>) -> Result<(), SwarmServiceError> {
    if let Some(description) = value {
        *description = description.trim().to_owned();
        if description.len() > 500 {
            return Err(validation(
                "Service description cannot exceed 500 characters.",
            ));
        }
        if description.is_empty() {
            *value = None;
        }
    }
    Ok(())
}
fn unique_ids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut seen = HashSet::with_capacity(ids.len());
    ids.iter().copied().filter(|id| seen.insert(*id)).collect()
}
fn referenced_binding_names(environment: &[String]) -> Result<Vec<String>, SwarmServiceError> {
    let mut names = Vec::new();
    for entry in environment {
        if let Some((name, _)) = entry.split_once('=') {
            if name.trim().is_empty() {
                return Err(validation(
                    "Service environment variable names must not be empty.",
                ));
            }
        } else if valid_binding_name(entry) {
            push_unique_name(&mut names, entry);
        }
        let mut offset = 0;
        while let Some(start) = entry[offset..].find("${") {
            let name_start = offset + start + 2;
            let Some(end) = entry[name_start..].find('}') else {
                return Err(validation(&format!(
                    "Service environment entry '{entry}' has an incomplete variable reference."
                )));
            };
            let name = &entry[name_start..name_start + end];
            if !valid_binding_name(name) {
                return Err(validation(&format!(
                    "Service environment entry '{entry}' contains an invalid variable reference."
                )));
            }
            push_unique_name(&mut names, name);
            offset = name_start + end + 1;
        }
    }
    Ok(names)
}
fn valid_binding_name(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}
fn push_unique_name(names: &mut Vec<String>, value: &str) {
    if !names.iter().any(|name| name.eq_ignore_ascii_case(value)) {
        names.push(value.to_owned());
    }
}
fn redact(mut message: String, values: &[String]) -> String {
    for value in values.iter().filter(|value| !value.is_empty()) {
        message = message.replace(value, "********");
    }
    message
}
fn validation(message: &str) -> SwarmServiceError {
    SwarmServiceError::Validation(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_references_are_unique_and_case_insensitive() {
        let references = referenced_binding_names(&[
            "TOKEN=${API_TOKEN}".to_owned(),
            "URL=https://${HOST}/api/${api_token}".to_owned(),
            "PLAIN".to_owned(),
        ])
        .unwrap();

        assert_eq!(references, ["API_TOKEN", "HOST", "PLAIN"]);
    }

    #[test]
    fn incomplete_binding_reference_is_rejected() {
        assert!(referenced_binding_names(&["TOKEN=${MISSING".to_owned()]).is_err());
    }

    #[test]
    fn secret_values_are_redacted_from_runtime_errors() {
        assert_eq!(
            redact(
                "Docker rejected value very-secret".to_owned(),
                &["very-secret".to_owned()]
            ),
            "Docker rejected value ********"
        );
    }
}
