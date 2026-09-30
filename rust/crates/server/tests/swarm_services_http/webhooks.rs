use super::*;
use citadel_licensing::LicenseCapability;
use citadel_swarm_services::{
    ServiceAutomationEntitlements, ServiceImageDigestPort, UpdateBehavior, UpdateSwarmService,
};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Default)]
struct Entitlements {
    automated: AtomicBool,
    guardrails: AtomicBool,
}
impl ServiceAutomationEntitlements for Entitlements {
    fn enabled(
        &self,
        capability: LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, SwarmServiceError>> {
        Box::pin(async move {
            Ok(match capability {
                LicenseCapability::AutomatedOperations => self.automated.load(Ordering::Relaxed),
                LicenseCapability::OperationalGuardrails => self.guardrails.load(Ordering::Relaxed),
                _ => false,
            })
        })
    }
}
#[derive(Default)]
struct Digests {
    calls: AtomicUsize,
    changed: AtomicBool,
}
impl ServiceImageDigestPort for Digests {
    fn digest<'a>(
        &'a self,
        _: Uuid,
        _: Uuid,
        _: &'a str,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, SwarmServiceError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(if self.changed.load(Ordering::Relaxed) {
                format!("sha256:{}", "a".repeat(64))
            } else {
                "sha256:fixture".into()
            })
        })
    }
}

