use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use chrono::Duration;
use citadel_adapters::{
    persistence::postgres::{
        alerts::PostgresAlertRepository,
        backups::PostgresBackupPersistence,
        builds::PostgresBuildRepository,
        identity::authentication::store::{PostgresIdentityStore, StaticEntitlementService},
    },
    security::identity::crypto::{
        Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
    },
};
use citadel_alerts::{AlertChannel, AlertDelivery, AlertError, AlertEvent};
use citadel_backups::{
    BackupClaim, BackupError, BackupExecutionResult, BackupExecutor, BackupLog, BackupRepository,
    BackupRunAuthorizer, BackupService, BackupSourcePlan, BackupSourcePlanner, RestoreClaim,
    RestoreExecutionResult,
};
use citadel_builds::{
    BuildClaim, BuildExecutionResult, BuildExecutor, BuildLog, BuildRepository, BuildService,
};
use citadel_database::MigrationRunner;
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, AuthenticatedPrincipalType, IdentityService,
    NoopServiceAccountLastUsedTracker, SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_primitives::{ActorId, ResourceType};
use citadel_server::{
    api::routes::{
        alerts as alerts_http, alerts::AlertsHttpState, backups as backups_http,
        backups::BackupsHttpState, bindings, builds as builds_http, builds::BuildsHttpState,
        git_repositories as git_catalog, registries, tags,
    },
    metrics::Metrics,
    realtime::{RealtimeHub, change_callback},
};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "execution_resources_http/alert_assertions.rs"]
mod alert_assertions;
#[path = "execution_resources_http/alert_rule_create.rs"]
mod alert_rule_create;
#[path = "execution_resources_http/alert_rule_list.rs"]
mod alert_rule_list;
#[path = "execution_resources_http/alert_rule_metadata.rs"]
mod alert_rule_metadata;
#[path = "execution_resources_http/alert_rule_patch.rs"]
mod alert_rule_patch;
#[path = "execution_resources_http/backup_completion.rs"]
mod backup_completion;
#[path = "execution_resources_http/backup_policy_metadata.rs"]
mod backup_policy_metadata;
#[path = "execution_resources_http/backup_repository_patch.rs"]
mod backup_repository_patch;
#[path = "execution_resources_http/backup_summaries.rs"]
mod backup_summaries;
#[path = "execution_resources_http/backup_webhooks.rs"]
mod backup_webhooks;
#[path = "execution_resources_http/build_completion.rs"]
mod build_completion;
#[path = "execution_resources_http/build_contracts.rs"]
mod build_contracts;
#[path = "execution_resources_http/build_pools.rs"]
mod build_pools;
#[path = "execution_resources_http/build_webhooks.rs"]
mod build_webhooks;
#[path = "execution_resources_http/git_contracts.rs"]
mod git_contracts;
#[path = "execution_resources_http/git_webhooks.rs"]
mod git_webhooks;
#[path = "execution_resources_http/stack_webhooks.rs"]
mod stack_webhooks;
#[path = "execution_resources_http/validation.rs"]
mod validation;

