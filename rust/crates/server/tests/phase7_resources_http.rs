use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use chrono::Duration;
use citadel_adapters::alert_store::PostgresAlertStore;
use citadel_adapters::backup_store::PostgresBackupStore;
use citadel_adapters::build_store::PostgresBuildStore;
use citadel_adapters::crypto::{
    Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_alerts::{AlertChannelView, AlertDelivery, AlertError, AlertEventView};
use citadel_backups::{
    BackupClaim, BackupError, BackupExecutionResult, BackupExecutor, BackupLog,
    BackupRepositoryView, BackupRunAuthorizer, BackupService, BackupSourcePlan,
    BackupSourcePlanner, RestoreClaim, RestoreExecutionResult,
};
use citadel_builds::{BuildClaim, BuildExecutionResult, BuildExecutor, BuildLog, BuildService};
use citadel_database::MigrationRunner;
use citadel_domain::{ActorId, AuthenticatedPrincipalType, ResourceType};
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, IdentityService, NoopServiceAccountLastUsedTracker,
    SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_server::alerts_http::{self, AlertsHttpState};
use citadel_server::backups_http::{self, BackupsHttpState};
use citadel_server::builds_http::{self, BuildsHttpState};
use citadel_server::metrics::Metrics;
use citadel_server::realtime::{RealtimeHub, change_callback};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn phase7_resource_endpoints_authorize_validate_and_persist_lifecycles() {
    let database_url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
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
    let build_store = Arc::new(PostgresBuildStore::new(pool.clone()));
    let backup_store = Arc::new(PostgresBackupStore::new(pool.clone()));
    let alert_store = Arc::new(PostgresAlertStore::new(pool.clone()));
    let hub = RealtimeHub::new(128, Arc::new(Metrics::default()));
    let _subscriber = hub.subscribe();
    let builds = Arc::new(
        BuildService::new(
            build_store,
            Arc::new(FakeBuildExecutor),
            Duration::minutes(5),
        )
        .with_change_notifier(change_callback(Some(hub.clone()), "Build")),
    );
    let backups = Arc::new(
        BackupService::new(
            backup_store,
            Arc::new(FakeBackupExecutor),
            Arc::new(FakeBackupPlanner),
            Duration::minutes(5),
            Arc::new(AllowBackupExecution),
        )
        .with_change_notifier(change_callback(Some(hub.clone()), "BackupPolicy")),
    );
    let cancellation = CancellationToken::new();
    let app = builds_http::router(BuildsHttpState {
        identity: Arc::clone(&identity),
        builds: builds.clone(),
    })
    .merge(backups_http::router(BackupsHttpState {
        identity: Arc::clone(&identity),
        backups: backups.clone(),
        cancellation,
    }))
    .merge(alerts_http::router(AlertsHttpState {
        identity,
        store: alert_store,
        delivery: Arc::new(FakeAlertDelivery),
    }))
    .layer(axum::Extension(hub.clone()))
    .layer(axum::Extension(
        citadel_server::platforms_http::EdgeHttpContext {
            store: citadel_adapters::edge::PostgresEdgeStore::new(pool.clone()),
            registry: citadel_adapters::edge::EdgeRegistry::default(),
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
    let suffix = Uuid::now_v7().simple().to_string();

    let build_pool_response = request(
        &app,
        Method::POST,
        "/api/v1/buildAgentPools",
        Some(principal.clone()),
        Some(json!({
            "name":format!("pool-{suffix}"), "enabled":true,
            "providerSpec":{"$type":"GenericEdge","connectionMode":"EdgeAgent"}
        })),
    )
    .await;
    assert_eq!(build_pool_response.status(), StatusCode::OK);
    let build_pool = response_json(build_pool_response).await;
    let pool_id = build_pool["id"].as_str().unwrap();
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

    let before_channel = hub.current_revision();
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
        hub.current_revision(),
        before_channel + 1,
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
        hub.current_revision(),
        before_channel + 1,
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
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,$4,0)")
            .bind(Uuid::now_v7())
            .bind(reader.actor_id.value())
            .bind(resource_id)
            .bind(resource_type as i32)
            .execute(&pool)
            .await
            .unwrap();
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
    assert_eq!(hub.current_revision(), before + 2);
    let status: String = sqlx::query_scalar("SELECT status FROM buildruns WHERE id=$1")
        .bind(Uuid::parse_str(run["id"].as_str().unwrap()).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Succeeded");
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
        .bind("Phase 7 admin")
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
        name: "Phase 7 admin".into(),
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
        .bind("Phase 7 reader")
        .execute(pool)
        .await
        .unwrap();
    ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name: "Phase 7 reader".into(),
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
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)")
        .bind(ids.platform).bind(format!("phase7-platform-{}", ids.platform.simple())).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{}',$2,$3,'docker.io','Enabled')")
        .bind(ids.registry).bind(actor.value()).bind(format!("phase7-registry-{}", ids.registry.simple())).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/repo.git','Idle')")
        .bind(ids.git_repository).bind(actor.value()).bind(format!("phase7-git-{}", ids.git_repository.simple())).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(ids.secret)
    .bind(format!("phase7-secret-{}", ids.secret.simple()))
    .execute(pool)
    .await
    .unwrap();
    ids
}

fn alert_rule_json(suffix: &str, channel_id: Uuid) -> Value {
    json!({
        "name":format!("rule-{suffix}"),"description":null,"type":"BuildRunFailed",
        "severity":"Critical","cooldownSeconds":60,"requiredMatches":null,"threshold":null,
        "status":"Enabled","channelIds":[channel_id],"limitedTo":[],"quietHours":[]
    })
}

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    principal: Option<ActorPrincipal>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
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
    assert!(
        response.status().is_success(),
        "unexpected status {}",
        response.status()
    );
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

struct FakeBuildExecutor;
impl BuildExecutor for FakeBuildExecutor {
    fn execute<'a>(
        &'a self,
        _: &'a BuildClaim,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, BuildExecutionResult> {
        Box::pin(async {
            BuildExecutionResult {
                status: "Succeeded",
                exit_code: Some(0),
                image_digest: None,
                resolved_commit_sha: None,
                image_references: vec![],
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
struct FakeBackupPlanner;
struct AllowBackupExecution;
impl BackupSourcePlanner for FakeBackupPlanner {
    fn plan<'a>(
        &'a self,
        _: &'a BackupClaim,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<BackupSourcePlan, BackupError>> {
        Box::pin(async { Err(BackupError::Validation("unused fixture".into())) })
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
        _: &'a BackupRepositoryView,
        _: &'a str,
        _: &'a str,
        _: Option<Uuid>,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<BackupLog>, BackupError>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn backup<'a>(
        &'a self,
        _: &'a BackupClaim,
        _: &'a BackupSourcePlan,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, BackupExecutionResult> {
        Box::pin(async {
            BackupExecutionResult {
                status: "Succeeded",
                snapshot_availability: "Available",
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
                items: vec![],
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
                status: "Succeeded",
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
        _: &'a AlertChannelView,
        _: &'a AlertEventView,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async { Ok(()) })
    }
}
