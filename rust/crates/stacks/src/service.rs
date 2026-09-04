use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use tokio::sync::{Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    ApplyStackInput, ComposeModel, ComposeProjectImportDraftView, ComposeProjectImportSourceView,
    ComposeProjectImportValidation, ComposeProjectServiceComparison, ComposeProjectStackDraftView,
    CreateStackInput, ImportComposeProjectInput, PatchStackInput, ResolvedStackBindings,
    ResourceCapabilities, RollbackStackInput, StackAction, StackAdoptionIssue, StackConfigView,
    StackDeletionClaim, StackDrift, StackDriftReport, StackError, StackFilter, StackImportClaim,
    StackOperationClaim, StackOrchestrationMode, StackReleaseStatus, StackReleaseView,
    StackRuntimeResult, StackRuntimeSnapshot, StackSpec, StackStateClaim, StackStreamItem,
    StackView, StacksView, analyze_swarm_compatibility, compose_digest, inject_ownership_labels,
    merge_json, normalize_description, normalize_name, normalize_tags, parse_compose, validation,
};

pub trait StackStore: Send + Sync {
    fn list_authorized<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        filter: &'a StackFilter,
    ) -> BoxFuture<'a, Result<Vec<StackView>, StackError>>;
    fn get_authorized(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<StackView, StackError>>;
    fn create<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        input: &'a CreateStackInput,
    ) -> BoxFuture<'a, Result<StackView, StackError>>;
    fn update<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        expected_row_version: i64,
        input: &'a PatchStackInput,
        spec: &'a StackSpec,
    ) -> BoxFuture<'a, Result<StackView, StackError>>;
    fn update_metadata<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<StackView, StackError>>;
    fn rename<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        name: &'a str,
    ) -> BoxFuture<'a, Result<StackView, StackError>>;
    fn releases(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<StackReleaseView>, StackError>>;
    fn claim_apply(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        rollback_release_id: Option<Uuid>,
    ) -> BoxFuture<'_, Result<StackOperationClaim, StackError>>;
    fn complete_apply<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackOperationClaim,
        result: &'a StackRuntimeResult,
        bindings: &'a [crate::ResourceBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn fail_apply<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackOperationClaim,
        message: &'a str,
        unknown: bool,
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn stale_apply_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackOperationClaim)>, StackError>>;
    fn stale_delete_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackDeletionClaim)>, StackError>>;
    fn claim_delete<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StackDeletionClaim>, StackError>>;
    fn complete_delete<'a>(
        &'a self,
        actor: ActorId,
        claims: &'a [StackDeletionClaim],
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn release_delete<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), StackError>>;
    fn claim_state<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StackStateClaim>, StackError>>;
    fn complete_state<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackStateClaim,
        status: StackReleaseStatus,
        container_ids: &'a [String],
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn release_state<'a>(
        &'a self,
        claims: &'a [StackStateClaim],
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn stale_state_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackStateClaim)>, StackError>>;
    fn import<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        input: &'a ImportComposeProjectInput,
        claim: &'a StackImportClaim,
    ) -> BoxFuture<'a, Result<StackView, StackError>>;
    fn find_import_owner(
        &self,
        platform_id: Uuid,
        project_name: &str,
    ) -> BoxFuture<'_, Result<Option<Uuid>, StackError>>;
}

pub trait StackRuntimePort: Send + Sync {
    fn apply<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        compose: &'a str,
        environment: &'a [String],
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackRuntimeResult, StackError>>;
    fn observe<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<StackRuntimeResult>, StackError>>;
    fn delete<'a>(
        &'a self,
        claim: &'a StackDeletionClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn change_state<'a>(
        &'a self,
        platform_id: Uuid,
        project_name: &'a str,
        orchestration: StackOrchestrationMode,
        action: StackAction,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, StackError>>;
    fn runtime_snapshot<'a>(
        &'a self,
        platform_id: Uuid,
        project_name: &'a str,
        orchestration: StackOrchestrationMode,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackRuntimeSnapshot, StackError>>;
    fn reconcile<'a>(
        &'a self,
        platform_id: Uuid,
        drifts: &'a [StackDrift],
        policy: &'a crate::StackDriftPolicy,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<crate::StackReconciliationAction>, StackError>>;
    fn import_claim<'a>(
        &'a self,
        platform_id: Uuid,
        project_name: &'a str,
        import_kind: Option<crate::StackImportKind>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackImportClaim, StackError>>;
}

pub trait StackBindingResolverPort: Send + Sync {
    fn resolve<'a>(
        &'a self,
        stack_id: Uuid,
        names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedStackBindings, StackError>>;
}

pub trait StackChangeNotifier: Send + Sync {
    fn changed(&self, stack_id: Uuid, event: &'static str);
}

#[derive(Default)]
pub struct NoopStackChangeNotifier;

impl StackChangeNotifier for NoopStackChangeNotifier {
    fn changed(&self, _stack_id: Uuid, _event: &'static str) {}
}

pub struct StackService {
    store: Arc<dyn StackStore>,
    runtime: Arc<dyn StackRuntimePort>,
    bindings: Arc<dyn StackBindingResolverPort>,
    notifier: Arc<dyn StackChangeNotifier>,
    operations: Arc<Semaphore>,
    shutdown: CancellationToken,
    timeout: Duration,
}

impl StackService {
    #[must_use]
    pub fn new(
        store: Arc<dyn StackStore>,
        runtime: Arc<dyn StackRuntimePort>,
        bindings: Arc<dyn StackBindingResolverPort>,
        notifier: Arc<dyn StackChangeNotifier>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            store,
            runtime,
            bindings,
            notifier,
            operations: Arc::new(Semaphore::new(4)),
            shutdown,
            timeout: Duration::from_secs(15 * 60),
        }
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn list(
        &self,
        actor: ActorId,
        administrator: bool,
        filter: &StackFilter,
        capabilities: ResourceCapabilities,
    ) -> Result<StacksView, StackError> {
        Ok(StacksView {
            stacks: self
                .store
                .list_authorized(actor, administrator, filter)
                .await?,
            capabilities,
        })
    }