#[path = "execution_resources_http/backup_contract.rs"]
mod backup_contract;

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn resource_endpoints_authorize_validate_and_persist_lifecycles() {
    let database_url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(&[83_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let build_store = Arc::new(PostgresBuildRepository::new(pool.clone()));
    let build_entitlement = Arc::new(build_webhooks::Entitlement::default());
    let backup_store = Arc::new(PostgresBackupPersistence::new(pool.clone()));
    let backup_entitlement = Arc::new(backup_webhooks::Entitlement::default());
    let backup_planner = Arc::new(FakeBackupPlanner::default());
    let alert_entitlement = Arc::new(alert_rule_create::Entitlement::default());
    let alert_store = Arc::new(
        PostgresAlertRepository::new(pool.clone()).with_entitlements(alert_entitlement.clone()),
    );
    let hub = RealtimeHub::new(128, Arc::new(Metrics::default()));
    let _subscriber = hub.subscribe();
    let edge = citadel_adapters::connectors::edge::EdgeRegistry::default();
    let cancellation = CancellationToken::new();
    let build_tasks = citadel_runtime::DynamicTasks::new(cancellation.clone());
    let builds = Arc::new(
        BuildService::new(
            Arc::new(citadel_server::tasks::builds::TrackedBuildTasks::new(
                build_tasks.clone(),
            )),
            cancellation.clone(),
            build_store.clone(),
            Arc::new(FakeBuildExecutor { pool: pool.clone() }),
            Duration::minutes(5),
        )
        .with_entitlements(build_entitlement.clone())
        .with_change_notifier(change_callback(Some(hub.clone()), "Build"))
        .with_pool_checker(Arc::new(
            citadel_adapters::connectors::agent::build_pool_checker::AgentBuildPoolChecker {
                agent: None,
                edge: edge.clone(),
            },
        ))
        .with_pool_change_notifier(change_callback(Some(hub.clone()), "BuildAgentPool"))
        .with_log_notifier(citadel_server::realtime::build_log_callback(Some(
            hub.clone(),
        ))),
    );
    let backups = Arc::new(
        BackupService::new(
            backup_store,
            Arc::new(FakeBackupExecutor),
            backup_planner.clone(),
            Duration::minutes(5),
            Arc::new(AllowBackupExecution),
        )
        .with_change_notifier(change_callback(Some(hub.clone()), "BackupPolicy"))
        .with_entitlements(backup_entitlement.clone()),
    );
    let (stacks, stack_entitlement) = stack_webhooks::service(pool.clone());
    let webhook_router = backup_webhooks::router(
        pool.clone(),
        identity.clone(),
        backups.clone(),
        builds.clone(),
        stacks.clone(),
    );
    let app = builds_http::router(BuildsHttpState {
        identity: Arc::clone(&identity),
        builds: builds.clone(),
    })
    .merge(webhook_router)
    .merge(backups_http::router(BackupsHttpState {
        identity: Arc::clone(&identity),
        backups: backups.clone(),
        cancellation: cancellation.clone(),
    }))
    .merge(alerts_http::router(AlertsHttpState {
        identity: identity.clone(),
        store: alert_store.clone(),
        delivery: Arc::new(FakeAlertDelivery),
    }))
    .merge(tags::router(tags::TagsHttpState {
        identity: identity.clone(),
        tags: Arc::new(
            citadel_adapters::persistence::postgres::tags::PostgresTagRepository::new(
                pool.clone(),
            ),
        ),
        realtime: None,
    })
    .merge(registries::router(registries::RegistriesHttpState {
        registry_connections: Arc::new(citadel_adapters::connectors::registries::browser::RegistryBrowser::with_endpoints("http://127.0.0.1:1", "http://127.0.0.1:1").unwrap()),
        identity: identity.clone(),
        registries: Arc::new(
            citadel_adapters::persistence::postgres::registries::PostgresRegistryRepository::new(
                pool.clone(),
            ),
        ),
        realtime: None,
    }))
    .merge(bindings::router(bindings::BindingsHttpState {
        identity: identity.clone(),
        secrets: Arc::new(citadel_bindings::SecretService::new(
                Arc::new(
                    citadel_adapters::persistence::postgres::bindings::PostgresBindingRepository::new(
                        pool.clone(),
                    ),
                ),
                Arc::new(citadel_adapters::security::identity::crypto::AesGcmSecretProtector::new(&[59; 32]).unwrap()),
            )),
        realtime: None,
    }))
    .merge(git_catalog::catalog_router(git_catalog::GitCatalogHttpState {
        identity: identity.clone(),
        git_repositories: Arc::new(
            citadel_adapters::persistence::postgres::git::repositories::PostgresGitRepositoryPersistence::new(
                pool.clone(),
            ),
        ),
        realtime: None,
    })))
    .layer(axum::Extension(hub.clone()))
    .layer(axum::Extension(
        citadel_server::api::resources::platforms::edge::EdgeHttpContext {
        node_agent_policy: Default::default(),
            node_agent_ca_bundle: None,
            store: std::sync::Arc::new(citadel_adapters::persistence::postgres::platforms::edge::store::PostgresEdgeStore::new(pool.clone())),
            registry: std::sync::Arc::new(edge.clone()),
            core_url: "https://core.example.test:8001".into(),
            agent_image: "citadel-agent:test".into(),
        },
    ));

    assert_eq!(
        request(&app, Method::GET, "/api/v1/buildProjects", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let principal = seed_administrator(&pool).await;
    assert_eq!(
        hub.current_revision(),
        0,
        "reads/authentication failures do not notify"
    );
    let fixture = seed_dependencies(&pool, principal.actor_id).await;
    validation::verify_inputs(&app, &principal, fixture.git_repository).await;
    git_contracts::verify(&app, &principal).await;
    let suffix = Uuid::now_v7().simple().to_string();

    let build_pool_response = request(
        &app,
        Method::POST,
        "/api/v1/buildAgentPools",
        Some(principal.clone()),
        Some(json!({
            "name":format!("pool-{suffix}"), "enabled":true,
            "providerSpec":{"$type":"SelfManagedVm","connectionMode":"EdgeAgent"}
        })),
    )
    .await;
    assert_eq!(build_pool_response.status(), StatusCode::OK);
    let build_pool = response_json(build_pool_response).await;
    let pool_id = build_pool["id"].as_str().unwrap();
    // Port the BuildAgentPool endpoint lifecycle's tag persistence, and cover
    // the existing table's positive/negative tag-name filters.
    let pool_tag_response = request(
        &app,
        Method::POST,
        "/api/v1/tags",
        Some(principal.clone()),
        Some(json!({"name":format!("pool-tag-{suffix}"),"color":"#3366FF"})),
    )
    .await;
    assert_eq!(pool_tag_response.status(), StatusCode::OK);
    let pool_tag = response_json(pool_tag_response).await;
    let pool_tags_path = format!("/api/v1/buildAgentPools/{pool_id}/tags");
    assert_eq!(
        request(
            &app,
            Method::PUT,
            &pool_tags_path,
            Some(principal.clone()),
            Some(json!({"tagIds":[pool_tag["id"]]}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    let persisted_tags = response_json(
        request(
            &app,
            Method::GET,
            &pool_tags_path,
            Some(principal.clone()),
            None,
        )
        .await,
    )
    .await;
    assert!(
        persisted_tags
            .to_string()
            .contains(pool_tag["id"].as_str().unwrap())
    );
    for (filter, expected) in [
        (pool_tag["name"].as_str().unwrap().to_owned(), true),
        (format!("absent-{suffix}"), false),
    ] {
        let response = request(
            &app,
            Method::GET,
            &format!("/api/v1/buildAgentPools?tags={filter}"),
            Some(principal.clone()),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let filtered = response_json(response).await;
        assert_eq!(
            filtered["pools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|pool| pool["id"] == pool_id),
            expected
        );
    }
    build_pools::verify(
        &app,
        &pool,
        &edge,
        &builds,
        &principal,
        Uuid::parse_str(pool_id).unwrap(),
    )
    .await;
    let edge_path = format!("/api/v1/buildAgentPools/{pool_id}/edge");
    assert_eq!(
        request(
            &app,
            Method::POST,
            &format!("{edge_path}/enrollments"),
            None,
            None
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let enrollment = request(
        &app,
        Method::POST,
        &format!("{edge_path}/enrollments"),
        Some(principal.clone()),
        None,
    )
    .await;
    assert_eq!(enrollment.status(), StatusCode::OK);
    assert!(
        enrollment.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("no-store")
    );
    let enrollment = response_json(enrollment).await;
    assert_eq!(enrollment["platformId"], Uuid::nil().to_string());
    assert_eq!(
        enrollment["instructions"]["environment"]["CITADEL_EDGE_AGENT_PROFILE"],
        "edge-build-agent"
    );
    assert!(
        enrollment["instructions"]["dockerRunCommand"]
            .as_str()
            .unwrap()
            .contains("-e 'CITADEL_EDGE_AGENT_PROFILE=edge-build-agent'")
    );
    let command = enrollment["instructions"]["dockerRunCommand"]
        .as_str()
        .unwrap();
    assert!(command.contains("edge-build-agent"));
    assert!(
        !command.contains("/:/host"),
        "Build pool does not need host filesystem access"
    );
    let stored_hash: String =
        sqlx::query_scalar("SELECT tokenhash FROM edgeagentenrollments WHERE id=$1")
            .bind(Uuid::parse_str(enrollment["enrollmentId"].as_str().unwrap()).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(stored_hash, enrollment["token"].as_str().unwrap());
    assert_eq!(
        request(
            &app,
            Method::GET,
            &format!("{edge_path}/status"),
            Some(principal.clone()),
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            Method::POST,
            &format!("{edge_path}/revoke"),
            Some(principal.clone()),
            None
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let revoked: bool =
        sqlx::query_scalar("SELECT revokedatutc IS NOT NULL FROM edgeagentenrollments WHERE id=$1")
            .bind(Uuid::parse_str(enrollment["enrollmentId"].as_str().unwrap()).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(revoked);

    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/buildAgentPools/{pool_id}"),
            Some(principal.clone()),
            None
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/buildAgentPools/{pool_id}"),
            Some(principal.clone()),
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    let archived: bool = sqlx::query_scalar(
        "SELECT archivedat IS NOT NULL AND NOT enabled FROM buildagentpools WHERE id=$1",
    )
    .bind(Uuid::parse_str(pool_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(archived);
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='BuildAgentPoolDeleted'")
        .bind(Uuid::parse_str(pool_id).unwrap()).fetch_one(&pool).await.unwrap();
    assert_eq!(events, 1);

    // A cancelled Pool Test completes in the background. Check the Alert
    // subscription, not the global revision shared with that independent job.
    let mut alert_changes = hub.subscribe();
    let channel = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/alertRules/channels",
            Some(principal.clone()),
            Some(json!({
                "name":format!("channel-{suffix}"),
                "alertDestination":"Generic",
                "url":"https://alerts.example.test/hook",
                "isActive":true
            })),
        )
        .await,
    )
    .await;
    let channel_id = channel["id"].as_str().unwrap();
    assert_eq!(
        drain_resource_changes(&mut alert_changes, "Alert"),
        1,
        "successful persisted mutations notify"
    );
    let invalid_rule = request(
        &app,
        Method::POST,
        "/api/v1/alertRules",
        Some(principal.clone()),
        Some(alert_rule_json(&suffix, Uuid::now_v7())),
    )
    .await;
    assert_eq!(invalid_rule.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        drain_resource_changes(&mut alert_changes, "Alert"),
        0,
        "rejected writes must not notify"
    );
    let rule = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/alertRules",
            Some(principal.clone()),
            Some(alert_rule_json(
                &suffix,
                Uuid::parse_str(channel_id).unwrap(),
            )),
        )
        .await,
    )
    .await;
    assert_eq!(rule["channelIds"][0], channel_id);
    alert_rule_list::verify(&app, &pool, &principal, &rule, &channel).await;
    alert_rule_metadata::verify(&app, &pool, &principal, &rule, &alert_store).await;
    alert_rule_patch::verify(&app, &pool, &principal, &rule, &channel, &hub, &alert_store).await;
    alert_rule_create::verify(
        &app,
        &pool,
        &principal,
        &alert_entitlement,
        &channel,
        &alert_store,
    )
    .await;

    let project = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/buildProjects",
            Some(principal.clone()),
            Some(json!({
                "name":format!("build-{suffix}"),"description":null,"enabled":true,
                "gitRepositoryId":fixture.git_repository,"branch":"main","contextPath":".",
                "dockerfilePath":"Dockerfile","target":null,"buildArgs":[],"buildSecrets":[],
                "builderKind":"Platform","platformId":fixture.platform,"buildAgentPoolId":null,
                "registryId":fixture.registry,"imageRepository":"citadel/test",
                "tagTemplates":["{branch}-{shortSha}"],"webhook":null,"timeoutSeconds":60,
                "retentionRunCount":2,"tagIds":[]
            })),
        )
        .await,
    )
    .await;
    let project_id = project["id"].as_str().unwrap();
    assert_eq!(project["capabilities"]["canWrite"], true);
    let project_path = format!("/api/v1/buildProjects/{project_id}");
    let tag = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/tags",
            Some(principal.clone()),
            Some(json!({"name":format!("build-tag-{suffix}"),"color":"#3366FF"})),
        )
        .await,
    )
    .await;
    assert_eq!(
        request(
            &app,
            Method::PUT,
            &format!("{project_path}/tags"),
            Some(principal.clone()),
            Some(json!({"tagIds":[tag["id"]]}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    let filtered = response_json(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/v1/buildProjects?tags={}",
                tag["name"].as_str().unwrap()
            ),
            Some(principal.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(filtered["projects"].as_array().unwrap().len(), 1);
    let response = request(
        &app,
        Method::PATCH,
        &project_path,
        Some(principal.clone()),
        Some(json!({"description":"edited Build","dockerfilePath":"src/Dockerfile"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await["dockerfilePath"],
        "src/Dockerfile"
    );
    let renamed = request(
        &app,
        Method::POST,
        "/api/v1/buildProjects/rename",
        Some(principal.clone()),
        Some(json!({"id":project_id,"name":format!("renamed-{suffix}")})),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("{project_path}/_metadata"),
            Some(principal.clone()),
            Some(json!({"description":"metadata Build"}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("{project_path}/_metadata"),
            Some(principal.clone()),
            Some(json!({"enabled":false}))
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let stored = build_store
        .get(Uuid::parse_str(project_id).unwrap())
        .await
        .unwrap();
    assert_eq!(stored.description.as_deref(), Some("metadata Build"));
    assert_eq!(stored.tags[0].id.to_string(), tag["id"]);
    let mut stale_edit = stored
        .apply_patch(json!({"description":"stale"}), false)
        .unwrap();
    stale_edit.validate().unwrap();
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &project_path,
            Some(principal.clone()),
            Some(json!({"description":"newest"}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert!(matches!(
        build_store
            .update(&stored, &stale_edit, principal.actor_id, false)
            .await,
        Err(citadel_builds::BuildError::Conflict(_))
    ));
    let events: Vec<String> = sqlx::query_scalar(
        "SELECT eventtype FROM activityevents WHERE resourceid=$1 ORDER BY createdat,id",
    )
    .bind(stored.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        events,
        [
            "BuildCreated",
            "BuildUpdated",
            "BuildRenamed",
            "BuildUpdated"
        ]
    );
    build_entitlement.set_enabled(true);
    build_contracts::verify(&app, &principal, project_id).await;
    build_entitlement.set_enabled(false);
    let run = response_json(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/buildProjects/{project_id}/runs"),
            Some(principal.clone()),
            Some(json!({"trigger":"Manual"})),
        )
        .await,
    )
    .await;
    let build_run_id = run["id"].as_str().unwrap();
    assert_eq!(run["status"], "Queued");
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &project_path,
            Some(principal.clone()),
            Some(json!({"enabled":false}))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/buildRuns/{build_run_id}/cancel"),
            Some(principal.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );

    let repository = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/backupRepositories",
            Some(principal.clone()),
            Some(json!({
                "name":format!("repository-{suffix}"),"description":null,
                "spec":{"$type":"FileSystem","location":"Core","platformId":null,"path":"/tmp/citadel-backups"},
                "passwordSecretId":fixture.secret
            })),
        )
        .await,
    )
    .await;
    let repository_id = repository["id"].as_str().unwrap();
    backup_repository_patch::before_ready(&app, &pool, &principal, &repository).await;
    let validation = response_json(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/backupRepositories/{repository_id}/validate"),
            Some(principal.clone()),
            Some(json!({"location":"Core","platformId":null})),
        )
        .await,
    )
    .await;
    assert_eq!(validation["status"], "Ready");
    backup_repository_patch::after_ready(&app, &pool, &principal, &repository).await;
    let persisted_validation: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM backuprepositoryvalidations WHERE backuprepositoryid=$1",
    )
    .bind(Uuid::parse_str(repository_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(persisted_validation, 1);
    let policy = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/backupPolicies",
            Some(principal.clone()),
            Some(json!({
                "name":format!("policy-{suffix}"),"description":null,
                "source":{"$type":"DockerVolume","platformId":fixture.platform,"volumeName":"data"},
                "backupRepositoryId":repository_id,"enabled":true,"cron":null,"timeZone":"UTC",
                "webhook":null,"keepLastSuccessful":2,"timeoutSeconds":60,"alertOnFailure":true,
                "runAsActorId":principal.actor_id.value(),"tagIds":[]
            })),
        )
        .await,
    )
    .await;
    let policy_id = policy["id"].as_str().unwrap();
    validation::verify_policy(&app, &principal, &policy).await;
    backup_policy_metadata::verify(&app, &pool, &principal, &policy).await;
    backup_contract::verify(&pool, &principal, &policy).await;
    backup_summaries::verify(&app, &pool, &principal, &policy).await;
    backup_completion::verify_policy(&app, &pool, &principal, &policy).await;
    backup_completion::verify_previews(&pool, identity.clone(), &principal, &fixture).await;
    backup_completion::verify_streams(&app, &pool, &backups, &principal, &policy).await;
    let backup_run = response_json(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/backupPolicies/{policy_id}/runs"),
            Some(principal.clone()),
            Some(json!({"trigger":"Manual"})),
        )
        .await,
    )
    .await;
    assert_eq!(backup_run["status"], "Queued");
    let backup_run_id = backup_run["id"].as_str().unwrap();
    assert_eq!(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/backupRuns/{backup_run_id}/cancel"),
            Some(principal.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );

    let reader = seed_regular_user(&pool).await;
    let restore_id = Uuid::now_v7();
    sqlx::query("INSERT INTO backuprestoreruns(id,backuprunid,backuprepositoryid,targetplatformid,targetvolumename,overwriteexisting,targetvolumecreatedbycitadel,status,triggeredbyactorid) VALUES($1,$2,$3,$4,'restored',false,false,'Queued',$5)")
        .bind(restore_id).bind(Uuid::parse_str(backup_run_id).unwrap()).bind(Uuid::parse_str(repository_id).unwrap())
        .bind(fixture.platform).bind(principal.actor_id.value()).execute(&pool).await.unwrap();
    for (resource, id) in [
        ("backupRuns", Uuid::parse_str(backup_run_id).unwrap()),
        ("backupRestoreRuns", restore_id),
    ] {
        let path = format!("/api/v1/{resource}/{id}/events");
        assert_eq!(
            request(&app, Method::GET, &path, None, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(&app, Method::GET, &path, Some(reader.clone()), None)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        let response = request(&app, Method::GET, &path, Some(principal.clone()), None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response_json(response).await,
            json!({"runId":id,"events":[]})
        );
        assert_eq!(
            request(
                &app,
                Method::GET,
                &format!("/api/v1/{resource}/{}/events", Uuid::now_v7()),
                Some(principal.clone()),
                None
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
    }
    git_webhooks::verify(&app, &pool, &fixture).await;
    build_webhooks::verify(
        &app,
        &pool,
        &builds,
        &build_entitlement,
        &principal,
        &fixture,
    )
    .await;
    backup_webhooks::verify(
        &app,
        &pool,
        &backups,
        &backup_entitlement,
        Uuid::parse_str(policy_id).unwrap(),
    )
    .await;
    stack_webhooks::verify(&app, &pool, &stacks, &stack_entitlement).await;
    for (method, suffix) in [
        (Method::GET, "status"),
        (Method::POST, "enrollments"),
        (Method::POST, "revoke"),
    ] {
        assert_eq!(
            request(
                &app,
                method,
                &format!("{edge_path}/{suffix}"),
                Some(reader.clone()),
                None
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        response_json(
            request(
                &app,
                Method::GET,
                "/api/v1/buildProjects",
                Some(reader.clone()),
                None,
            )
            .await,
        )
        .await["projects"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    use citadel_identity::{ResourceAccessInput, UserRepository};
    let users = citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository::new(pool.clone());
    for (resource_type, resource_id) in [
        (ResourceType::Build, Uuid::parse_str(project_id).unwrap()),
        (
            ResourceType::BackupRepository,
            Uuid::parse_str(repository_id).unwrap(),
        ),
        (
            ResourceType::BackupPolicy,
            Uuid::parse_str(policy_id).unwrap(),
        ),
        (
            ResourceType::AlertChannel,
            Uuid::parse_str(channel_id).unwrap(),
        ),
        (
            ResourceType::Alert,
            Uuid::parse_str(rule["id"].as_str().unwrap()).unwrap(),
        ),
    ] {
        users
            .add_resource_access(
                reader.subject_id,
                &ResourceAccessInput {
                    resource_type,
                    resource_id,
                    permission_level: citadel_primitives::PermissionLevel::Read,
                    specific_permissions: Vec::new(),
                },
                principal.actor_id,
                chrono::Utc::now(),
                true,
            )
            .await
            .unwrap();
    }
    for path in [
        format!("/api/v1/backupRepositories/{repository_id}"),
        format!("/api/v1/backupPolicies/{policy_id}"),
        format!("/api/v1/alertRules/channels/{channel_id}"),
        format!("/api/v1/alertRules/{}", rule["id"].as_str().unwrap()),
    ] {
        let response = request(&app, Method::GET, &path, Some(reader.clone()), None).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let value = response_json(response).await;
        assert_eq!(value["capabilities"]["canRead"], true, "{path}");
        assert_eq!(value["capabilities"]["canWrite"], false, "{path}");
    }
    let visible_builds = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/buildProjects",
            Some(reader.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(visible_builds["projects"].as_array().unwrap().len(), 1);
    let queue_path = format!("/api/v1/buildProjects/{project_id}/runs");
    assert_eq!(
        request(&app, Method::POST, &queue_path, Some(reader.clone()), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut access = ResourceAccessInput {
        resource_type: ResourceType::Build,
        resource_id: Uuid::parse_str(project_id).unwrap(),
        permission_level: citadel_primitives::PermissionLevel::Read,
        specific_permissions: Vec::new(),
    };
    users
        .remove_resource_access(
            reader.subject_id,
            &access,
            principal.actor_id,
            chrono::Utc::now(),
        )
        .await
        .unwrap();
    access
        .specific_permissions
        .push(citadel_primitives::SpecificPermission::Apply);
    users
        .add_resource_access(
            reader.subject_id,
            &access,
            principal.actor_id,
            chrono::Utc::now(),
            true,
        )
        .await
        .unwrap();
    let queued = request(&app, Method::POST, &queue_path, Some(reader.clone()), None).await;
    assert_eq!(queued.status(), StatusCode::OK);
    let queued = response_json(queued).await;
    assert_eq!(
        request(
            &app,
            Method::POST,
            &format!(
                "/api/v1/buildRuns/{}/cancel",
                queued["id"].as_str().unwrap()
            ),
            Some(reader.clone()),
            None
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let denied_trigger = request(
        &app,
        Method::POST,
        &queue_path,
        Some(reader.clone()),
        Some(json!({"trigger":"Webhook"})),
    )
    .await;
    assert_eq!(denied_trigger.status(), StatusCode::FORBIDDEN);
    assert_eq!(request(&app, Method::PATCH, &format!("/api/v1/buildProjects/{project_id}"), Some(seed_administrator(&pool).await),
        Some(json!({"webhook":{"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"fixture-shared-secret"}}))).await.status(), StatusCode::FORBIDDEN);
    let visible_repositories = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/backupRepositories",
            Some(reader.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        visible_repositories["repositories"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let visible_channels = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/alertRules/channels",
            Some(reader),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(visible_channels["channels"].as_array().unwrap().len(), 1);

    // Execution notifications must cover both claim and completion, including a
    // persisted failure, without an HTTP request keeping the browser alive.
    let principal = seed_administrator(&pool).await;
    let run = response_json(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/buildProjects/{project_id}/runs"),
            Some(principal.clone()),
            Some(json!({"trigger":"Manual"})),
        )
        .await,
    )
    .await;
    let before = hub.current_revision();
    assert!(builds.process_one(&CancellationToken::new()).await.unwrap());
    assert_eq!(hub.current_revision(), before + 3);
    // A webhook run already queued before license loss must fail before the
    // executor runs (the fake executor would persist "live output").
    let denied = build_store
        .enqueue(
            principal.actor_id,
            Uuid::parse_str(project_id).unwrap(),
            "Webhook",
        )
        .await
        .unwrap();
    assert!(builds.process_one(&CancellationToken::new()).await.unwrap());
    let denied_result = build_store.get_run(denied.id).await.unwrap();
    assert_eq!(denied_result.status, citadel_builds::BuildRunStatus::Failed);
    assert_eq!(
        denied_result.error_code.as_deref(),
        Some("build.entitlement")
    );
    assert!(build_store.logs(denied.id).await.unwrap().is_empty());
    let status: String = sqlx::query_scalar("SELECT status FROM buildruns WHERE id=$1")
        .bind(Uuid::parse_str(run["id"].as_str().unwrap()).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Succeeded");
    let build_run_id = Uuid::parse_str(run["id"].as_str().unwrap()).unwrap();
    let logs_url = format!("/api/v1/buildRuns/{build_run_id}/logs");
    let logs = request(&app, Method::GET, &logs_url, Some(principal.clone()), None).await;
    assert_eq!(logs.status(), StatusCode::OK);
    let logs = response_json(logs).await;
    assert_eq!(logs["logs"][0]["message"], "live output");
    assert_eq!(logs["logs"][0]["buildRunId"], run["id"]);
    assert!(Uuid::parse_str(logs["logs"][0]["id"].as_str().unwrap()).is_ok());
    assert!(
        chrono::DateTime::parse_from_rfc3339(logs["logs"][0]["createdAt"].as_str().unwrap())
            .is_ok()
    );
    let again =
        response_json(request(&app, Method::GET, &logs_url, Some(principal.clone()), None).await)
            .await;
    assert_eq!(
        logs, again,
        "Persisted IDs must be stable across reconnect/refetch"
    );
    assert!(
        build_store
            .append_log(
                build_run_id,
                &BuildLog {
                    stream: "stdout".into(),
                    message: "late output".into()
                }
            )
            .await
            .is_err(),
        "A terminal run must reject late log writes"
    );
    backup_planner
        .fail
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let run = response_json(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/backupPolicies/{policy_id}/runs"),
            Some(principal),
            Some(json!({"trigger":"Manual"})),
        )
        .await,
    )
    .await;
    let before = hub.current_revision();
    assert!(
        backups
            .process_backup(&CancellationToken::new())
            .await
            .unwrap()
    );
    assert_eq!(hub.current_revision(), before + 2);
    let status: String = sqlx::query_scalar("SELECT status FROM backupruns WHERE id=$1")
        .bind(Uuid::parse_str(run["id"].as_str().unwrap()).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Failed");
    cancellation.cancel();
    build_tasks
        .drain(std::time::Duration::from_secs(5))
        .await
        .unwrap();
}

struct FixtureIds {
    platform: Uuid,
    registry: Uuid,
    git_repository: Uuid,
    secret: Uuid,
}

async fn seed_administrator(pool: &sqlx::PgPool) -> ActorPrincipal {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,email,name) VALUES($1,$2,$3,$4,$5)")
        .bind(user_id)
        .bind(actor_id)
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("{user_id}@example.test"))
        .bind("execution admin")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(pool)
        .await
        .unwrap();
    ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name: "execution admin".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".into()],
    }
}

async fn seed_regular_user(pool: &sqlx::PgPool) -> ActorPrincipal {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,email,name) VALUES($1,$2,$3,$4,$5)")
        .bind(user_id)
        .bind(actor_id)
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("{user_id}@example.test"))
        .bind("execution reader")
        .execute(pool)
        .await
        .unwrap();
    ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name: "execution reader".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    }
}

async fn seed_dependencies(pool: &sqlx::PgPool, actor: ActorId) -> FixtureIds {
    let ids = FixtureIds {
        platform: Uuid::now_v7(),
        registry: Uuid::now_v7(),
        git_repository: Uuid::now_v7(),
        secret: Uuid::now_v7(),
    };
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(ids.platform).bind(format!("execution-platform-{}", ids.platform.simple())).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{}',$2,$3,'docker.io','Enabled')")
        .bind(ids.registry).bind(actor.value()).bind(format!("execution-registry-{}", ids.registry.simple())).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/repo.git','Idle')")
        .bind(ids.git_repository).bind(actor.value()).bind(format!("execution-git-{}", ids.git_repository.simple())).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(ids.secret)
    .bind(format!("execution-secret-{}", ids.secret.simple()))
    .execute(pool)
    .await
    .unwrap();
    ids
}

fn alert_rule_json(suffix: &str, channel_id: Uuid) -> Value {
    json!({
        "name":format!("rule-{suffix}"),"description":"Original description","type":"BuildRunFailed",
        "severity":"Warning","cooldownSeconds":60,"requiredMatches":null,"threshold":null,
        "status":"Enabled","channelIds":[channel_id],"limitedTo":[],"quietHours":[]
    })
}

fn drain_resource_changes(
    receiver: &mut tokio::sync::broadcast::Receiver<
        Arc<citadel_server::realtime::PublishedRuntimeEvent>,
    >,
    resource_type: &str,
) -> usize {
    let mut count = 0;
    loop {
        match receiver.try_recv() {
            Ok(event) => count += usize::from(event.resource_type() == resource_type),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => return count,
            Err(error) => panic!("Realtime fixture lost events: {error}"),
        }
    }
}

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    principal: Option<ActorPrincipal>,
    body: Option<Value>,
) -> axum::response::Response {
    let content_type = if method == Method::PATCH {
        "application/merge-patch+json"
    } else {
        "application/json"
    };
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-request-id", "alert-parity-request");
    if body.is_some() {
        builder = builder.header("content-type", content_type);
    }
    let mut request = builder
        .body(Body::from(body.map_or_else(Vec::new, |value| {
            serde_json::to_vec(&value).unwrap()
        })))
        .unwrap();
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    app.clone().oneshot(request).await.unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert!(
        status.is_success(),
        "unexpected status {status}: {}",
        String::from_utf8_lossy(&body)
    );
    serde_json::from_slice(&body).unwrap()
}

struct FakeBuildExecutor {
    pool: sqlx::PgPool,
}
impl BuildExecutor for FakeBuildExecutor {
    fn execute<'a>(
        &'a self,
        claim: &'a BuildClaim,
        progress: &'a dyn citadel_builds::BuildLogSink,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, BuildExecutionResult> {
        Box::pin(async move {
            progress.append("stdout", "live\0 output").await.unwrap();
            let persisted: String = sqlx::query_scalar("SELECT message FROM buildrunlogs WHERE buildrunid=$1 ORDER BY createdat,id LIMIT 1")
                .bind(claim.run.id).fetch_one(&self.pool).await.unwrap();
            assert_eq!(
                persisted, "live output",
                "Output must be persisted before the execution completes"
            );
            BuildExecutionResult {
                status: citadel_builds::BuildRunStatus::Succeeded,
                exit_code: Some(0),
                image_digest: Some(format!("sha256:{}", "b".repeat(64))),
                resolved_commit_sha: None,
                image_references: vec!["registry.test/project:latest".into()],
                error_code: None,
                error_message: None,
                logs: vec![BuildLog {
                    stream: "stdout".into(),
                    message: "ok".into(),
                }],
            }
        })
    }
}

struct FakeBackupExecutor;
#[derive(Default)]
struct FakeBackupPlanner {
    fail: std::sync::atomic::AtomicBool,
}
struct AllowBackupExecution;
impl BackupSourcePlanner for FakeBackupPlanner {
    fn cleanup_staging<'a>(
        &'a self,
        plan: &'a citadel_backups::BackupSourcePlan,
    ) -> BoxFuture<'a, Result<(), citadel_backups::BackupError>> {
        Box::pin(async move {
            assert!(plan.local_directory.is_none());
            Ok(())
        })
    }

    fn preview<'a>(
        &'a self,
        _: citadel_backups::policies::read_models::BackupPreviewKind,
        _: Uuid,
        _: ActorId,
        _: bool,
        _: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<citadel_backups::policies::read_models::BackupSourcePreview, BackupError>,
    > {
        Box::pin(async { Err(BackupError::NotFound) })
    }
    fn validate_source<'a>(
        &'a self,
        _: &'a citadel_backups::spec::BackupSourceSpec,
        _: &'a citadel_backups::BackupRepository,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async { Ok(()) })
    }
    fn plan<'a>(
        &'a self,
        claim: &'a BackupClaim,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<BackupSourcePlan, BackupError>> {
        Box::pin(async move {
            if self.fail.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(BackupError::Validation("fixture source unavailable".into()));
            }
            let citadel_backups::spec::BackupSourceSpec::DockerVolume {
                platform_id,
                volume_name,
                ..
            } = &claim.policy.source
            else {
                panic!("fixture expects volume source")
            };
            Ok(BackupSourcePlan {
                display_name: "data".into(),
                items: vec![citadel_backups::BackupSourceItem::new(
                    *platform_id,
                    volume_name.clone(),
                    None,
                    None,
                )],
                warnings: vec![],
                local_directory: None,
            })
        })
    }
}
impl BackupRunAuthorizer for AllowBackupExecution {
    fn authorize_backup<'a>(
        &'a self,
        _: &'a BackupClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async { Ok(()) })
    }
    fn authorize_restore<'a>(
        &'a self,
        _: &'a RestoreClaim,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async { Ok(()) })
    }
}
impl BackupExecutor for FakeBackupExecutor {
    fn repository<'a>(
        &'a self,
        repository: &'a BackupRepository,
        _: &'a str,
        _: &'a str,
        _: Option<Uuid>,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<BackupLog>, BackupError>> {
        Box::pin(async move {
            if matches!(&repository.spec, citadel_backups::spec::BackupRepositorySpec::FileSystem { path, .. } if path == "/outside-allowed-backup-path")
            {
                return Err(BackupError::Validation(
                    "Core repository path is outside Backups__AllowedCorePaths.".into(),
                ));
            }
            Ok(vec![])
        })
    }
    fn backup<'a>(
        &'a self,
        _: &'a BackupClaim,
        plan: &'a BackupSourcePlan,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, BackupExecutionResult> {
        Box::pin(async move {
            BackupExecutionResult {
                status: citadel_backups::BackupRunStatus::Succeeded,
                snapshot_availability: citadel_backups::BackupSnapshotAvailability::Available,
                restic_snapshot_id: None,
                parent_snapshot_id: None,
                files_processed: None,
                bytes_processed: None,
                bytes_added: None,
                exit_code: Some(0),
                error_code: None,
                error_message: None,
                warnings: vec![],
                logs: vec![],
                items: plan
                    .items
                    .iter()
                    .map(|item| citadel_backups::BackupRunItemResult {
                        id: item.id,
                        status: citadel_backups::BackupRunItemStatus::Succeeded,
                        restic_snapshot_id: Some("a".repeat(64)),
                        parent_snapshot_id: None,
                        files_processed: Some(1),
                        bytes_processed: Some(16),
                        bytes_added: Some(16),
                        exit_code: Some(0),
                        error_code: None,
                        error_message: None,
                    })
                    .collect(),
            }
        })
    }
    fn restore<'a>(
        &'a self,
        _: &'a RestoreClaim,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, RestoreExecutionResult> {
        Box::pin(async {
            RestoreExecutionResult {
                status: citadel_backups::BackupRestoreStatus::Succeeded,
                exit_code: Some(0),
                error_code: None,
                error_message: None,
                logs: vec![],
            }
        })
    }
}

struct FakeAlertDelivery;
impl AlertDelivery for FakeAlertDelivery {
    fn send<'a>(
        &'a self,
        _: &'a AlertChannel,
        _: &'a AlertEvent,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn build_run_api_preserves_queued_resource_snapshots() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    let principal = seed_administrator(&pool).await;
    let fixture = seed_dependencies(&pool, principal.actor_id).await;
    let store = PostgresBuildRepository::new(pool.clone());
    let mut input: citadel_builds::BuildProjectConfiguration = serde_json::from_value(json!({
        "name": "snapshot-build", "enabled": true,
        "gitRepositoryId": fixture.git_repository,
        "platformId": fixture.platform, "registryId": fixture.registry,
        "imageRepository": "citadel/test"
    }))
    .unwrap();
    input.validate().unwrap();
    let project = store.create(principal.actor_id, &input).await.unwrap();
    let queued = store
        .enqueue(principal.actor_id, project.id, "Manual")
        .await
        .unwrap();
    sqlx::query("UPDATE platforms SET name='Renamed platform' WHERE id=$1")
        .bind(fixture.platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE gitrepositories SET name='Renamed repository' WHERE id=$1")
        .bind(fixture.git_repository)
        .execute(&pool)
        .await
        .unwrap();
    let persisted = store.get_run(queued.id).await.unwrap();
    for run in [queued, persisted] {
        let response = serde_json::to_value(
            citadel_server::api::resources::builds::views::BuildRunView::try_from(run).unwrap(),
        )
        .unwrap();
        assert_eq!(
            response["platformSnapshot"]["id"],
            fixture.platform.to_string()
        );
        assert_eq!(
            response["platformSnapshot"]["name"],
            format!("execution-platform-{}", fixture.platform.simple())
        );
        assert_eq!(
            response["gitRepositoryNameSnapshot"],
            format!("execution-git-{}", fixture.git_repository.simple())
        );
    }
    pool.close().await;
}