// Ports ReceiveWebhookTests' Generic Swarm Service, Notify and license cases,
// exercising the actual HTTP handler and persisted operation rather than a
// mocked mediator result.
pub(super) async fn verify(
    pool: &sqlx::PgPool,
    identity: Arc<IdentityService>,
    admin: &ActorPrincipal,
    id: Uuid,
) {
    let entitlements = Arc::new(Entitlements::default());
    let digests = Arc::new(Digests::default());
    let services = Arc::new(
        SwarmServiceService::new(
            Arc::new(
                citadel_server::tasks::swarm_services::TrackedSwarmServiceTasks::new(
                    citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
                ),
            ),
            Arc::new(PostgresSwarmServiceRepository::new(pool.clone())),
            Arc::new(CompletingRuntime),
            CancellationToken::new(),
        )
        .with_image_digests(digests.clone())
        .with_entitlements(entitlements.clone()),
    );
    let app = router(
        pool.clone(),
        identity.clone(),
        services.clone(),
        Arc::new(
            citadel_adapters::persistence::postgres::activities::store::PostgresActivityStore::new(
                pool.clone(),
            ),
        ),
    );
    let mut current = services.get(admin.actor_id, true, id).await.unwrap();
    current.resource.spec.update_behavior = UpdateBehavior::Notify;
    current.resource.spec.webhook = Some(citadel_primitives::WebhookConfig {
        enabled: true,
        provider: citadel_primitives::WebhookProvider::Generic,
        auth_scheme: citadel_primitives::WebhookAuthScheme::BearerToken,
        secret: Some("disposable-service-webhook".into()),
        branch_filter: None,
    });
    services
        .update(
            admin.actor_id,
            true,
            id,
            UpdateSwarmService {
                spec: current.resource.spec,
                row_version: current.resource.row_version,
            },
        )
        .await
        .unwrap();
    let operation = services
        .get(admin.actor_id, true, id)
        .await
        .unwrap()
        .resource
        .current_operation
        .unwrap()
        .id;
    assert_eq!(send(&app, id, "wrong").await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(digests.calls.load(Ordering::Relaxed), 0);
    let (status, body) = send(&app, id, "disposable-service-webhook").await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["status"], "noop");
    assert_eq!(
        digests.calls.load(Ordering::Relaxed),
        0,
        "license is checked before registry I/O"
    );
    entitlements.automated.store(true, Ordering::Relaxed);
    let (_, body) = send(&app, id, "disposable-service-webhook").await;
    assert_eq!(body["status"], "noop", "equal image is not redeployed");
    digests.changed.store(true, Ordering::Relaxed);
    let (_, body) = send(&app, id, "disposable-service-webhook").await;
    assert_eq!(body["status"], "queued");
    let current = services.get(admin.actor_id, true, id).await.unwrap();
    assert_eq!(
        current.auto_update_state.status,
        citadel_primitives::AutoUpdateStatus::UpdateAvailable
    );
    assert_eq!(
        current.resource.current_operation.as_ref().unwrap().id,
        operation,
        "Notify never applies"
    );
    let mut spec = current.resource.spec;
    spec.update_behavior = UpdateBehavior::AutoDeploy;
    services
        .update(
            admin.actor_id,
            true,
            id,
            UpdateSwarmService {
                spec,
                row_version: current.resource.row_version,
            },
        )
        .await
        .unwrap();
    let (_, body) = send(&app, id, "disposable-service-webhook").await;
    assert_eq!(body["status"], "noop");
    assert!(body["reason"].as_str().unwrap().contains("license"));
    assert_eq!(
        services
            .get(admin.actor_id, true, id)
            .await
            .unwrap()
            .resource
            .current_operation
            .unwrap()
            .id,
        operation
    );
    entitlements.guardrails.store(true, Ordering::Relaxed);
    let (_, body) = send(&app, id, "disposable-service-webhook").await;
    assert_eq!(body["status"], "queued");
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let current = services.get(admin.actor_id, true, id).await.unwrap();
            if current.control_state == citadel_primitives::ResourceControlState::Idle {
                assert_ne!(
                    current.resource.current_operation.as_ref().unwrap().id,
                    operation
                );
                assert_eq!(
                    current.resource.current_operation.unwrap().state,
                    citadel_swarm_services::SwarmServiceOperationState::Completed
                );
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    // An authenticated snapshot cannot authorize a newer configuration.
    let old = services.get(admin.actor_id, true, id).await.unwrap();
    let mut spec = old.spec.clone();
    spec.webhook.as_mut().unwrap().secret = Some("rotated-service-webhook".into());
    services
        .update(
            admin.actor_id,
            true,
            id,
            UpdateSwarmService {
                spec,
                row_version: old.row_version,
            },
        )
        .await
        .unwrap();
    let scans = digests.calls.load(Ordering::Relaxed);
    use citadel_swarm_services::SwarmServiceRepository;
    assert!(matches!(
        PostgresSwarmServiceRepository::new(pool.clone())
            .claim_operation(
                admin.actor_id,
                true,
                citadel_swarm_services::ServiceOperationRequest {
                    id,
                    kind: citadel_swarm_services::ServiceOperationKind::Apply,
                    replicas: None,
                    expected_version: Some(old.row_version),
                }
            )
            .await,
        Err(SwarmServiceError::Conflict(_))
    ));
    assert!(matches!(
        services
            .check_automated_updates(old, &CancellationToken::new())
            .await,
        Err(SwarmServiceError::Conflict(_))
    ));
    assert_eq!(digests.calls.load(Ordering::Relaxed), scans);
    assert_eq!(
        send(&app, id, "disposable-service-webhook").await.0,
        StatusCode::UNAUTHORIZED
    );
    let events: Vec<Value> = sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='SwarmServiceWebhookReceived'").bind(id).fetch_all(pool).await.unwrap();
    assert_eq!(events.len(), 7);
    assert!(
        events
            .iter()
            .all(|event| event["$type"] == "SwarmServiceWebhookReceived"
                && event["RequestId"].as_str().is_some())
    );
    assert!(events.iter().any(|event| event["Status"] == "rejected"));
    assert!(events.iter().any(|event| event["Status"] == "queued"));
    assert!(!format!("{events:?}").contains("disposable-service-webhook"));

    let current = services.get(admin.actor_id, true, id).await.unwrap();
    let mut spec = current.resource.spec;
    spec.update_behavior = UpdateBehavior::Notify;
    services
        .update(
            admin.actor_id,
            true,
            id,
            UpdateSwarmService {
                spec,
                row_version: current.resource.row_version,
            },
        )
        .await
        .unwrap();
    let store = PostgresSwarmServiceRepository::new(pool.clone());
    assert_eq!(
        store.update_check_candidates(None, 1).await.unwrap(),
        vec![id]
    );
    assert!(
        store
            .update_check_candidates(Some(id), 1)
            .await
            .unwrap()
            .is_empty()
    );
    let calls = digests.calls.load(Ordering::Relaxed);
    assert_eq!(
        services
            .run_scheduled_update_checks(&CancellationToken::new())
            .await
            .unwrap(),
        1
    );
    assert_eq!(digests.calls.load(Ordering::Relaxed), calls + 1);
    let failing_audit_app = router(
        pool.clone(),
        identity,
        services.clone(),
        Arc::new(FailingAudit),
    );
    let (status, body) = send(&failing_audit_app, id, "rotated-service-webhook").await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "audit failure must not request a retry of committed dispatch"
    );
    assert_eq!(body["status"], "queued");
    let current = services.get(admin.actor_id, true, id).await.unwrap();
    let mut spec = current.resource.spec;
    spec.update_behavior = UpdateBehavior::Disabled;
    services
        .update(
            admin.actor_id,
            true,
            id,
            UpdateSwarmService {
                spec,
                row_version: current.resource.row_version,
            },
        )
        .await
        .unwrap();
    let calls = digests.calls.load(Ordering::Relaxed);
    assert_eq!(
        services
            .run_scheduled_update_checks(&CancellationToken::new())
            .await
            .unwrap(),
        0
    );
    assert_eq!(digests.calls.load(Ordering::Relaxed), calls);
}

