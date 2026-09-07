use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_domain::{ActorId, LicenseCapability};
use futures_util::future::BoxFuture;
use serde_json::Value;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[path = "updates.rs"]
mod updates;
pub use updates::*;

use crate::{
    ApplyClaim, CreateDeploymentInput, DeletionClaim, DeploymentBindingSnapshot,
    DeploymentConfigView, DeploymentDuplicateDraftView, DeploymentError, DeploymentFilter,
    DeploymentImageInfo, DeploymentSpec, DeploymentStreamItem, DeploymentView, DeploymentsView,
    FieldPatch, PatchDeploymentMetadataInput, PreparedDeploymentImage, ResolvedDeploymentBindings,
    ResourceCapabilities, RuntimeContainerState, RuntimeDeploymentCommand, RuntimeDeploymentResult,
};

pub trait DeploymentStore: Send + Sync {
    fn begin_update_check<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a DeploymentView,
    ) -> BoxFuture<'a, Result<DeploymentUpdateCheck, DeploymentError>> {
        let _ = (actor, administrator, expected);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Image update checks are unavailable.".into(),
            ))
        })
    }
    fn complete_update_check<'a>(
        &'a self,
        claim: &'a DeploymentUpdateCheck,
        state: Option<&'a crate::AutoUpdateState>,
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        let _ = (claim, state);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Image update checks are unavailable.".into(),
            ))
        })
    }
    fn recover_update_checks(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, DeploymentError>> {
        let _ = (started_before, limit);
        Box::pin(async { Ok(Vec::new()) })
    }
    fn scheduled_update_candidates(
        &self,
        after: Uuid,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, DeploymentError>> {
        let _ = (after, limit);
        Box::pin(async { Ok(Vec::new()) })
    }
    fn claim_apply_versioned(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        expected_version: Option<i64>,
    ) -> BoxFuture<'_, Result<ApplyClaim, DeploymentError>> {
        if expected_version.is_some() {
            return Box::pin(async {
                Err(DeploymentError::Runtime(
                    "Versioned Apply is unavailable.".into(),
                ))
            });
        }
        self.claim_apply(actor, administrator, id)
    }
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        filter: &'a DeploymentFilter,
    ) -> BoxFuture<'a, Result<Vec<DeploymentView>, DeploymentError>>;

    fn get_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>>;

    fn create<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a CreateDeploymentInput,
    ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>>;

    fn update_config<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        expected_row_version: i64,
        spec: &'a DeploymentSpec,
    ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>>;

    fn update_metadata<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: &'a PatchDeploymentMetadataInput,
    ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>>;

    fn rename<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        name: &'a str,
    ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>>;

    fn duplicate_draft<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<DeploymentDuplicateDraftView, DeploymentError>>;

    fn claim_delete<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<DeletionClaim>, DeploymentError>>;

    fn complete_delete<'a>(
        &'a self,
        actor_id: ActorId,
        claims: &'a [DeletionClaim],
    ) -> BoxFuture<'a, Result<(), DeploymentError>>;

    fn release_delete<'a>(
        &'a self,
        claims: &'a [DeletionClaim],
    ) -> BoxFuture<'a, Result<(), DeploymentError>>;

    fn claim_apply<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<ApplyClaim, DeploymentError>> {
        let _ = (actor_id, administrator, id);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Deployment Apply persistence is unavailable.".to_owned(),
            ))
        })
    }

    fn complete_apply<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ApplyClaim,
        result: &'a RuntimeDeploymentResult,
        digest: Option<&'a str>,
        bindings: &'a [DeploymentBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        let _ = (actor_id, claim, result, digest, bindings);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Deployment Apply persistence is unavailable.".to_owned(),
            ))
        })
    }

    fn fail_apply<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ApplyClaim,
        message: &'a str,
        result: Option<&'a RuntimeDeploymentResult>,
        bindings: &'a [DeploymentBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        let _ = (actor_id, claim, message, result, bindings);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Deployment Apply persistence is unavailable.".to_owned(),
            ))
        })
    }

    fn stale_apply_claims<'a>(
        &'a self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<(ActorId, ApplyClaim)>, DeploymentError>> {
        let _ = (started_before, limit);
        Box::pin(async { Ok(Vec::new()) })
    }
}

pub trait DeploymentRuntimePort: Send + Sync {
    fn remote_image_digest<'a>(
        &'a self,
        platform: Uuid,
        registry: Uuid,
        reference: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, DeploymentError>> {
        let _ = (platform, registry, reference, cancellation);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Image update checking is unavailable.".into(),
            ))
        })
    }
    fn delete_container<'a>(
        &'a self,
        platform_id: Uuid,
        docker_container_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), DeploymentError>>;

    fn prepare_image<'a>(
        &'a self,
        platform_id: Uuid,
        image: &'a DeploymentImageInfo,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<PreparedDeploymentImage, DeploymentError>> {
        let _ = (platform_id, image, cancellation);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Deployment image preparation is unavailable.".to_owned(),
            ))
        })
    }

    fn apply_container<'a>(
        &'a self,
        platform_id: Uuid,
        command: &'a RuntimeDeploymentCommand,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeDeploymentResult, DeploymentError>> {
        let _ = (platform_id, command, cancellation);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Deployment runtime Apply is unavailable.".to_owned(),
            ))
        })
    }

    fn observe_deployment<'a>(
        &'a self,
        platform_id: Uuid,
        deployment_id: Uuid,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<RuntimeDeploymentResult>, DeploymentError>> {
        let _ = (platform_id, deployment_id, cancellation);
        Box::pin(async { Ok(None) })
    }
}

pub trait DeploymentBindingResolverPort: Send + Sync {
    fn resolve<'a>(
        &'a self,
        deployment_id: Uuid,
        referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedDeploymentBindings, DeploymentError>>;
}

#[derive(Default)]
pub struct EmptyDeploymentBindingResolver;

impl DeploymentBindingResolverPort for EmptyDeploymentBindingResolver {
    fn resolve<'a>(
        &'a self,
        _deployment_id: Uuid,
        _referenced_names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedDeploymentBindings, DeploymentError>> {
        Box::pin(async { Ok(ResolvedDeploymentBindings::default()) })
    }
}

