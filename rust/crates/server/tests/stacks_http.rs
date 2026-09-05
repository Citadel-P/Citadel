use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use chrono::Duration;
use citadel_adapters::crypto::{
    Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_adapters::stack_store::PostgresStackStore;
use citadel_database::MigrationRunner;
use citadel_domain::{ActorId, AuthenticatedPrincipalType};
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, IdentityService, NoopServiceAccountLastUsedTracker,
    SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_server::stacks_http::{self, StacksHttpState};
use citadel_stacks::{
    ComposeProjectRuntimeService, NoopStackChangeNotifier, ResolvedStackBindings, StackApplySource,
    StackBindingResolverPort, StackDeletionClaim, StackError, StackImportClaim, StackImportKind,
    StackOperationClaim, StackOrchestrationMode, StackReleaseStatus, StackRuntimePort,
    StackRuntimeResult, StackRuntimeSnapshot, StackService,
};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct ApplyCall {
    platform_type: String,
    compose: String,
    environment: Vec<String>,
}

#[derive(Default)]
struct CompletingStackRuntime {
    apply_calls: Mutex<Vec<ApplyCall>>,
    import_claim: Mutex<Option<StackImportClaim>>,
    state_failure: AtomicU8,
}

impl StackRuntimePort for CompletingStackRuntime {
    fn apply<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        source: &'a StackApplySource,
        environment: &'a [String],
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackRuntimeResult, StackError>> {
        Box::pin(async move {
            self.apply_calls.lock().unwrap().push(ApplyCall {
                platform_type: claim.platform_type.clone(),
                compose: source.compose_contents()?.join("\n"),
                environment: environment.to_vec(),
            });
            Ok(healthy())
        })
    }

    fn observe<'a>(
        &'a self,
        _claim: &'a StackOperationClaim,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<StackRuntimeResult>, StackError>> {
        Box::pin(async { Ok(Some(healthy())) })
    }

    fn delete<'a>(
        &'a self,
        _claim: &'a StackDeletionClaim,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async { Ok(()) })
    }

    fn change_state<'a>(
        &'a self,
        _platform_id: Uuid,
        _project_name: &'a str,
        _orchestration: StackOrchestrationMode,
        _action: citadel_stacks::StackAction,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, StackError>> {
        Box::pin(async move {
            match self.state_failure.load(Ordering::Relaxed) {
                1 => Err(StackError::RuntimeRejected(
                    "state command rejected".to_owned(),
                )),
                2 => Err(StackError::Runtime("transport lost".to_owned())),
                _ => Ok(vec!["container-1".to_owned()]),
            }
        })
    }

    fn runtime_snapshot<'a>(
        &'a self,
        _platform_id: Uuid,
        _project_name: &'a str,
        _orchestration: StackOrchestrationMode,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackRuntimeSnapshot, StackError>> {
        Box::pin(async {
            Ok(StackRuntimeSnapshot {
                containers: Vec::new(),
                services: Vec::new(),
            })
        })
    }

    fn reconcile<'a>(
        &'a self,
        _platform_id: Uuid,
        _drifts: &'a [citadel_stacks::StackDrift],
        _policy: &'a citadel_stacks::StackDriftPolicy,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<citadel_stacks::StackReconciliationAction>, StackError>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn import_claim<'a>(
        &'a self,
        _platform_id: Uuid,
        _project_name: &'a str,
        _import_kind: Option<StackImportKind>,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackImportClaim, StackError>> {
        Box::pin(async move {
            self.import_claim
                .lock()
                .unwrap()
                .clone()
                .ok_or(StackError::NotFound)
        })
    }
}

struct EmptyBindings;

impl StackBindingResolverPort for EmptyBindings {
    fn resolve<'a>(
        &'a self,
        _stack_id: Uuid,
        _names: &'a [String],
    ) -> BoxFuture<'a, Result<ResolvedStackBindings, StackError>> {
        Box::pin(async { Ok(ResolvedStackBindings::default()) })
    }
}