    pub async fn get(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<StackView, StackError> {
        self.store.get_authorized(actor, administrator, id).await
    }

    pub async fn get_config(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<StackConfigView, StackError> {
        self.get(actor, administrator, id).await.map(Into::into)
    }

    pub async fn create(
        &self,
        actor: ActorId,
        administrator: bool,
        mut input: CreateStackInput,
    ) -> Result<StackView, StackError> {
        normalize_name(&mut input.name)?;
        normalize_description(&mut input.description)?;
        if input.platform_id.is_nil() {
            return Err(validation("A Platform must be selected."));
        }
        if input.stack_source != input.spec.source() {
            return Err(validation(
                "Stack source and specification type must match.",
            ));
        }
        input.spec = input.spec.for_create();
        input.spec.validate()?;
        input.drift_policy = Some(input.drift_policy.unwrap_or_default().normalized());
        input.tag_ids = normalize_tags(&input.tag_ids);
        if input.tag_ids.len() > 100 {
            return Err(validation("A Stack cannot contain more than 100 Tags."));
        }
        let created = self.store.create(actor, administrator, &input).await?;
        self.notifier.changed(created.id, "created");
        Ok(created)
    }

    pub async fn update(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        mut input: PatchStackInput,
    ) -> Result<StackView, StackError> {
        if let Some(name) = input.name.as_mut() {
            normalize_name(name)?;
        }
        if let Some(description) = input.description.as_mut() {
            normalize_description(description)?;
        }
        let current = self.store.get_authorized(actor, administrator, id).await?;
        let expected = input.row_version.unwrap_or(current.row_version);
        let mut api = serde_json::to_value(current.spec.as_ref().ok_or_else(|| {
            StackError::Storage("Stack has no current specification.".to_owned())
        })?)
        .map_err(|error| StackError::Storage(error.to_string()))?;
        if let Some(patch) = &input.spec {
            merge_json(&mut api, patch);
        }
        let spec: StackSpec = serde_json::from_value(api)
            .map_err(|error| validation(&format!("Invalid Stack specification: {error}")))?;
        if input
            .stack_source
            .is_some_and(|source| source != spec.source())
            || current.stack_source != spec.source()
        {
            return Err(validation("A Stack source type cannot be changed."));
        }
        spec.validate()?;
        let updated = self
            .store
            .update(actor, administrator, id, expected, &input, &spec)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn update_metadata(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        mut description: Option<String>,
    ) -> Result<StackView, StackError> {
        normalize_description(&mut description)?;
        let updated = self
            .store
            .update_metadata(actor, administrator, id, description.as_deref())
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn rename(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        mut name: String,
    ) -> Result<StackView, StackError> {
        normalize_name(&mut name)?;
        let updated = self.store.rename(actor, administrator, id, &name).await?;
        self.notifier.changed(id, "renamed");
        Ok(updated)
    }

    pub async fn releases(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<Vec<StackReleaseView>, StackError> {
        self.store.releases(actor, administrator, id).await
    }

    pub async fn duplicate_draft(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<crate::StackDuplicateDraftView, StackError> {
        let stack = self.store.get_authorized(actor, administrator, id).await?;
        let mut spec = stack.spec.clone().ok_or(StackError::NotFound)?.for_create();
        spec.common_mut().project_name = None;
        Ok(crate::StackDuplicateDraftView {
            draft: serde_json::json!({
                "name": format!("{} Copy", stack.name),
                "platformId": stack.platform_id.ok_or(StackError::NotFound)?,
                "description": stack.description,
                "stackSource": stack.stack_source,
                "spec": spec,
                "driftPolicy": stack.drift_policy,
                "tagIds": stack.tags.into_iter().map(|tag| tag.id).collect::<Vec<_>>(),
                "duplicateSource": {
                    "resourceType": "Stack",
                    "resourceId": stack.id,
                    "resourceName": stack.name,
                }
            }),
            warnings: Vec::new(),
        })
    }

    pub async fn apply(
        &self,
        actor: ActorId,
        administrator: bool,
        input: ApplyStackInput,
    ) -> Result<mpsc::Receiver<StackStreamItem>, StackError> {
        self.start_apply(actor, administrator, input.id, None).await
    }

    pub async fn rollback(
        &self,
        actor: ActorId,
        administrator: bool,
        input: RollbackStackInput,
    ) -> Result<mpsc::Receiver<StackStreamItem>, StackError> {
        self.start_apply(actor, administrator, input.stack_id, Some(input.release_id))
            .await
    }

    async fn start_apply(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        rollback: Option<Uuid>,
    ) -> Result<mpsc::Receiver<StackStreamItem>, StackError> {
        let permit = self.operations.clone().try_acquire_owned().map_err(|_| {
            StackError::Conflict("Too many Stack operations are already running.".to_owned())
        })?;
        let claim = self
            .store
            .claim_apply(actor, administrator, id, rollback)
            .await?;
        let runtime = Arc::clone(&self.runtime);
        let store = Arc::clone(&self.store);
        let bindings = Arc::clone(&self.bindings);
        let notifier = Arc::clone(&self.notifier);
        let cancellation = self.shutdown.child_token();
        let timeout = self.timeout;
        let (sender, receiver) = mpsc::channel(32);
        tokio::spawn(async move {
            let _permit = permit;
            execute_apply(
                runtime,
                store,
                bindings,
                notifier,
                actor,
                claim,
                cancellation,
                timeout,
                sender,
            )
            .await;
        });
        Ok(receiver)
    }

    pub async fn delete(
        &self,
        actor: ActorId,
        administrator: bool,
        ids: &[Uuid],
    ) -> Result<(), StackError> {
        if ids.is_empty() || ids.len() > 100 {
            return Err(validation("Between 1 and 100 Stack IDs are required."));
        }
        let mut unique = ids.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let claims = self
            .store
            .claim_delete(actor, administrator, &unique)
            .await?;
        let cancellation = self.shutdown.child_token();
        for (index, claim) in claims.iter().enumerate() {
            let outcome = tokio::time::timeout(
                self.timeout.min(Duration::from_secs(120)),
                self.runtime.delete(claim, &cancellation),
            )
            .await;
            match outcome {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    let pending = claims[index..]
                        .iter()
                        .map(|claim| claim.stack_id)
                        .collect::<Vec<_>>();
                    self.store.release_delete(&pending).await?;
                    return Err(error);
                }
                Err(_) => {
                    let unstarted = claims[index + 1..]
                        .iter()
                        .map(|claim| claim.stack_id)
                        .collect::<Vec<_>>();
                    if !unstarted.is_empty() {
                        self.store.release_delete(&unstarted).await?;
                    }
                    return Err(StackError::Runtime(
                        "Timed out while removing Stack runtime resources. Citadel will reconcile the claimed deletion.".to_owned(),
                    ));
                }
            }
            self.store
                .complete_delete(actor, std::slice::from_ref(claim))
                .await?;
            self.notifier.changed(claim.stack_id, "deleted");
        }
        Ok(())
    }

    pub async fn change_state(
        &self,
        actor: ActorId,
        administrator: bool,
        ids: &[Uuid],
        action: StackAction,
    ) -> Result<(), StackError> {
        let ids = ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if ids.is_empty() || ids.len() > 100 {
            return Err(validation("Between 1 and 100 Stack IDs are required."));
        }
        let Ok(_permit) = self.operations.clone().acquire_owned().await else {
            return Err(StackError::Cancelled);
        };
        let claims = self.store.claim_state(actor, administrator, &ids).await?;
        let cancellation = self.shutdown.child_token();
        for (index, claim) in claims.iter().enumerate() {
            let outcome = tokio::time::timeout(
                self.timeout.min(Duration::from_secs(120)),
                self.runtime.change_state(
                    claim.platform_id,
                    &claim.project_name,
                    orchestration(&claim.platform_type)?,
                    action,
                    &cancellation,
                ),
            )
            .await;
            match outcome {
                Ok(Ok(container_ids)) => {
                    self.store
                        .complete_state(actor, claim, state_action_status(action), &container_ids)
                        .await?;
                    self.notifier.changed(claim.stack_id, "stateChanged");
                }
                Ok(Err(error)) => {
                    let ambiguous = matches!(error, StackError::Runtime(_) | StackError::Cancelled);
                    let release_from = if ambiguous { index + 1 } else { index };
                    if release_from < claims.len() {
                        self.store.release_state(&claims[release_from..]).await?;
                    }
                    return Err(error);
                }
                Err(_) => {
                    if index + 1 < claims.len() {
                        self.store.release_state(&claims[index + 1..]).await?;
                    }
                    return Err(StackError::Runtime(
                        "The Stack state operation timed out. Citadel will reconcile its runtime outcome."
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub async fn preflight_swarm(
        &self,
        compose_files: &[String],
        bindings: &[crate::StackBuildImageBinding],
    ) -> Result<crate::SwarmStackCompatibilityReport, StackError> {
        analyze_swarm_compatibility(compose_files, bindings)
    }

    pub async fn import_draft(
        &self,
        platform_id: Uuid,
        project_name: &str,
        import_kind: Option<crate::StackImportKind>,
    ) -> Result<ComposeProjectImportDraftView, StackError> {
        let claim = self
            .runtime
            .import_claim(
                platform_id,
                project_name,
                import_kind,
                &self.shutdown.child_token(),
            )
            .await?;
        let existing = self
            .store
            .find_import_owner(platform_id, project_name)
            .await?;
        let mut issues = Vec::new();
        if existing.is_some() {
            issues.push(StackAdoptionIssue {
                code: "ExistingCitadelOwnership".to_owned(),
                message: "This runtime project is already owned by a Citadel Stack.".to_owned(),
                severity: "Error".to_owned(),
                field_path: None,
            });
        }
        let runtime_fingerprint = import_runtime_fingerprint(&claim);
        Ok(ComposeProjectImportDraftView {
            import_kind: claim.import_kind,
            source: ComposeProjectImportSourceView {
                platform_id,
                platform_name: claim.platform_name,
                project_name: project_name.to_owned(),
                container_ids: claim.container_ids,
                container_names: claim.container_names,
                services: claim.services,
            },
            draft: ComposeProjectStackDraftView {
                name: project_name.to_owned(),
                platform_id,
                description: Some(format!("Imported from Docker project '{project_name}'.")),
                drift_policy: crate::StackDriftPolicy::default(),
                tag_ids: Vec::new(),
            },
            issues,
            runtime_fingerprint,
        })
    }

    pub async fn validate_import(
        &self,
        platform_id: Uuid,
        project_name: &str,
        name: &str,
        stack_source: crate::StackSource,
        spec: &StackSpec,
        import_kind: Option<crate::StackImportKind>,
    ) -> Result<ComposeProjectImportValidation, StackError> {
        let mut name = name.to_owned();
        normalize_name(&mut name)?;
        if spec.source() != stack_source {
            return Err(validation(
                "Stack source and specification type must match.",
            ));
        }
        spec.validate()?;
        let claim = self
            .runtime
            .import_claim(
                platform_id,
                project_name,
                import_kind,
                &self.shutdown.child_token(),
            )
            .await?;
        if import_kind.is_some_and(|kind| kind != claim.import_kind) {
            return Err(validation(
                "The requested import kind does not match the Platform runtime.",
            ));
        }
        let compose = spec.compose_file().ok_or_else(|| {
            validation("Git Stack import validation requires Phase 7 source materialization.")
        })?;
        let desired = parse_compose(&[compose.to_owned()])?;
        let desired_by_name = desired
            .services
            .iter()
            .map(|service| (service.name.as_str(), service.image.clone()))
            .collect::<BTreeMap<_, _>>();
        let runtime_by_name = claim
            .services
            .iter()
            .map(|service| (service.name.as_str(), service))
            .collect::<BTreeMap<_, _>>();
        let names = desired_by_name
            .keys()
            .chain(runtime_by_name.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        let services: Vec<ComposeProjectServiceComparison> = names
            .iter()
            .map(|name| {
                let runtime = runtime_by_name.get(name);
                ComposeProjectServiceComparison {
                    name: (*name).to_owned(),
                    runtime_container_count: runtime.map_or(0, |value| value.container_count),
                    runtime_image: runtime.and_then(|value| value.image.clone()),
                    defined_in_source: desired_by_name.contains_key(name),
                    source_image: desired_by_name.get(name).cloned().flatten(),
                }
            })
            .collect();
        let mut issues = Vec::new();
        if claim.import_kind == crate::StackImportKind::SwarmStack {
            let compatibility = analyze_swarm_compatibility(
                &[compose.to_owned()],
                &spec.common().build_image_bindings,
            )?;
            issues.extend(compatibility.issues.into_iter().map(|issue| {
                StackAdoptionIssue {
                    code: issue.code,
                    message: issue.message,
                    severity: match issue.severity {
                        crate::SwarmStackCompatibilitySeverity::Warning => "Warning",
                        crate::SwarmStackCompatibilitySeverity::Error => "Error",
                    }
                    .to_owned(),
                    field_path: issue.field_path,
                }
            }));
        }
        for comparison in &services {
            if !comparison.defined_in_source {
                issues.push(StackAdoptionIssue {
                    code: "MissingSourceService".to_owned(),
                    message: format!(
                        "Runtime Service '{}' is not defined by the selected source.",
                        comparison.name
                    ),
                    severity: "Error".to_owned(),
                    field_path: Some(format!("services.{}", comparison.name)),
                });
            }
        }
        let spec_json =
            serde_json::to_string(spec).map_err(|error| StackError::Storage(error.to_string()))?;
        let preview_fingerprint = compose_digest(&[import_runtime_fingerprint(&claim), spec_json]);
        Ok(ComposeProjectImportValidation {
            services,
            issues,
            preview_fingerprint,
            importable_sensitive_environment_names: Vec::new(),
            can_import_sensitive_environment_values: false,
        })
    }

    pub async fn import(
        &self,
        actor: ActorId,
        administrator: bool,
        mut input: ImportComposeProjectInput,
    ) -> Result<StackView, StackError> {
        normalize_name(&mut input.name)?;
        normalize_description(&mut input.description)?;
        input.spec.validate()?;
        let claim = self
            .runtime
            .import_claim(
                input.platform_id,
                &input.project_name,
                Some(input.import_kind),
                &self.shutdown.child_token(),
            )
            .await?;
        let spec_json = serde_json::to_string(&input.spec)
            .map_err(|error| StackError::Storage(error.to_string()))?;
        let expected_fingerprint = compose_digest(&[import_runtime_fingerprint(&claim), spec_json]);
        if expected_fingerprint != input.preview_fingerprint
            || claim.import_kind != input.import_kind
        {
            return Err(StackError::Conflict(
                "The runtime Stack changed after it was reviewed. Refresh the import draft."
                    .to_owned(),
            ));
        }
        let imported = self
            .store
            .import(actor, administrator, &input, &claim)
            .await?;
        self.notifier.changed(imported.id, "imported");
        Ok(imported)
    }

    pub async fn drift(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<StackDriftReport, StackError> {
        let stack = self.store.get_authorized(actor, administrator, id).await?;
        let platform_id = stack.platform_id.ok_or(StackError::NotFound)?;
        let project = project_name(&stack)?;
        let runtime = self
            .runtime
            .runtime_snapshot(
                platform_id,
                &project,
                orchestration(&stack.platform_type)?,
                &self.shutdown.child_token(),
            )
            .await?;
        calculate_drift(&stack, &runtime)
    }

    pub async fn update_drift_policy(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        policy: crate::StackDriftPolicy,
    ) -> Result<StackView, StackError> {
        let current = self.store.get_authorized(actor, administrator, id).await?;
        let spec = current.spec.clone().ok_or(StackError::NotFound)?;
        let input = PatchStackInput {
            name: None,
            platform_id: None,
            description: None,
            stack_source: None,
            spec: None,
            drift_policy: Some(policy.normalized()),
            row_version: Some(current.row_version),
        };
        let updated = self
            .store
            .update(actor, administrator, id, current.row_version, &input, &spec)
            .await?;
        self.notifier.changed(id, "driftPolicyUpdated");
        Ok(updated)
    }

    pub async fn reconcile_drift(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<crate::StackReconciliationResult, StackError> {
        let stack = self.store.get_authorized(actor, administrator, id).await?;
        if stack.control_state == "Processing"
            || matches!(
                stack.status,
                StackReleaseStatus::Applying | StackReleaseStatus::Pending
            )
        {
            return Err(StackError::Conflict(
                "The Stack is currently applying and cannot be reconciled.".to_owned(),
            ));
        }
        if stack.drift_policy.mode != crate::StackDriftMode::AutoFix {
            return Err(validation("Stack drift auto-fix is not enabled."));
        }
        let before = self.drift(actor, administrator, id).await?;
        if !before.has_drift {
            return Ok(crate::StackReconciliationResult {
                stack_id: id,
                status: crate::StackReconciliationStatus::NoDrift,
                before_report: before,
                after_report: None,
                actions: Vec::new(),
            });
        }
        let actions = self
            .runtime
            .reconcile(
                stack.platform_id.ok_or(StackError::NotFound)?,
                &before.drifts,
                &stack.drift_policy,
                &self.shutdown.child_token(),
            )
            .await?;
        let after = if actions.is_empty() {
            None
        } else {
            Some(self.drift(actor, administrator, id).await?)
        };
        let status = if after.as_ref().is_some_and(|report| !report.has_drift)
            && actions.iter().all(|action| action.succeeded)
        {
            crate::StackReconciliationStatus::Reconciled
        } else if after
            .as_ref()
            .map_or(before.has_structural_drift, |report| {
                report.has_structural_drift
            })
        {
            crate::StackReconciliationStatus::RequiresReapply
        } else {
            crate::StackReconciliationStatus::Partial
        };
        self.notifier.changed(id, "reconciled");
        Ok(crate::StackReconciliationResult {
            stack_id: id,
            status,
            before_report: before,
            after_report: after,
            actions,
        })
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
            let result = tokio::time::timeout(
                Duration::from_secs(30),
                self.runtime.observe(&claim, &self.shutdown.child_token()),
            )
            .await;
            match result {
                Ok(Ok(Some(result))) if result.status == StackReleaseStatus::Healthy => {
                    self.store
                        .complete_apply(actor, &claim, &result, &[])
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
                        orchestration(&claim.platform_type)?,
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
                let orchestration = orchestration(&claim.platform_type)?;
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

const fn state_action_status(action: StackAction) -> StackReleaseStatus {
    match action {
        StackAction::Start | StackAction::Resume | StackAction::Restart => {
            StackReleaseStatus::Healthy
        }
        StackAction::Stop => StackReleaseStatus::Stopped,
        StackAction::Pause => StackReleaseStatus::Paused,
    }
}

fn stable_runtime_status(snapshot: &StackRuntimeSnapshot) -> Option<StackReleaseStatus> {
    if snapshot.containers.is_empty() {
        return None;
    }
    if snapshot
        .containers
        .iter()
        .all(|container| container.state.eq_ignore_ascii_case("paused"))
    {
        return Some(StackReleaseStatus::Paused);
    }
    if snapshot.containers.iter().all(|container| {
        container.state.eq_ignore_ascii_case("exited")
            || container.state.eq_ignore_ascii_case("offline")
            || container.state.eq_ignore_ascii_case("stopped")
    }) {
        return Some(StackReleaseStatus::Stopped);
    }
    if snapshot.containers.iter().all(|container| {
        container.state.eq_ignore_ascii_case("running")
            && !container
                .health
                .as_deref()
                .is_some_and(|health| health.eq_ignore_ascii_case("unhealthy"))
    }) {
        return Some(StackReleaseStatus::Healthy);
    }
    None
}

#[allow(clippy::too_many_arguments)]
async fn execute_apply(
    runtime: Arc<dyn StackRuntimePort>,
    store: Arc<dyn StackStore>,
    bindings: Arc<dyn StackBindingResolverPort>,
    notifier: Arc<dyn StackChangeNotifier>,
    actor: ActorId,
    claim: StackOperationClaim,
    cancellation: CancellationToken,
    timeout: Duration,
    sender: mpsc::Sender<StackStreamItem>,
) {
    let _ = sender
        .send(StackStreamItem::system(
            "Resolving Stack variables and secrets...",
        ))
        .await;
    let compose = match claim.spec.compose_file() {
        Some(value) => value.to_owned(),
        None => {
            let message = "Git Stack source materialization belongs to Phase 7 and is unavailable.";
            let _ = store.fail_apply(actor, &claim, message, false).await;
            let _ = sender
                .send(StackStreamItem::completed(
                    StackReleaseStatus::Failed,
                    message,
                ))
                .await;
            return;
        }
    };
    let model = match parse_compose(std::slice::from_ref(&compose)) {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            let _ = store.fail_apply(actor, &claim, &message, false).await;
            let _ = sender
                .send(StackStreamItem::completed(
                    StackReleaseStatus::Failed,
                    message,
                ))
                .await;
            return;
        }
    };
    let names = model.variables.iter().cloned().collect::<Vec<_>>();
    let resolved = match bindings.resolve(claim.stack_id, &names).await {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            let _ = store.fail_apply(actor, &claim, &message, false).await;
            let _ = sender
                .send(StackStreamItem::completed(
                    StackReleaseStatus::Failed,
                    message,
                ))
                .await;
            return;
        }
    };
    let (environment, redactions) = match resolve_environment(&model, &resolved) {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            let _ = store.fail_apply(actor, &claim, &message, false).await;
            let _ = sender
                .send(StackStreamItem::completed(
                    StackReleaseStatus::Failed,
                    message,
                ))
                .await;
            return;
        }
    };
    let compose = match inject_ownership_labels(
        &compose,
        claim.stack_id,
        claim.release_id,
        claim.platform_type == "DockerSwarm",
    ) {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            let _ = store.fail_apply(actor, &claim, &message, false).await;
            let _ = sender
                .send(StackStreamItem::completed(
                    StackReleaseStatus::Failed,
                    message,
                ))
                .await;
            return;
        }
    };
    let _ = sender
        .send(StackStreamItem::system(
            "Submitting Stack deployment to Docker...",
        ))
        .await;
    match tokio::time::timeout(
        timeout,
        runtime.apply(&claim, &compose, &environment, &cancellation),
    )
    .await
    {
        Ok(Ok(result)) => {
            for item in &result.messages {
                let _ = sender.send(item.clone()).await;
            }
            if result.status == StackReleaseStatus::Healthy {
                let snapshots = resolved
                    .entries
                    .iter()
                    .map(|entry| entry.snapshot.clone())
                    .collect::<Vec<_>>();
                if let Err(error) = store
                    .complete_apply(actor, &claim, &result, &snapshots)
                    .await
                {
                    let _ = sender
                        .send(StackStreamItem::completed(
                            StackReleaseStatus::Failed,
                            error.to_string(),
                        ))
                        .await;
                    return;
                }
                notifier.changed(claim.stack_id, "applied");
            } else {
                let message = result
                    .messages
                    .last()
                    .and_then(|item| item.message.as_deref())
                    .unwrap_or("Stack deployment failed.");
                let _ = store.fail_apply(actor, &claim, message, false).await;
                notifier.changed(claim.stack_id, "failed");
            }
            let _ = sender
                .send(StackStreamItem::completed(
                    result.status,
                    if result.status == StackReleaseStatus::Healthy {
                        "Stack deployment completed."
                    } else {
                        "Stack deployment failed."
                    },
                ))
                .await;
        }
        Ok(Err(error)) => {
            let unknown = matches!(error, StackError::Runtime(_) | StackError::Cancelled);
            let message = redact(error.to_string(), &redactions);
            let _ = store.fail_apply(actor, &claim, &message, unknown).await;
            notifier.changed(
                claim.stack_id,
                if unknown { "outcomeUnknown" } else { "failed" },
            );
            let _ = sender
                .send(StackStreamItem::completed(
                    StackReleaseStatus::Failed,
                    message,
                ))
                .await;
        }
        Err(_) => {
            let message = "Stack operation timed out before its Docker outcome could be confirmed.";
            let _ = store.fail_apply(actor, &claim, message, true).await;
            notifier.changed(claim.stack_id, "outcomeUnknown");
            let _ = sender
                .send(StackStreamItem::completed(
                    StackReleaseStatus::TimedOut,
                    message,
                ))
                .await;
        }
    }
}

fn resolve_environment(
    model: &ComposeModel,
    resolved: &ResolvedStackBindings,
) -> Result<(Vec<String>, Vec<String>), StackError> {
    let mut values = BTreeMap::new();
    for binding in &resolved.entries {
        values.insert(binding.name.to_ascii_lowercase(), binding);
    }
    let mut environment = Vec::with_capacity(model.variables.len());
    let mut redactions = Vec::new();
    for name in &model.variables {
        let Some(binding) = values.get(&name.to_ascii_lowercase()) else {
            if model.optional_variables.contains(name) {
                continue;
            }
            return Err(validation(&format!(
                "Stack references undefined Citadel variable or secret '{name}'."
            )));
        };
        environment.push(format!("{name}={}", binding.value.as_str()));
        if binding.secret {
            redactions.push(binding.value.to_string());
        }
    }
    Ok((environment, redactions))
}

fn redact(mut message: String, secrets: &[String]) -> String {
    for value in secrets.iter().filter(|value| !value.is_empty()) {
        message = message.replace(value, "********");
    }
    message
}

fn orchestration(platform_type: &str) -> Result<StackOrchestrationMode, StackError> {
    match platform_type {
        "Docker" => Ok(StackOrchestrationMode::DockerCompose),
        "DockerSwarm" => Ok(StackOrchestrationMode::DockerSwarm),
        _ => Err(validation(
            "Stacks require a Docker or Docker Swarm Platform.",
        )),
    }
}

fn project_name(stack: &StackView) -> Result<String, StackError> {
    if let Some(project) = stack
        .spec
        .as_ref()
        .and_then(|spec| spec.common().project_name.clone())
    {
        return Ok(project);
    }
    let normalized = stack
        .name
        .to_ascii_lowercase()
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || value == '-' || value == '_' {
                value
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if normalized.is_empty() {
        Ok(format!("stack-{}", stack.id.simple()))
    } else {
        Ok(normalized)
    }
}

pub fn calculate_drift(
    stack: &StackView,
    runtime: &StackRuntimeSnapshot,
) -> Result<StackDriftReport, StackError> {
    if stack.drift_policy.mode == crate::StackDriftMode::Disabled
        || matches!(
            stack.status,
            StackReleaseStatus::Stopped | StackReleaseStatus::Paused
        )
    {
        return Ok(StackDriftReport {
            stack_id: stack.id,
            platform_id: stack.platform_id.ok_or(StackError::NotFound)?,
            has_drift: false,
            has_auto_fixable_drift: false,
            has_structural_drift: false,
            drifts: Vec::new(),
        });
    }
    let spec = stack.spec.as_ref().ok_or(StackError::NotFound)?;
    let compose = spec
        .compose_file()
        .ok_or_else(|| validation("Git Stack drift requires materialized source."))?;
    let desired = parse_compose(&[compose.to_owned()])?;
    let desired_names = desired
        .services
        .iter()
        .map(|service| service.name.as_str())
        .collect::<BTreeSet<_>>();
    let actual_names = runtime
        .containers
        .iter()
        .map(|container| container.service_name.as_str())
        .chain(runtime.services.iter().map(|service| {
            service
                .name
                .rsplit_once('_')
                .map_or(service.name.as_str(), |(_, name)| name)
        }))
        .collect::<BTreeSet<_>>();
    let mut drifts = Vec::new();
    for missing in desired_names.difference(&actual_names) {
        drifts.push(StackDrift::MissingContainer {
            service_name: (*missing).to_owned(),
        });
    }
    for extra in actual_names.difference(&desired_names) {
        for container in runtime
            .containers
            .iter()
            .filter(|item| item.service_name == *extra)
        {
            drifts.push(StackDrift::ExtraContainer {
                container_id: container.docker_container_id.clone(),
                service_name: (*extra).to_owned(),
            });
        }
    }
    for container in runtime
        .containers
        .iter()
        .filter(|item| desired_names.contains(item.service_name.as_str()))
    {
        match container.state.as_str() {
            "exited" | "offline" => drifts.push(StackDrift::ContainerStopped {
                container_id: container.docker_container_id.clone(),
                service_name: container.service_name.clone(),
            }),
            "paused" => drifts.push(StackDrift::ContainerPaused {
                container_id: container.docker_container_id.clone(),
                service_name: container.service_name.clone(),
            }),
            _ if container
                .health
                .as_deref()
                .is_some_and(|health| health == "unhealthy") =>
            {
                drifts.push(StackDrift::ContainerUnhealthy {
                    container_id: container.docker_container_id.clone(),
                    service_name: container.service_name.clone(),
                    health_status: container.health.clone(),
                })
            }
            _ => {}
        }
    }
    let has_structural = drifts
        .iter()
        .any(|drift| matches!(drift, StackDrift::MissingContainer { .. }));
    let has_auto = drifts.iter().any(|drift| match drift {
        StackDrift::ContainerStopped { .. } => stack.drift_policy.auto_start_stopped_containers,
        StackDrift::ContainerPaused { .. } => stack.drift_policy.auto_resume_paused_containers,
        StackDrift::ExtraContainer { .. } => stack.drift_policy.remove_extra_containers,
        _ => false,
    });
    Ok(StackDriftReport {
        stack_id: stack.id,
        platform_id: stack.platform_id.ok_or(StackError::NotFound)?,
        has_drift: !drifts.is_empty(),
        has_auto_fixable_drift: has_auto,
        has_structural_drift: has_structural,
        drifts,
    })
}

#[must_use]
pub fn import_runtime_fingerprint(claim: &StackImportClaim) -> String {
    compose_digest(&[
        claim.platform_id.to_string(),
        claim.project_name.clone(),
        format!("{:?}", claim.import_kind),
        claim.service_names.join("\n"),
        claim.runtime_fingerprint.clone(),
    ])
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::{RecreateStackOnNewImageState, StackDriftPolicy, StackSource, StackUpdateState};

    fn stack(status: StackReleaseStatus, policy: StackDriftPolicy) -> StackView {
        let id = Uuid::now_v7();
        StackView {
            id,
            name: "demo".to_owned(),
            description: None,
            stack_source: StackSource::WebEditor,
            stack_update_state: StackUpdateState::WebEditor {
                recreate_stack_on_new_image_state: RecreateStackOnNewImageState::default(),
            },
            drift_policy: policy,
            status,
            created_at: Utc::now(),
            created_by_actor_id: Uuid::now_v7(),
            control_state: "Idle".to_owned(),
            current_stack_release_id: Uuid::now_v7(),
            platform_type: "Docker".to_owned(),
            platform_id: Some(Uuid::now_v7()),
            version: Some("1".to_owned()),
            spec: Some(StackSpec::WebEditor {
                compose_file: "services:\n  api:\n    image: nginx\n".to_owned(),
                update_behavior: crate::StackUpdateBehavior::Disabled,
                common: Default::default(),
            }),
            source: None,
            resource_bindings: None,
            platform_status: "Online".to_owned(),
            platform_name: Some("local".to_owned()),
            tags: Vec::new(),
            latest_activity_view: None,
            capabilities: None,
            row_version: 0,
        }
    }

    #[test]
    fn drift_reports_missing_extra_stopped_and_paused_containers() {
        let policy = StackDriftPolicy {
            auto_start_stopped_containers: true,
            auto_resume_paused_containers: true,
            remove_extra_containers: true,
            ..Default::default()
        };
        let stack = stack(StackReleaseStatus::Healthy, policy);
        let report = calculate_drift(
            &stack,
            &StackRuntimeSnapshot {
                containers: vec![crate::StackRuntimeContainer {
                    docker_container_id: "old".to_owned(),
                    service_name: "old".to_owned(),
                    state: "running".to_owned(),
                    health: None,
                }],
                services: Vec::new(),
            },
        )
        .unwrap();
        assert!(report.has_structural_drift);
        assert!(report.has_auto_fixable_drift);
        assert_eq!(report.drifts.len(), 2);
    }

    #[test]
    fn intentionally_stopped_stack_has_no_drift() {
        let stack = stack(StackReleaseStatus::Stopped, StackDriftPolicy::default());
        let report = calculate_drift(
            &stack,
            &StackRuntimeSnapshot {
                containers: Vec::new(),
                services: Vec::new(),
            },
        )
        .unwrap();
        assert!(!report.has_drift);
    }

    #[test]
    fn stale_state_recovery_only_accepts_stable_container_aggregates() {
        let snapshot = |states: &[(&str, Option<&str>)]| StackRuntimeSnapshot {
            containers: states
                .iter()
                .enumerate()
                .map(|(index, (state, health))| crate::StackRuntimeContainer {
                    docker_container_id: format!("container-{index}"),
                    service_name: format!("service-{index}"),
                    state: (*state).to_owned(),
                    health: health.map(str::to_owned),
                })
                .collect(),
            services: Vec::new(),
        };

        assert_eq!(
            stable_runtime_status(&snapshot(&[("running", Some("healthy"))])),
            Some(StackReleaseStatus::Healthy)
        );
        assert_eq!(
            stable_runtime_status(&snapshot(&[("paused", None), ("paused", None)])),
            Some(StackReleaseStatus::Paused)
        );
        assert_eq!(
            stable_runtime_status(&snapshot(&[("exited", None), ("offline", None)])),
            Some(StackReleaseStatus::Stopped)
        );
        assert_eq!(
            stable_runtime_status(&snapshot(&[("running", None), ("exited", None)])),
            None
        );
        assert_eq!(
            stable_runtime_status(&snapshot(&[("running", Some("unhealthy"))])),
            None
        );
        assert_eq!(
            stable_runtime_status(&StackRuntimeSnapshot {
                containers: Vec::new(),
                services: Vec::new(),
            }),
            None
        );
    }

    #[test]
    fn import_fingerprint_changes_with_authoritative_runtime_state() {
        let platform_id = Uuid::now_v7();
        let base = StackImportClaim {
            platform_id,
            platform_name: "swarm".to_owned(),
            project_name: "demo".to_owned(),
            import_kind: crate::StackImportKind::SwarmStack,
            runtime_fingerprint: "sha256:first".to_owned(),
            service_names: vec!["api".to_owned()],
            container_ids: Vec::new(),
            container_names: Vec::new(),
            services: vec![crate::ComposeProjectRuntimeService {
                name: "api".to_owned(),
                image: Some("nginx:1".to_owned()),
                container_count: 1,
                states: vec!["completed".to_owned(), "running:1".to_owned()],
            }],
        };
        let mut changed = base.clone();
        changed.runtime_fingerprint = "sha256:second".to_owned();
        assert_ne!(
            import_runtime_fingerprint(&base),
            import_runtime_fingerprint(&changed)
        );
    }
}
