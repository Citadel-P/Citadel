use super::apply::*;
use super::mutations::*;
use crate::{PreparedDeploymentImage, RuntimeDeploymentResult};
use futures_util::future::BoxFuture;

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

impl DeploymentRepository for TimeoutStore {
    fn list_authorized<'a>(
        &'a self,
        _: ActorId,
        _: bool,
        _: &'a DeploymentFilter,
    ) -> BoxFuture<'a, Result<Vec<DeploymentDetails>, DeploymentError>> {
        async { unreachable!() }.boxed()
    }

    fn get_authorized<'a>(
        &'a self,
        _: ActorId,
        _: bool,
        _: Uuid,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>> {
        async { unreachable!() }.boxed()
    }

    fn create<'a>(
        &'a self,
        _: ActorId,
        _: bool,
        _: &'a CreateDeployment,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>> {
        async { unreachable!() }.boxed()
    }

    fn update_config<'a>(
        &'a self,
        _: ActorId,
        _: bool,
        _: Uuid,
        _: i64,
        _: &'a DeploymentSpec,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>> {
        async { unreachable!() }.boxed()
    }

    fn update_metadata<'a>(
        &'a self,
        _: ActorId,
        _: bool,
        _: Uuid,
        _: &'a UpdateDeploymentMetadata,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>> {
        async { unreachable!() }.boxed()
    }

    fn rename<'a>(
        &'a self,
        _: ActorId,
        _: bool,
        _: Uuid,
        _: &'a str,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>> {
        async { unreachable!() }.boxed()
    }

    fn duplicate_draft<'a>(
        &'a self,
        _: ActorId,
        _: bool,
        _: Uuid,
    ) -> BoxFuture<'a, Result<DeploymentDuplicateDraft, DeploymentError>> {
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

impl DeploymentRuntime for PendingRuntime {
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

impl DeploymentRuntime for SignalledPendingRuntime {
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

impl DeploymentRuntime for SuccessfulApplyRuntime {
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

impl DeploymentRuntime for PendingApplyRuntime {
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

impl DeploymentRuntime for ObservedApplyRuntime {
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
        Arc::new(TestTasks::default()),
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

    assert!(matches!(error, DeploymentError::Runtime(message) if message.contains("timed out")));
    assert_eq!(releases.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn dropping_the_request_does_not_cancel_claim_recovery() {
    let deployment_id = Uuid::now_v7();
    let releases = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(Notify::new());
    let owner = Arc::new(TestTasks::default());
    let service = DeploymentService::new(
        owner.clone(),
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
    assert_eq!(owner.tracker.len(), 1);
    request.abort();
    tokio::time::timeout(Duration::from_secs(1), async {
        while releases.load(Ordering::Relaxed) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    owner.tracker.close();
    tokio::time::timeout(Duration::from_secs(1), owner.tracker.wait())
        .await
        .unwrap();
    assert_eq!(owner.tracker.len(), 0);
    assert_eq!(releases.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn dropping_apply_progress_does_not_cancel_the_claimed_operation() {
    let deployment_id = Uuid::now_v7();
    let completions = Arc::new(AtomicUsize::new(0));
    let commands = Arc::new(Mutex::new(Vec::new()));
    let service = DeploymentService::new(
        Arc::new(TestTasks::default()),
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
        Arc::new(TestTasks::default()),
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
        Arc::new(TestTasks::default()),
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
        Arc::new(TestTasks::default()),
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
        Arc::new(TestTasks::default()),
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
    let input = CreateDeployment {
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

#[derive(Default)]
struct TestTasks {
    reject: bool,
    tracker: tokio_util::task::TaskTracker,
}
impl crate::DeploymentTaskSpawner for TestTasks {
    fn spawn(
        &self,
        name: &'static str,
        operation: BoxFuture<'static, Result<(), DeploymentError>>,
    ) -> bool {
        if self.reject {
            return false;
        }
        self.tracker.spawn(async move {
            if let Err(error) = operation.await {
                tracing::error!(name,%error,"Deployment test task failed");
            }
        });
        true
    }
}

#[tokio::test]
async fn rejected_task_admission_releases_delete_and_apply_claims_without_runtime_work() {
    let id = Uuid::now_v7();
    let completions = Arc::new(AtomicUsize::new(0));
    let failures = Arc::new(AtomicUsize::new(0));
    let store = Arc::new(apply_store(id, completions.clone(), failures.clone()));
    let releases = store.releases.clone();
    let service = DeploymentService::new(
        Arc::new(TestTasks {
            reject: true,
            ..Default::default()
        }),
        store,
        Arc::new(PendingRuntime),
        Arc::new(AllowEntitlements),
        CancellationToken::new(),
    );
    let actor = ActorId::new(Uuid::now_v7());
    assert!(matches!(
        service.delete(actor, true, vec![id]).await,
        Err(DeploymentError::Cancelled)
    ));
    assert_eq!(releases.load(Ordering::Relaxed), 1);
    assert!(matches!(
        service.apply(actor, true, id, false).await,
        Err(DeploymentError::Cancelled)
    ));
    assert_eq!(failures.load(Ordering::Relaxed), 1);
    assert_eq!(completions.load(Ordering::Relaxed), 0);
    assert_eq!(service.apply_slots.available_permits(), 4);
}