fn healthy() -> StackRuntimeResult {
    StackRuntimeResult {
        status: StackReleaseStatus::Healthy,
        messages: Vec::new(),
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn stack_endpoints_enforce_auth_and_persist_apply_release_and_delete() {
    let database_url = std::env::var("CITADEL_PHASE6_DATABASE_URL")
        .expect("CITADEL_PHASE6_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(&[73_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let runtime = Arc::new(CompletingStackRuntime::default());
    let stacks = Arc::new(StackService::new(
        Arc::new(PostgresStackStore::new(pool.clone())),
        runtime.clone(),
        Arc::new(EmptyBindings),
        Arc::new(NoopStackChangeNotifier),
        CancellationToken::new(),
    ));
    let app = stacks_http::router(StacksHttpState { identity, stacks });
    assert_eq!(
        request(&app, Method::GET, "/api/v1/stacks", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let platform_id = Uuid::now_v7();
    let swarm_platform_id = Uuid::now_v7();
    let suffix = user_id.simple().to_string();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
        .bind(actor_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdat,createdbyactorid,email,name) VALUES($1,$2,CURRENT_TIMESTAMP,$3,$4,$5)")
        .bind(user_id).bind(actor_id).bind(SYSTEM_ACTOR_ID).bind(format!("stack-{suffix}@example.test")).bind(format!("stack-{suffix}"))
        .execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(r#"INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
        VALUES($1,$2,'Local',1,0,1,$3,0,'{"$type":"Docker"}'::json,'Online',0)"#)
        .bind(platform_id)
        .bind(format!("local-{suffix}"))
        .bind(format!("platform-{suffix}"))
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(r#"INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
        VALUES($1,$2,'Local',1,0,1,$3,0,'{"$type":"DockerSwarm"}'::json,'Online',0)"#)
        .bind(swarm_platform_id).bind(format!("swarm-local-{suffix}")).bind(format!("swarm-platform-{suffix}")).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let admin = ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name: format!("stack-{suffix}"),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".to_owned()],
    };

    let created_response = request(
        &app,
        Method::POST,
        "/api/v1/stacks",
        Some(admin.clone()),
        Some(json!({
            "name":format!("stack-{suffix}"),
            "platformId":platform_id,
            "description":"fixture",
            "stackSource":"WebEditor",
            "spec":{"$type":"WebEditor","composeFile":"services:\n  web:\n    image: nginx:alpine\n","updateBehavior":"Disabled","destroyBeforeDeploy":false},
            "driftPolicy":{"mode":"DetectOnly","alertOnDrift":true,"markDegraded":true,"autoStartStoppedContainers":false,"autoResumePausedContainers":false,"removeExtraContainers":false}
        })),
    ).await;
    let status = created_response.status();
    let created = response_json(created_response).await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();

    let cleared = response_json(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/stacks/{id}"),
            Some(admin.clone()),
            Some(json!({
                "description": null,
                "rowVersion": created["rowVersion"]
            })),
        )
        .await,
    )
    .await;
    assert_eq!(cleared["description"], Value::Null);

    let apply = request(
        &app,
        Method::POST,
        "/api/v1/stacks/apply",
        Some(admin.clone()),
        Some(json!({"id":id})),
    )
    .await;
    assert_eq!(apply.status(), StatusCode::OK);
    let progress = response_json(apply).await;
    assert_eq!(
        progress.as_array().unwrap().last().unwrap()["stackStatus"],
        "Healthy"
    );
    {
        let calls = runtime.apply_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].platform_type, "Docker");
        assert!(calls[0].environment.is_empty());
        assert!(calls[0].compose.contains("com.citadel.stack-id"));
        assert!(calls[0].compose.contains("com.citadel.release-id"));
        assert!(!calls[0].compose.contains("deploy:\n  labels:"));
    }
    let detail = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/stacks/{id}"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(detail["status"], "Healthy");
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks/stop",
            Some(admin.clone()),
            Some(json!([id])),
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let stopped = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/stacks/{id}"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(stopped["status"], "Stopped");
    assert_eq!(stopped["controlState"], "Idle");
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks/start",
            Some(admin.clone()),
            Some(json!([id])),
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    runtime.state_failure.store(1, Ordering::Relaxed);
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks/pause",
            Some(admin.clone()),
            Some(json!([id])),
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    let rejected_state = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/stacks/{id}"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(rejected_state["status"], "Healthy");
    assert_eq!(rejected_state["controlState"], "Idle");

    runtime.state_failure.store(2, Ordering::Relaxed);
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks/pause",
            Some(admin.clone()),
            Some(json!([id])),
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    let ambiguous_state = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/stacks/{id}"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(ambiguous_state["status"], "Pending");
    assert_eq!(ambiguous_state["controlState"], "Processing");
    runtime.state_failure.store(0, Ordering::Relaxed);
    sqlx::query("UPDATE stackreleases SET status='Healthy' WHERE stackid=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let releases = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/stacks/{id}/releases"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert!(releases["releases"].is_array());
    assert!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM activityevents WHERE resourceid=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap()
            >= 2
    );

    let swarm_created = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks",
            Some(admin.clone()),
            Some(json!({
                "name":format!("swarm-stack-{suffix}"),
                "platformId":swarm_platform_id,
                "stackSource":"WebEditor",
                "spec":{"$type":"WebEditor","composeFile":"services:\n  api:\n    image: nginx:alpine\n","updateBehavior":"Disabled","destroyBeforeDeploy":false}
            })),
        )
        .await,
    )
    .await;
    let swarm_id = Uuid::parse_str(swarm_created["id"].as_str().unwrap()).unwrap();
    let swarm_apply = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks/apply",
            Some(admin.clone()),
            Some(json!({"id":swarm_id})),
        )
        .await,
    )
    .await;
    assert_eq!(
        swarm_apply.as_array().unwrap().last().unwrap()["stackStatus"],
        "Healthy"
    );
    {
        let calls = runtime.apply_calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].platform_type, "DockerSwarm");
        assert!(calls[1].compose.contains("deploy:"));
        assert!(calls[1].compose.matches("com.citadel.stack-id").count() >= 2);
    }
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks/stop",
            Some(admin.clone()),
            Some(json!([id, swarm_id])),
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let unclaimed_standalone = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/stacks/{id}"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(unclaimed_standalone["status"], "Healthy");
    assert_eq!(unclaimed_standalone["controlState"], "Idle");

    let defaulted_created = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks",
            Some(admin.clone()),
            Some(json!({
                "name":format!("defaulted-variable-{suffix}"),
                "platformId":swarm_platform_id,
                "stackSource":"WebEditor",
                "spec":{"$type":"WebEditor","composeFile":"services:\n  api:\n    image: nginx:${TAG:-alpine}\n","updateBehavior":"Disabled","destroyBeforeDeploy":false}
            })),
        )
        .await,
    )
    .await;
    let defaulted_id = Uuid::parse_str(defaulted_created["id"].as_str().unwrap()).unwrap();
    let defaulted_apply = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks/apply",
            Some(admin.clone()),
            Some(json!({"id":defaulted_id})),
        )
        .await,
    )
    .await;
    assert_eq!(
        defaulted_apply.as_array().unwrap().last().unwrap()["stackStatus"],
        "Healthy"
    );

    let invalid_created = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks",
            Some(admin.clone()),
            Some(json!({
                "name":format!("undefined-variable-{suffix}"),
                "platformId":swarm_platform_id,
                "stackSource":"WebEditor",
                "spec":{"$type":"WebEditor","composeFile":"services:\n  api:\n    image: nginx:alpine\n    environment:\n      REQUIRED: ${REQUIRED:?required}\n","updateBehavior":"Disabled","destroyBeforeDeploy":false}
            })),
        )
        .await,
    )
    .await;
    let invalid_id = Uuid::parse_str(invalid_created["id"].as_str().unwrap()).unwrap();
    let rejected = response_json(
        request(
            &app,
            Method::POST,
            "/api/v1/stacks/apply",
            Some(admin.clone()),
            Some(json!({"id":invalid_id})),
        )
        .await,
    )
    .await;
    let final_item = rejected.as_array().unwrap().last().unwrap();
    assert_eq!(final_item["stackStatus"], "Failed");
    assert!(
        final_item["message"]
            .as_str()
            .unwrap()
            .contains("undefined Citadel variable")
    );
    assert_eq!(runtime.apply_calls.lock().unwrap().len(), 3);

    let import_namespace = format!("runtime-{suffix}");
    sqlx::query(
        r#"INSERT INTO swarmserviceprojections(
               platformid,dockerserviceid,configids,desiredtaskcount,dockerstacknamespace,
               forceupdate,image,isstale,labels,mode,name,networkids,observedat,ownership,
               ports,runningtaskcount,secretids,updatestate,versionindex)
           VALUES($1,$2,'[]',1,$3,0,'nginx:alpine',FALSE,'{}','Replicated',$4,'[]',CURRENT_TIMESTAMP,
                  'DockerStackExternal','[]',1,'[]','completed',4)"#,
    )
    .bind(swarm_platform_id)
    .bind(format!("import-service-{suffix}"))
    .bind(&import_namespace)
    .bind(format!("{import_namespace}_api"))
    .execute(&pool)
    .await
    .unwrap();
    *runtime.import_claim.lock().unwrap() = Some(StackImportClaim {
        platform_id: swarm_platform_id,
        platform_name: format!("swarm-platform-{suffix}"),
        project_name: import_namespace.clone(),
        import_kind: StackImportKind::SwarmStack,
        runtime_fingerprint: format!("sha256:runtime-{suffix}"),
        service_names: vec!["api".to_owned()],
        container_ids: Vec::new(),
        container_names: Vec::new(),
        services: vec![ComposeProjectRuntimeService {
            name: "api".to_owned(),
            image: Some("nginx:alpine".to_owned()),
            container_count: 1,
            states: vec!["completed".to_owned(), "running:1".to_owned()],
        }],
    });
    let import_path = format!(
        "/api/v1/platforms/{swarm_platform_id}/unmanaged-compose-projects/{import_namespace}"
    );
    let draft = response_json(
        request(
            &app,
            Method::GET,
            &format!("{import_path}?importKind=SwarmStack"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(draft["importKind"], "SwarmStack");
    assert_eq!(draft["source"]["services"][0]["name"], "api");
    let spec = json!({
        "$type":"WebEditor",
        "composeFile":"services:\n  api:\n    image: nginx:alpine\n",
        "updateBehavior":"Disabled",
        "projectName":import_namespace,
        "destroyBeforeDeploy":false
    });
    let validation = response_json(
        request(
            &app,
            Method::POST,
            &format!("{import_path}/import-draft"),
            Some(admin.clone()),
            Some(json!({
                "name":format!("imported-{suffix}"),
                "stackSource":"WebEditor",
                "spec":spec,
                "importKind":"SwarmStack"
            })),
        )
        .await,
    )
    .await;
    assert_eq!(validation["issues"], json!([]));
    let imported_response = request(
        &app,
        Method::POST,
        &format!("{import_path}/import"),
        Some(admin.clone()),
        Some(json!({
            "name":format!("imported-{suffix}"),
            "description":"Imported fixture",
            "stackSource":"WebEditor",
            "spec":spec,
            "previewFingerprint":validation["previewFingerprint"],
            "importKind":"SwarmStack"
        })),
    )
    .await;
    let imported_status = imported_response.status();
    let imported = response_json(imported_response).await;
    assert_eq!(imported_status, StatusCode::OK, "{imported}");
    let imported_id = Uuid::parse_str(imported["id"].as_str().unwrap()).unwrap();
    assert_eq!(runtime.apply_calls.lock().unwrap().len(), 3);
    assert_eq!(
        sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT stackid FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid=$2"
        )
        .bind(swarm_platform_id)
        .bind(format!("import-service-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(imported_id),
        "import atomically claims ownership without mutating Docker"
    );

    assert_eq!(
        request(
            &app,
            Method::DELETE,
            "/api/v1/stacks",
            Some(admin),
            Some(json!([id, swarm_id, defaulted_id, invalid_id, imported_id]))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stacks WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );

    sqlx::query("DELETE FROM activityevents WHERE createdbyactorid=$1")
        .bind(actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(swarm_platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor_id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    principal: Option<ActorPrincipal>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut request = Request::builder().method(method).uri(uri);
    if body.is_some() {
        request = request.header("content-type", "application/json");
    }
    let mut request = request
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap();
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    app.clone().oneshot(request).await.unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}
