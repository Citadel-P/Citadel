use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use citadel_domain::{ActorId, LicenseCapability};
use futures_util::future::BoxFuture;
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    CreateDeploymentInput, DeletionClaim, DeploymentConfigView, DeploymentDuplicateDraftView,
    DeploymentError, DeploymentFilter, DeploymentSpec, DeploymentView, DeploymentsView, FieldPatch,
    PatchDeploymentMetadataInput, ResourceCapabilities,
};

pub trait DeploymentStore: Send + Sync {
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
}

pub trait DeploymentRuntimePort: Send + Sync {
    fn delete_container<'a>(
        &'a self,
        platform_id: Uuid,
        docker_container_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), DeploymentError>>;
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
    shutdown: CancellationToken,
    delete_timeout: Duration,
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
            shutdown,
            delete_timeout: Duration::from_secs(30),
        }
    }

    #[must_use]
    pub fn with_notifier(mut self, notifier: Arc<dyn DeploymentChangeNotifier>) -> Self {
        self.notifier = notifier;
        self
    }

    #[must_use]
    pub fn with_delete_timeout(mut self, timeout: Duration) -> Self {
        self.delete_timeout = timeout;
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

    struct TimeoutStore {
        claim: DeletionClaim,
        releases: Arc<AtomicUsize>,
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