pub trait DeploymentEntitlementPort: Send + Sync {
    fn enabled(
        &self,
        capability: LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, DeploymentError>>;
}

pub trait DeploymentChangeNotifier: Send + Sync {
    fn changed(&self, deployment_id: Uuid, event: &'static str);
}

#[derive(Default)]
pub struct NoopDeploymentChangeNotifier;

impl DeploymentChangeNotifier for NoopDeploymentChangeNotifier {
    fn changed(&self, _deployment_id: Uuid, _event: &'static str) {}
}

#[derive(Clone)]
pub struct DeploymentService {
    store: Arc<dyn DeploymentStore>,
    runtime: Arc<dyn DeploymentRuntimePort>,
    entitlements: Arc<dyn DeploymentEntitlementPort>,
    notifier: Arc<dyn DeploymentChangeNotifier>,
    bindings: Arc<dyn DeploymentBindingResolverPort>,
    shutdown: CancellationToken,
    delete_timeout: Duration,
    apply_timeout: Duration,
    apply_slots: Arc<Semaphore>,
    alerts: Option<Arc<dyn AlertEventSink>>,
}

impl DeploymentService {
    #[must_use]
    pub fn new(
        store: Arc<dyn DeploymentStore>,
        runtime: Arc<dyn DeploymentRuntimePort>,
        entitlements: Arc<dyn DeploymentEntitlementPort>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            store,
            runtime,
            entitlements,
            notifier: Arc::new(NoopDeploymentChangeNotifier),
            bindings: Arc::new(EmptyDeploymentBindingResolver),
            shutdown,
            delete_timeout: Duration::from_secs(30),
            apply_timeout: Duration::from_secs(10 * 60),
            apply_slots: Arc::new(Semaphore::new(4)),
            alerts: None,
        }
    }

    #[must_use]
    pub fn with_notifier(mut self, notifier: Arc<dyn DeploymentChangeNotifier>) -> Self {
        self.notifier = notifier;
        self
    }

    #[must_use]
    pub fn with_binding_resolver(
        mut self,
        bindings: Arc<dyn DeploymentBindingResolverPort>,
    ) -> Self {
        self.bindings = bindings;
        self
    }

    #[must_use]
    pub fn with_alerts(mut self, alerts: Arc<dyn AlertEventSink>) -> Self {
        self.alerts = Some(alerts);
        self
    }

    #[must_use]
    pub fn with_delete_timeout(mut self, timeout: Duration) -> Self {
        self.delete_timeout = timeout;
        self
    }

    #[must_use]
    pub fn with_apply_timeout(mut self, timeout: Duration) -> Self {
        self.apply_timeout = timeout;
        self
    }