struct FailingAudit;
impl citadel_activities::WebhookActivitySink for FailingAudit {
    fn record_webhook(
        &self,
        _: citadel_activities::ActivityResourceType,
        _: Uuid,
        _: citadel_activities::WebhookActivityDetails,
    ) -> BoxFuture<'_, Result<(), citadel_activities::ActivityError>> {
        Box::pin(async {
            Err(citadel_activities::ActivityError::Storage(
                "audit unavailable".into(),
            ))
        })
    }
}

async fn send(app: &Router, id: Uuid, secret: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/listener/generic/swarm-service/{id}/update"))
                .header("authorization", format!("Bearer {secret}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (status, response_json(response).await)
}

fn router(
    pool: sqlx::PgPool,
    identity: Arc<IdentityService>,
    services: Arc<SwarmServiceService>,
    audit: Arc<dyn citadel_activities::WebhookActivitySink>,
) -> Router {
    use citadel_adapters::{
        persistence::postgres::{
            automation::PostgresAutomationRepository,
            git::{
                accounts::PostgresGitAccountRepository,
                repositories::PostgresGitRepositoryExecutionPersistence,
            },
        },
        security::identity::{
            automation_token::IdentityAutomationRunTokenIssuer, crypto::AesGcmSecretProtector,
        },
    };
    let git = Arc::new(citadel_git::GitRepositoryExecutionService::new(
        Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone())),
        Arc::new(citadel_git::GitAccountService::new(
            Arc::new(PostgresGitAccountRepository::new(pool.clone())),
            Arc::new(AesGcmSecretProtector::new(&[59; 32]).unwrap()),
        )),
        Arc::new(citadel_git::GitCli::new(
            std::sync::Arc::new(citadel_processes::SystemProcess),
            std::time::Duration::from_secs(5),
        )),
        std::env::temp_dir().join(format!("unused-service-webhook-git-{}", Uuid::now_v7())),
        std::time::Duration::from_secs(60),
    ));
    let automation_shutdown = tokio_util::sync::CancellationToken::new();
    let automation_tasks = citadel_runtime::DynamicTasks::new(automation_shutdown.clone());

    let automation = Arc::new(citadel_automation::AutomationService::new(
        std::sync::Arc::new(citadel_processes::SystemProcess),
        Arc::new(
            citadel_server::tasks::automation::TrackedAutomationTasks::new(
                automation_tasks.clone(),
            ),
        ),
        automation_shutdown.clone(),
        Arc::new(PostgresAutomationRepository::new(pool.clone())),
        Arc::new(IdentityAutomationRunTokenIssuer::new(identity)),
        citadel_automation::AutomationRuntimeConfig {
            deno_path: "deno".into(),
            work_root: std::env::temp_dir(),
            internal_base_url: "http://127.0.0.1:8000".into(),
            endpoint_catalog_json: "[]".into(),
            maximum_log_bytes: 1024,
            stale_after: std::time::Duration::from_secs(60),
        },
    ));
    citadel_server::api::routes::webhooks::router(
        citadel_server::api::routes::webhooks::WebhooksHttpState {
            git,
            automation,
            backups: None,
            builds: None,
            stacks: None,
            services: Some(services),
            alerts: None,
            audit: Some(audit),
        },
    )
}