    pub async fn list(
        &self,
        actor_id: ActorId,
        administrator: bool,
        filter: &DeploymentFilter,
        capabilities: ResourceCapabilities,
    ) -> Result<DeploymentsView, DeploymentError> {
        Ok(DeploymentsView {
            deployments: self
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
    ) -> Result<DeploymentView, DeploymentError> {
        self.store.get_authorized(actor_id, administrator, id).await
    }

    pub async fn get_config(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<DeploymentConfigView, DeploymentError> {
        self.get(actor_id, administrator, id)
            .await
            .map(|value| DeploymentConfigView::from(&value))
    }

    pub async fn create(
        &self,
        actor_id: ActorId,
        administrator: bool,
        mut input: CreateDeploymentInput,
    ) -> Result<DeploymentView, DeploymentError> {
        normalize_name(&mut input.name)?;
        normalize_description(&mut input.description)?;
        if input.platform_id.is_nil() {
            return Err(DeploymentError::Validation(
                "A Platform must be selected.".to_owned(),
            ));
        }
        if input.tag_ids.len() > 100 {
            return Err(DeploymentError::Validation(
                "A Deployment cannot contain more than 100 Tags.".to_owned(),
            ));
        }
        input.tag_ids = unique_ids(&input.tag_ids);
        if let Some(source) = &input.duplicate_source
            && (source.resource_type != "Deployment" || source.resource_id.is_nil())
        {
            return Err(DeploymentError::Validation(
                "Duplicate source must identify a Deployment.".to_owned(),
            ));
        }
        input.spec = input.spec.for_create();
        input.spec.validate()?;
        self.ensure_expansion_entitlements(None, &input.spec)
            .await?;
        let created = self.store.create(actor_id, administrator, &input).await?;
        self.notifier.changed(created.id, "created");
        Ok(created)
    }

    pub async fn update_config(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        platform_id: Option<Uuid>,
        spec: Option<Value>,
    ) -> Result<DeploymentView, DeploymentError> {
        let current = self
            .store
            .get_authorized(actor_id, administrator, id)
            .await?;
        if platform_id.is_some_and(|platform_id| platform_id != current.platform_id) {
            return Err(DeploymentError::Validation(
                "A Deployment cannot be moved to another Platform.".to_owned(),
            ));
        }
        let spec = match spec {
            Some(patch) => merge_spec(&current.spec, patch)?,
            None => current.spec.clone(),
        };
        let spec = spec.for_update(&current.spec);
        spec.validate()?;
        self.ensure_expansion_entitlements(Some(&current.spec), &spec)
            .await?;
        let updated = self
            .store
            .update_config(actor_id, administrator, id, current.row_version, &spec)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn update_metadata(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        mut input: PatchDeploymentMetadataInput,
    ) -> Result<DeploymentView, DeploymentError> {
        if let FieldPatch::Set(description) = &mut input.description {
            let mut value = Some(std::mem::take(description));
            normalize_description(&mut value)?;
            input.description = value.map_or(FieldPatch::Clear, FieldPatch::Set);
        }
        let updated = self
            .store
            .update_metadata(actor_id, administrator, id, &input)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn rename(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        mut name: String,
    ) -> Result<DeploymentView, DeploymentError> {
        normalize_name(&mut name)?;
        let renamed = self
            .store
            .rename(actor_id, administrator, id, &name)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(renamed)
    }

    pub async fn duplicate_draft(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<DeploymentDuplicateDraftView, DeploymentError> {
        self.store
            .duplicate_draft(actor_id, administrator, id)
            .await
    }

    pub async fn delete(
        &self,
        actor_id: ActorId,
        administrator: bool,
        ids: Vec<Uuid>,
    ) -> Result<(), DeploymentError> {
        let ids = unique_ids(&ids);
        if ids.is_empty() || ids.iter().any(Uuid::is_nil) {
            return Err(DeploymentError::Validation(
                "Deployment IDs must not be empty.".to_owned(),
            ));
        }
        if ids.len() > 100 {
            return Err(DeploymentError::Validation(
                "At most 100 Deployments can be deleted at once.".to_owned(),
            ));
        }
        let claims = self
            .store
            .claim_delete(actor_id, administrator, &ids)
            .await?;
        let store = Arc::clone(&self.store);
        let runtime = Arc::clone(&self.runtime);
        let notifier = Arc::clone(&self.notifier);
        let shutdown = self.shutdown.clone();
        let timeout = self.delete_timeout;
        let task = tokio::spawn(async move {
            let cancellation = shutdown.child_token();
            let result = tokio::select! {
                biased;
                () = shutdown.cancelled() => Err(DeploymentError::Cancelled),
                result = tokio::time::timeout(
                    timeout,
                    delete_claimed(runtime.as_ref(), &claims, &cancellation),
                ) => result.unwrap_or_else(|_| {
                    Err(DeploymentError::Runtime(
                        "Deployment deletion timed out.".to_owned(),
                    ))
                }),
            };
            cancellation.cancel();
            if let Err(error) = result {
                return Err(release_after_failure(store.as_ref(), &claims, error).await);
            }
            if let Err(error) = store.complete_delete(actor_id, &claims).await {
                return Err(release_after_failure(store.as_ref(), &claims, error).await);
            }
            for claim in &claims {
                notifier.changed(claim.id, "deleted");
            }
            Ok(())
        });
        task.await.map_err(|error| {
            DeploymentError::Runtime(format!("delete worker failed unexpectedly: {error}"))
        })?
    }

    pub async fn apply(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        recreate: bool,
    ) -> Result<mpsc::Receiver<DeploymentStreamItem>, DeploymentError> {
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
    ) -> Result<mpsc::Receiver<DeploymentStreamItem>, DeploymentError> {
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
            claim,
            store: Arc::clone(&self.store),
            runtime: Arc::clone(&self.runtime),
            bindings: Arc::clone(&self.bindings),
            notifier: Arc::clone(&self.notifier),
            alerts: self.alerts.clone(),
            sender,
            _permit: permit,
        };
        let shutdown = self.shutdown.clone();
        let timeout = self.apply_timeout;
        tokio::spawn(async move {
            let _ = operation.run(&shutdown, timeout).await;
        });
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
                Ok(None) | Err(_) => {
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

    async fn ensure_expansion_entitlements(
        &self,
        current: Option<&DeploymentSpec>,
        proposed: &DeploymentSpec,
    ) -> Result<(), DeploymentError> {
        if expands_operational_guardrails(current, proposed) {
            self.require_entitlement(
                LicenseCapability::OperationalGuardrails,
                "operational-guardrails",
            )
            .await?;
        }
        if expands_automated_operations(current, proposed) {
            self.require_entitlement(
                LicenseCapability::AutomatedOperations,
                "automated-operations",
            )
            .await?;
        }
        Ok(())
    }

    async fn require_entitlement(
        &self,
        capability: LicenseCapability,
        name: &'static str,
    ) -> Result<(), DeploymentError> {
        if self.entitlements.enabled(capability).await? {
            Ok(())
        } else {
            Err(DeploymentError::LicenseRequired(name))
        }
    }
}

struct ApplyOperation {
    actor_id: ActorId,
    recreate: bool,
    claim: ApplyClaim,
    store: Arc<dyn DeploymentStore>,
    runtime: Arc<dyn DeploymentRuntimePort>,
    bindings: Arc<dyn DeploymentBindingResolverPort>,
    notifier: Arc<dyn DeploymentChangeNotifier>,
    alerts: Option<Arc<dyn AlertEventSink>>,
    sender: mpsc::Sender<DeploymentStreamItem>,
    _permit: OwnedSemaphorePermit,
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
            if matches!(error, DeploymentError::Cancelled)
                || matches!(&error, DeploymentError::Runtime(message) if message == "Deployment Apply timed out.")
            {
                self.send(DeploymentStreamItem::failure(
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
            self.send(DeploymentStreamItem::failure(
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
                self.send(DeploymentStreamItem::info(format!(
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
            self.send(DeploymentStreamItem::info(format!(
                "Container deleted: {container_id}"
            )));
        }

        self.send(DeploymentStreamItem::info(
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
        self.send(DeploymentStreamItem::info(binding_message(&resolved)));
        self.send(DeploymentStreamItem::info(format!(
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
        self.send(DeploymentStreamItem::info(format!(
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
            self.send(DeploymentStreamItem::failure(422, message));
            self.notifier.changed(self.claim.id, "updated");
            return Ok(());
        }
        if let Err(error) = self
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
            let message = error.to_string();
            self.store
                .fail_apply(
                    self.actor_id,
                    &self.claim,
                    &message,
                    Some(&runtime_result),
                    &environment.snapshots,
                )
                .await?;
            self.send(DeploymentStreamItem::failure(
                deployment_error_code(&error),
                message,
            ));
            self.notifier.changed(self.claim.id, "updated");
            return Ok(());
        }
        self.notifier.changed(self.claim.id, "updated");
        self.send(DeploymentStreamItem::info("Deployment is now running."));
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

    fn send(&self, item: DeploymentStreamItem) {
        let _ = self.sender.try_send(item);
    }
}

fn apply_resolved_build(
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

fn deployment_error_code(error: &DeploymentError) -> i64 {
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

fn referenced_binding_names(environment: &[String]) -> Result<Vec<String>, DeploymentError> {
    let mut names = Vec::new();
    for entry in environment {
        if let Some((name, _)) = entry.split_once('=') {
            if name.trim().is_empty() {
                return Err(DeploymentError::Validation(
                    "Deployment environment variable names must not be empty.".to_owned(),
                ));
            }
        } else if valid_binding_name(entry) {
            push_unique_name(&mut names, entry);
        }
        let bytes = entry.as_bytes();
        let mut offset = 0;
        while let Some(start) = entry[offset..].find("${") {
            let name_start = offset + start + 2;
            let Some(end) = entry[name_start..].find('}') else {
                return Err(DeploymentError::Validation(format!(
                    "Deployment environment entry '{entry}' has an incomplete variable reference."
                )));
            };
            let name = &entry[name_start..name_start + end];
            if !valid_binding_name(name) {
                return Err(DeploymentError::Validation(format!(
                    "Deployment environment entry '{entry}' contains an invalid variable reference."
                )));
            }
            push_unique_name(&mut names, name);
            offset = name_start + end + 1;
            if offset >= bytes.len() {
                break;
            }
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

struct ResolvedDeploymentEnvironment {
    values: Vec<String>,
    snapshots: Vec<DeploymentBindingSnapshot>,
    redaction_values: Vec<String>,
}

fn build_environment(
    configured: &[String],
    referenced: &[String],
    resolved: &ResolvedDeploymentBindings,
) -> Result<ResolvedDeploymentEnvironment, DeploymentError> {
    let selected = resolved
        .entries
        .iter()
        .filter(|entry| {
            referenced
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&entry.name))
        })
        .collect::<Vec<_>>();
    for name in referenced {
        if !selected
            .iter()
            .any(|entry| entry.name.eq_ignore_ascii_case(name))
        {
            return Err(DeploymentError::Validation(format!(
                "Deployment references undefined Citadel variable or secret '{name}'."
            )));
        }
    }
    let mut output = Vec::with_capacity(configured.len());
    for configured_entry in configured {
        if !configured_entry.contains('=') && valid_binding_name(configured_entry) {
            let binding = selected
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(configured_entry))
                .expect("referenced binding was validated");
            output.push(format!("{configured_entry}={}", binding.value.as_str()));
            continue;
        }
        let mut value = configured_entry.clone();
        for binding in &selected {
            value = value.replace(&format!("${{{}}}", binding.name), binding.value.as_str());
        }
        output.push(value);
    }
    let snapshots = selected
        .iter()
        .map(|entry| entry.snapshot.clone())
        .collect();
    let redaction_values = selected
        .iter()
        .filter(|entry| entry.secret)
        .map(|entry| entry.value.to_string())
        .collect();
    Ok(ResolvedDeploymentEnvironment {
        values: output,
        snapshots,
        redaction_values,
    })
}

fn binding_message(resolved: &ResolvedDeploymentBindings) -> String {
    let variables = resolved
        .entries
        .iter()
        .filter(|entry| !entry.secret)
        .count();
    let secrets = resolved.entries.iter().filter(|entry| entry.secret).count();
    if variables == 0 && secrets == 0 {
        "No Citadel variables or secrets were referenced by this Deployment.".to_owned()
    } else {
        format!("Injected {variables} Citadel variable(s) and {secrets} secret(s).")
    }
}

fn redact_error(error: DeploymentError, values: &[String]) -> DeploymentError {
    let mut message = error.to_string();
    for value in values.iter().filter(|value| !value.is_empty()) {
        message = message.replace(value, "********");
    }
    DeploymentError::Runtime(message)
}

fn expands_automated_operations(
    current: Option<&DeploymentSpec>,
    proposed: &DeploymentSpec,
) -> bool {
    let proposed_build = build_redeploy_target(proposed);
    let current_build = current.and_then(build_redeploy_target);
    proposed_build.is_some() && proposed_build != current_build
}

fn expands_operational_guardrails(
    current: Option<&DeploymentSpec>,
    proposed: &DeploymentSpec,
) -> bool {
    proposed.update_behavior == crate::UpdateBehavior::AutoDeploy
        && current.is_none_or(|value| value.update_behavior != crate::UpdateBehavior::AutoDeploy)
}

fn build_redeploy_target(spec: &DeploymentSpec) -> Option<Uuid> {
    match &spec.image {
        crate::DeploymentImageInfo::Build {
            build_project_id,
            redeploy_on_build: true,
            ..
        } => Some(*build_project_id),
        _ => None,
    }
}

fn merge_spec(current: &DeploymentSpec, patch: Value) -> Result<DeploymentSpec, DeploymentError> {
    if !patch.is_object() {
        return Err(DeploymentError::Validation(
            "Deployment spec patch must be a JSON object.".to_owned(),
        ));
    }
    let mut merged = serde_json::to_value(current).map_err(|error| {
        DeploymentError::Storage(format!("failed to serialize Deployment spec: {error}"))
    })?;
    merge_json(&mut merged, patch);
    serde_json::from_value(merged).map_err(|error| {
        DeploymentError::Validation(format!("Deployment spec patch is invalid: {error}"))
    })
}

fn merge_json(target: &mut Value, patch: Value) {
    match patch {
        Value::Object(patch) => {
            if !target.is_object() {
                *target = Value::Object(serde_json::Map::new());
            }
            let target = target
                .as_object_mut()
                .expect("target was replaced by an object");
            for (key, value) in patch {
                if value.is_null() {
                    target.remove(&key);
                } else {
                    merge_json(target.entry(key).or_insert(Value::Null), value);
                }
            }
        }
        value => *target = value,
    }
}

async fn release_after_failure(
    store: &dyn DeploymentStore,
    claims: &[DeletionClaim],
    operation_error: DeploymentError,
) -> DeploymentError {
    match store.release_delete(claims).await {
        Ok(()) => operation_error,
        Err(release_error) => DeploymentError::Storage(format!(
            "{operation_error}; deletion claim release also failed: {release_error}"
        )),
    }
}

async fn delete_claimed(
    runtime: &dyn DeploymentRuntimePort,
    claims: &[DeletionClaim],
    cancellation: &CancellationToken,
) -> Result<(), DeploymentError> {
    for claim in claims {
        for container_id in &claim.docker_container_ids {
            runtime
                .delete_container(claim.platform_id, container_id, cancellation)
                .await?;
        }
    }
    Ok(())
}

fn normalize_name(name: &mut String) -> Result<(), DeploymentError> {
    *name = name.trim().to_owned();
    if name.is_empty() || name.len() > 64 {
        return Err(DeploymentError::Validation(
            "Deployment name must contain between 1 and 64 characters.".to_owned(),
        ));
    }
    if !name
        .bytes()
        .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_' | b'.'))
    {
        return Err(DeploymentError::Validation(
            "Deployment name contains unsupported characters.".to_owned(),
        ));
    }
    Ok(())
}

fn normalize_description(description: &mut Option<String>) -> Result<(), DeploymentError> {
    *description = description
        .take()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if description.as_ref().is_some_and(|value| value.len() > 600) {
        return Err(DeploymentError::Validation(
            "Deployment description cannot exceed 600 characters.".to_owned(),
        ));
    }
    Ok(())
}

fn unique_ids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut seen = HashSet::with_capacity(ids.len());
    ids.iter().copied().filter(|id| seen.insert(*id)).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use futures_util::FutureExt;
    use serde_json::json;
    use tokio::sync::Notify;

    use super::*;
    use crate::{DeploymentImageInfo, UpdateBehavior};

    fn spec() -> DeploymentSpec {
        DeploymentSpec {
            image: DeploymentImageInfo::Local {
                image_id: "image".to_owned(),
            },
            update_behavior: UpdateBehavior::Disabled,
            life_cycle_spec: None,
            resource_spec: None,
            labels: None,
            ports: None,
            volumes: None,
            networks: None,
            command: None,
            environment_variables: None,
        }
    }

    #[test]
    fn non_external_images_cannot_enable_auto_update() {
        let mut value = spec();
        value.update_behavior = UpdateBehavior::Notify;
        assert!(value.validate().is_err());
    }

    #[test]
    fn preserving_provenance_only_applies_to_same_build_project() {
        let project = Uuid::now_v7();
        let current = DeploymentImageInfo::Build {
            build_project_id: project,
            redeploy_on_build: false,
            resolved_image_reference: Some("repo:build".to_owned()),
            resolved_digest: Some("sha256:abc".to_owned()),
            resolved_build_run_id: Some(Uuid::now_v7()),
            applied_image_reference: None,
            applied_digest: None,
            applied_build_run_id: None,
            applied_at: None,
        };
        let next = DeploymentImageInfo::Build {
            build_project_id: project,
            redeploy_on_build: true,
            resolved_image_reference: None,
            resolved_digest: None,
            resolved_build_run_id: None,
            applied_image_reference: None,
            applied_digest: None,
            applied_build_run_id: None,
            applied_at: None,
        }
        .preserving_build_provenance_from(&current);
        assert!(matches!(
            next,
            DeploymentImageInfo::Build {
                resolved_digest: Some(_),
                ..
            }
        ));
    }

    fn build_spec(project: Uuid, redeploy_on_build: bool) -> DeploymentSpec {
        DeploymentSpec {
            image: DeploymentImageInfo::Build {
                build_project_id: project,
                redeploy_on_build,
                resolved_image_reference: None,
                resolved_digest: None,
                resolved_build_run_id: None,
                applied_image_reference: None,
                applied_digest: None,
                applied_build_run_id: None,
                applied_at: None,
            },
            ..spec()
        }
    }

    #[test]
    fn merge_patch_preserves_unspecified_configuration_and_accepts_lowercase_enums() {
        let current = DeploymentSpec {
            image: DeploymentImageInfo::External {
                registry_id: Uuid::from_u128(0x100),
                image_tag: "nginx:latest".to_owned(),
                resolved_digest: Some("sha256:old".to_owned()),
            },
            update_behavior: UpdateBehavior::AutoDeploy,
            ports: Some(vec!["80:80".to_owned()]),
            ..spec()
        };

        let merged = merge_spec(&current, json!({"updateBehavior":"disabled"})).unwrap();

        assert_eq!(merged.update_behavior, UpdateBehavior::Disabled);
        assert_eq!(merged.ports, current.ports);
        assert_eq!(merged.image, current.image);
    }

    #[test]
    fn merge_patch_rejects_a_non_object_spec() {
        assert!(matches!(
            merge_spec(&spec(), json!([])),
            Err(DeploymentError::Validation(_))
        ));
    }

    #[test]
    fn automated_operations_policy_ignores_unrelated_changes() {
        let project = Uuid::now_v7();
        let current = build_spec(project, true);
        let mut proposed = current.clone();
        proposed.labels = Some(std::collections::BTreeMap::from([(
            "env".to_owned(),
            "prod".to_owned(),
        )]));

        assert!(!expands_automated_operations(Some(&current), &proposed));
    }

    #[test]
    fn automated_operations_policy_detects_a_build_target_change() {
        let current = build_spec(Uuid::now_v7(), true);
        let proposed = build_spec(Uuid::now_v7(), true);

        assert!(expands_automated_operations(Some(&current), &proposed));
    }

    #[test]
    fn automated_operations_policy_allows_disabling_redeploy_on_build() {
        let project = Uuid::now_v7();
        let current = build_spec(project, true);
        let proposed = build_spec(project, false);

        assert!(!expands_automated_operations(Some(&current), &proposed));
    }

    #[test]
    fn operational_guardrails_policy_detects_only_new_auto_deploy_authority() {
        let mut disabled = spec();
        disabled.image = DeploymentImageInfo::External {
            registry_id: Uuid::from_u128(0x100),
            image_tag: "nginx:latest".to_owned(),
            resolved_digest: None,
        };
        let mut automatic = disabled.clone();
        automatic.update_behavior = UpdateBehavior::AutoDeploy;
        let mut unrelated = automatic.clone();
        unrelated.labels = Some(std::collections::BTreeMap::from([(
            "env".to_owned(),
            "prod".to_owned(),
        )]));

        assert!(expands_operational_guardrails(Some(&disabled), &automatic));
        assert!(!expands_operational_guardrails(
            Some(&automatic),
            &unrelated
        ));
        assert!(!expands_operational_guardrails(Some(&automatic), &disabled));
    }

    #[test]
    fn deployment_environment_injects_only_referenced_bindings_and_masks_secrets() {
        let configured = vec![
            "LOG_LEVEL=${LOG_LEVEL}".to_owned(),
            "API_TOKEN".to_owned(),
            "UNCHANGED=value".to_owned(),
        ];
        let referenced = referenced_binding_names(&configured).unwrap();
        let resolved = ResolvedDeploymentBindings {
            entries: vec![
                resolved_binding("LOG_LEVEL", "debug", false),
                resolved_binding("API_TOKEN", "very-secret", true),
                resolved_binding("UNUSED", "must-not-be-injected", true),
            ],
        };

        let environment = build_environment(&configured, &referenced, &resolved).unwrap();

        assert_eq!(
            environment.values,
            [
                "LOG_LEVEL=debug",
                "API_TOKEN=very-secret",
                "UNCHANGED=value"
            ]
        );
        assert_eq!(environment.snapshots.len(), 2);
        assert_eq!(environment.snapshots[1].value, "********");
        assert_eq!(environment.redaction_values, ["very-secret"]);
        assert!(!format!("{:?}", environment.snapshots).contains("very-secret"));
        assert!(
            !environment
                .values
                .iter()
                .any(|entry| entry.contains("UNUSED"))
        );
    }

    #[test]
    fn deployment_environment_rejects_undefined_and_malformed_references() {
        let missing = vec!["TOKEN=${MISSING}".to_owned()];
        let referenced = referenced_binding_names(&missing).unwrap();
        assert!(
            build_environment(
                &missing,
                &referenced,
                &ResolvedDeploymentBindings::default()
            )
            .is_err()
        );
        assert!(referenced_binding_names(&["TOKEN=${BROKEN".to_owned()]).is_err());
    }

    #[test]
    fn successful_build_apply_records_the_exact_resolved_run_and_image() {
        let project_id = Uuid::now_v7();
        let run_id = Uuid::now_v7();
        let mut image = DeploymentImageInfo::Build {
            build_project_id: project_id,
            redeploy_on_build: true,
            resolved_image_reference: None,
            resolved_digest: None,
            resolved_build_run_id: None,
            applied_image_reference: None,
            applied_digest: None,
            applied_build_run_id: None,
            applied_at: None,
        };
        apply_resolved_build(
            &mut image,
            &PreparedDeploymentImage {
                docker_image_id: "sha256:local".to_owned(),
                digest: Some("registry/app@sha256:pulled".to_owned()),
                resolved_build: Some(crate::ResolvedDeploymentBuild {
                    image_reference: "registry/app:main".to_owned(),
                    digest: Some("sha256:built".to_owned()),
                    build_run_id: run_id,
                }),
            },
        )
        .unwrap();
        let DeploymentImageInfo::Build {
            resolved_image_reference,
            resolved_digest,
            resolved_build_run_id,
            applied_image_reference,
            applied_digest,
            applied_build_run_id,
            applied_at,
            ..
        } = image
        else {
            unreachable!();
        };
        assert_eq!(
            resolved_image_reference.as_deref(),
            Some("registry/app:main")
        );
        assert_eq!(resolved_digest.as_deref(), Some("sha256:built"));
        assert_eq!(resolved_build_run_id, Some(run_id));
        assert_eq!(
            applied_image_reference.as_deref(),
            Some("registry/app:main")
        );
        assert_eq!(
            applied_digest.as_deref(),
            Some("registry/app@sha256:pulled")
        );
        assert_eq!(applied_build_run_id, Some(run_id));
        assert!(applied_at.is_some());
    }

    fn resolved_binding(name: &str, value: &str, secret: bool) -> crate::ResolvedDeploymentBinding {
        crate::ResolvedDeploymentBinding {
            name: name.to_owned(),
            value: zeroize::Zeroizing::new(value.to_owned()),
            secret,
            snapshot: DeploymentBindingSnapshot {
                name: name.to_owned(),
                kind: if secret { "Secret" } else { "Variable" }.to_owned(),
                scope: "Deployment".to_owned(),
                value: if secret { "********" } else { value }.to_owned(),
                secret_id: secret.then(Uuid::now_v7),
                secret_delivery_mode: secret.then(|| "Environment".to_owned()),
                target_path: None,
            },
        }
    }

    struct TimeoutStore {
        claim: DeletionClaim,
        releases: Arc<AtomicUsize>,
        apply_claim: Option<ApplyClaim>,
        apply_completions: Arc<AtomicUsize>,
        apply_failures: Arc<AtomicUsize>,
    }

    impl DeploymentStore for TimeoutStore {
        fn list_authorized<'a>(
            &'a self,
            _: ActorId,
            _: bool,
            _: &'a DeploymentFilter,
        ) -> BoxFuture<'a, Result<Vec<DeploymentView>, DeploymentError>> {
            async { unreachable!() }.boxed()
        }

        fn get_authorized<'a>(
            &'a self,
            _: ActorId,
            _: bool,
            _: Uuid,
        ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>> {
            async { unreachable!() }.boxed()
        }

        fn create<'a>(
            &'a self,
            _: ActorId,
            _: bool,
            _: &'a CreateDeploymentInput,
        ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>> {
            async { unreachable!() }.boxed()
        }

        fn update_config<'a>(
            &'a self,
            _: ActorId,
            _: bool,
            _: Uuid,
            _: i64,
            _: &'a DeploymentSpec,
        ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>> {
            async { unreachable!() }.boxed()
        }

        fn update_metadata<'a>(
            &'a self,
            _: ActorId,
            _: bool,
            _: Uuid,
            _: &'a PatchDeploymentMetadataInput,
        ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>> {
            async { unreachable!() }.boxed()
        }

        fn rename<'a>(
            &'a self,
            _: ActorId,
            _: bool,
            _: Uuid,
            _: &'a str,
        ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>> {
            async { unreachable!() }.boxed()
        }

        fn duplicate_draft<'a>(
            &'a self,
            _: ActorId,
            _: bool,
            _: Uuid,
        ) -> BoxFuture<'a, Result<DeploymentDuplicateDraftView, DeploymentError>> {
            async { unreachable!() }.boxed()
        }

        fn claim_delete<'a>(
            &'a self,
            _: ActorId,
            _: bool,
            _: &'a [Uuid],
        ) -> BoxFuture<'a, Result<Vec<DeletionClaim>, DeploymentError>> {
            let claim = self.claim.clone();
            async move { Ok(vec![claim]) }.boxed()
        }

        fn complete_delete<'a>(
            &'a self,
            _: ActorId,
            _: &'a [DeletionClaim],
        ) -> BoxFuture<'a, Result<(), DeploymentError>> {
            async { unreachable!() }.boxed()
        }

        fn release_delete<'a>(
            &'a self,
            _: &'a [DeletionClaim],
        ) -> BoxFuture<'a, Result<(), DeploymentError>> {
            self.releases.fetch_add(1, Ordering::Relaxed);
            async { Ok(()) }.boxed()
        }

        fn claim_apply<'a>(
            &'a self,
            _: ActorId,
            _: bool,
            _: Uuid,
        ) -> BoxFuture<'a, Result<ApplyClaim, DeploymentError>> {
            let claim = self.apply_claim.clone();
            async move {
                claim.ok_or_else(|| {
                    DeploymentError::Runtime("Apply is unavailable in this fixture.".to_owned())
                })
            }
            .boxed()
        }

        fn complete_apply<'a>(
            &'a self,
            _: ActorId,
            _: &'a ApplyClaim,
            _: &'a RuntimeDeploymentResult,
            _: Option<&'a str>,
            _: &'a [DeploymentBindingSnapshot],
        ) -> BoxFuture<'a, Result<(), DeploymentError>> {
            self.apply_completions.fetch_add(1, Ordering::Relaxed);
            async { Ok(()) }.boxed()
        }

        fn fail_apply<'a>(
            &'a self,
            _: ActorId,
            _: &'a ApplyClaim,
            _: &'a str,
            _: Option<&'a RuntimeDeploymentResult>,
            _: &'a [DeploymentBindingSnapshot],
        ) -> BoxFuture<'a, Result<(), DeploymentError>> {
            self.apply_failures.fetch_add(1, Ordering::Relaxed);
            async { Ok(()) }.boxed()
        }

        fn stale_apply_claims<'a>(
            &'a self,
            _: i64,
            _: i64,
        ) -> BoxFuture<'a, Result<Vec<(ActorId, ApplyClaim)>, DeploymentError>> {
            let claim = self.apply_claim.clone();
            async move {
                Ok(claim
                    .map(|claim| (ActorId::new(Uuid::from_u128(1)), claim))
                    .into_iter()
                    .collect())
            }
            .boxed()
        }
    }

    struct PendingRuntime;

    impl DeploymentRuntimePort for PendingRuntime {
        fn delete_container<'a>(
            &'a self,
            _: Uuid,
            _: &'a str,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<(), DeploymentError>> {
            std::future::pending().boxed()
        }
    }

    struct SignalledPendingRuntime(Arc<Notify>);

    impl DeploymentRuntimePort for SignalledPendingRuntime {
        fn delete_container<'a>(
            &'a self,
            _: Uuid,
            _: &'a str,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<(), DeploymentError>> {
            async move {
                self.0.notify_one();
                std::future::pending().await
            }
            .boxed()
        }
    }

    struct SuccessfulApplyRuntime {
        commands: Arc<Mutex<Vec<RuntimeDeploymentCommand>>>,
    }

    impl DeploymentRuntimePort for SuccessfulApplyRuntime {
        fn delete_container<'a>(
            &'a self,
            _: Uuid,
            _: &'a str,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<(), DeploymentError>> {
            async { Ok(()) }.boxed()
        }

        fn prepare_image<'a>(
            &'a self,
            _: Uuid,
            _: &'a DeploymentImageInfo,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<PreparedDeploymentImage, DeploymentError>> {
            async {
                Ok(PreparedDeploymentImage {
                    docker_image_id: "sha256:image".to_owned(),
                    digest: None,
                    resolved_build: None,
                })
            }
            .boxed()
        }

        fn apply_container<'a>(
            &'a self,
            _: Uuid,
            command: &'a RuntimeDeploymentCommand,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimeDeploymentResult, DeploymentError>> {
            self.commands.lock().unwrap().push(command.clone());
            async {
                Ok(RuntimeDeploymentResult {
                    docker_container_id: "container".to_owned(),
                    docker_image_id: "sha256:image".to_owned(),
                    state: RuntimeContainerState::Running,
                })
            }
            .boxed()
        }
    }

    struct PendingApplyRuntime;

    impl DeploymentRuntimePort for PendingApplyRuntime {
        fn delete_container<'a>(
            &'a self,
            _: Uuid,
            _: &'a str,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<(), DeploymentError>> {
            async { Ok(()) }.boxed()
        }

        fn prepare_image<'a>(
            &'a self,
            _: Uuid,
            _: &'a DeploymentImageInfo,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<PreparedDeploymentImage, DeploymentError>> {
            async {
                Ok(PreparedDeploymentImage {
                    docker_image_id: "sha256:image".to_owned(),
                    digest: None,
                    resolved_build: None,
                })
            }
            .boxed()
        }

        fn apply_container<'a>(
            &'a self,
            _: Uuid,
            _: &'a RuntimeDeploymentCommand,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimeDeploymentResult, DeploymentError>> {
            std::future::pending().boxed()
        }
    }

    struct ObservedApplyRuntime(Option<RuntimeDeploymentResult>);

    impl DeploymentRuntimePort for ObservedApplyRuntime {
        fn delete_container<'a>(
            &'a self,
            _: Uuid,
            _: &'a str,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<(), DeploymentError>> {
            async { Ok(()) }.boxed()
        }

        fn observe_deployment<'a>(
            &'a self,
            _: Uuid,
            _: Uuid,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Option<RuntimeDeploymentResult>, DeploymentError>> {
            let result = self.0.clone();
            async move { Ok(result) }.boxed()
        }
    }

    struct AllowEntitlements;

    impl DeploymentEntitlementPort for AllowEntitlements {
        fn enabled(&self, _: LicenseCapability) -> BoxFuture<'_, Result<bool, DeploymentError>> {
            async { Ok(true) }.boxed()
        }
    }

    struct DenyEntitlements;

    impl DeploymentEntitlementPort for DenyEntitlements {
        fn enabled(&self, _: LicenseCapability) -> BoxFuture<'_, Result<bool, DeploymentError>> {
            async { Ok(false) }.boxed()
        }
    }

    #[tokio::test]
    async fn timed_out_delete_releases_its_claim() {
        let deployment_id = Uuid::now_v7();
        let releases = Arc::new(AtomicUsize::new(0));
        let service = DeploymentService::new(
            Arc::new(TimeoutStore {
                claim: DeletionClaim {
                    id: deployment_id,
                    platform_id: Uuid::now_v7(),
                    name: "web".to_owned(),
                    docker_container_ids: vec!["container".to_owned()],
                    row_version: 1,
                    previous_status: "Healthy".to_owned(),
                    description: None,
                    spec: spec(),
                },
                releases: Arc::clone(&releases),
                apply_claim: None,
                apply_completions: Arc::new(AtomicUsize::new(0)),
                apply_failures: Arc::new(AtomicUsize::new(0)),
            }),
            Arc::new(PendingRuntime),
            Arc::new(AllowEntitlements),
            CancellationToken::new(),
        )
        .with_delete_timeout(Duration::from_millis(5));

        let error = service
            .delete(ActorId::new(Uuid::now_v7()), true, vec![deployment_id])
            .await
            .unwrap_err();

        assert!(
            matches!(error, DeploymentError::Runtime(message) if message.contains("timed out"))
        );
        assert_eq!(releases.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn dropping_the_request_does_not_cancel_claim_recovery() {
        let deployment_id = Uuid::now_v7();
        let releases = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(Notify::new());
        let service = DeploymentService::new(
            Arc::new(TimeoutStore {
                claim: DeletionClaim {
                    id: deployment_id,
                    platform_id: Uuid::now_v7(),
                    name: "web".to_owned(),
                    docker_container_ids: vec!["container".to_owned()],
                    row_version: 1,
                    previous_status: "Healthy".to_owned(),
                    description: None,
                    spec: spec(),
                },
                releases: Arc::clone(&releases),
                apply_claim: None,
                apply_completions: Arc::new(AtomicUsize::new(0)),
                apply_failures: Arc::new(AtomicUsize::new(0)),
            }),
            Arc::new(SignalledPendingRuntime(Arc::clone(&started))),
            Arc::new(AllowEntitlements),
            CancellationToken::new(),
        )
        .with_delete_timeout(Duration::from_millis(20));
        let runtime_started = started.notified();
        let request = tokio::spawn(async move {
            service
                .delete(ActorId::new(Uuid::now_v7()), true, vec![deployment_id])
                .await
        });

        runtime_started.await;
        request.abort();
        tokio::time::timeout(Duration::from_secs(1), async {
            while releases.load(Ordering::Relaxed) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        assert_eq!(releases.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn dropping_apply_progress_does_not_cancel_the_claimed_operation() {
        let deployment_id = Uuid::now_v7();
        let completions = Arc::new(AtomicUsize::new(0));
        let commands = Arc::new(Mutex::new(Vec::new()));
        let service = DeploymentService::new(
            Arc::new(apply_store(
                deployment_id,
                Arc::clone(&completions),
                Arc::new(AtomicUsize::new(0)),
            )),
            Arc::new(SuccessfulApplyRuntime {
                commands: Arc::clone(&commands),
            }),
            Arc::new(AllowEntitlements),
            CancellationToken::new(),
        );

        let progress = service
            .apply(ActorId::new(Uuid::now_v7()), true, deployment_id, false)
            .await
            .unwrap();
        drop(progress);
        tokio::time::timeout(Duration::from_secs(1), async {
            while completions.load(Ordering::Relaxed) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        assert_eq!(completions.load(Ordering::Relaxed), 1);
        assert_eq!(commands.lock().unwrap()[0].deployment_id, deployment_id);
    }

    #[tokio::test]
    async fn timed_out_apply_leaves_the_claim_for_bounded_reconciliation() {
        let deployment_id = Uuid::now_v7();
        let failures = Arc::new(AtomicUsize::new(0));
        let service = DeploymentService::new(
            Arc::new(apply_store(
                deployment_id,
                Arc::new(AtomicUsize::new(0)),
                Arc::clone(&failures),
            )),
            Arc::new(PendingApplyRuntime),
            Arc::new(AllowEntitlements),
            CancellationToken::new(),
        )
        .with_apply_timeout(Duration::from_millis(5));

        let mut progress = service
            .apply(ActorId::new(Uuid::now_v7()), true, deployment_id, false)
            .await
            .unwrap();
        let mut terminal = None;
        while let Some(item) = progress.recv().await {
            terminal = item.error_message;
        }

        assert_eq!(failures.load(Ordering::Relaxed), 0);
        assert!(terminal.is_some_and(|message| message.contains("timed out")));
    }

    #[tokio::test]
    async fn stale_apply_reconciliation_finishes_only_a_running_container() {
        let deployment_id = Uuid::now_v7();
        let completions = Arc::new(AtomicUsize::new(0));
        let failures = Arc::new(AtomicUsize::new(0));
        let service = DeploymentService::new(
            Arc::new(apply_store(
                deployment_id,
                Arc::clone(&completions),
                Arc::clone(&failures),
            )),
            Arc::new(ObservedApplyRuntime(Some(RuntimeDeploymentResult {
                docker_container_id: "recovered".to_owned(),
                docker_image_id: "sha256:image".to_owned(),
                state: RuntimeContainerState::Running,
            }))),
            Arc::new(AllowEntitlements),
            CancellationToken::new(),
        );

        assert_eq!(service.reconcile_stale_applies(1, 10).await.unwrap(), 1);
        assert_eq!(completions.load(Ordering::Relaxed), 1);
        assert_eq!(failures.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn stale_apply_reconciliation_fails_an_unconfirmed_runtime_outcome() {
        let deployment_id = Uuid::now_v7();
        let completions = Arc::new(AtomicUsize::new(0));
        let failures = Arc::new(AtomicUsize::new(0));
        let service = DeploymentService::new(
            Arc::new(apply_store(
                deployment_id,
                Arc::clone(&completions),
                Arc::clone(&failures),
            )),
            Arc::new(ObservedApplyRuntime(None)),
            Arc::new(AllowEntitlements),
            CancellationToken::new(),
        );

        assert_eq!(service.reconcile_stale_applies(1, 10).await.unwrap(), 1);
        assert_eq!(completions.load(Ordering::Relaxed), 0);
        assert_eq!(failures.load(Ordering::Relaxed), 1);
    }

    fn apply_store(
        deployment_id: Uuid,
        completions: Arc<AtomicUsize>,
        failures: Arc<AtomicUsize>,
    ) -> TimeoutStore {
        TimeoutStore {
            claim: DeletionClaim {
                id: deployment_id,
                platform_id: Uuid::now_v7(),
                name: "web".to_owned(),
                docker_container_ids: Vec::new(),
                row_version: 1,
                previous_status: "Created".to_owned(),
                description: None,
                spec: spec(),
            },
            releases: Arc::new(AtomicUsize::new(0)),
            apply_claim: Some(ApplyClaim {
                id: deployment_id,
                platform_id: Uuid::now_v7(),
                platform_address: "local".to_owned(),
                name: "web".to_owned(),
                row_version: 1,
                description: None,
                spec: spec(),
                existing_container_id: None,
                existing_docker_container_id: None,
            }),
            apply_completions: completions,
            apply_failures: failures,
        }
    }

    #[tokio::test]
    async fn create_rejects_new_auto_deploy_without_its_entitlement() {
        let deployment_id = Uuid::now_v7();
        let service = DeploymentService::new(
            Arc::new(TimeoutStore {
                claim: DeletionClaim {
                    id: deployment_id,
                    platform_id: Uuid::now_v7(),
                    name: "web".to_owned(),
                    docker_container_ids: Vec::new(),
                    row_version: 1,
                    previous_status: "Created".to_owned(),
                    description: None,
                    spec: spec(),
                },
                releases: Arc::new(AtomicUsize::new(0)),
                apply_claim: None,
                apply_completions: Arc::new(AtomicUsize::new(0)),
                apply_failures: Arc::new(AtomicUsize::new(0)),
            }),
            Arc::new(PendingRuntime),
            Arc::new(DenyEntitlements),
            CancellationToken::new(),
        );
        let input = CreateDeploymentInput {
            name: "web".to_owned(),
            platform_id: Uuid::now_v7(),
            description: None,
            spec: DeploymentSpec {
                image: DeploymentImageInfo::External {
                    registry_id: Uuid::now_v7(),
                    image_tag: "nginx:latest".to_owned(),
                    resolved_digest: None,
                },
                update_behavior: UpdateBehavior::AutoDeploy,
                life_cycle_spec: None,
                resource_spec: None,
                labels: None,
                ports: None,
                volumes: None,
                networks: None,
                command: None,
                environment_variables: None,
            },
            tag_ids: Vec::new(),
            duplicate_source: None,
        };

        assert!(matches!(
            service
                .create(ActorId::new(Uuid::now_v7()), true, input)
                .await,
            Err(DeploymentError::LicenseRequired("operational-guardrails"))
        ));
    }
}
