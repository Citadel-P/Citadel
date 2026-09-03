use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use chrono::Duration;
use citadel_adapters::crypto::{
    Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::deployment_store::PostgresDeploymentStore;
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_database::MigrationRunner;
use citadel_deployments::{DeploymentError, DeploymentRuntimePort, DeploymentService};
use citadel_domain::{ActorId, AuthenticatedPrincipalType};
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, IdentityService, NoopServiceAccountLastUsedTracker,
    SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_server::deployments_http::{self, DeploymentsHttpState};
use futures_util::{FutureExt, future::BoxFuture};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

struct RecordingRuntime {
    deleted: Arc<Mutex<Vec<String>>>,
    fail: Arc<AtomicBool>,
}

impl DeploymentRuntimePort for RecordingRuntime {
    fn delete_container<'a>(
        &'a self,
        _platform_id: Uuid,
        docker_container_id: &'a str,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        async move {
            if self.fail.load(Ordering::Acquire) {
                return Err(DeploymentError::Runtime(
                    "Docker rejected deletion".to_owned(),
                ));
            }
            self.deleted
                .lock()
                .unwrap()
                .push(docker_container_id.to_owned());
            Ok(())
        }
        .boxed()
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn deployment_endpoints_enforce_auth_and_persist_the_crud_lifecycle() {
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
            JwtSessionTokenCodec::new(&[53_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let deleted = Arc::new(Mutex::new(Vec::new()));
    let fail_runtime = Arc::new(AtomicBool::new(false));
    let service = Arc::new(DeploymentService::new(
        Arc::new(PostgresDeploymentStore::new(pool.clone())),
        Arc::new(RecordingRuntime {
            deleted: Arc::clone(&deleted),
            fail: Arc::clone(&fail_runtime),
        }),
        Arc::new(StaticEntitlementService::new(true)),
        CancellationToken::new(),
    ));
    let app = deployments_http::router(DeploymentsHttpState {
        identity,
        deployments: service,
    });

    assert_eq!(
        request(&app, Method::GET, "/api/v1/deployments", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let platform_id = Uuid::now_v7();
    let suffix = user_id.simple().to_string();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
        .bind(actor_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdat,createdbyactorid,email,name) VALUES($1,$2,CURRENT_TIMESTAMP,$3,$4,$5)")
        .bind(user_id).bind(actor_id).bind(SYSTEM_ACTOR_ID)
        .bind(format!("phase6-{suffix}@example.test")).bind(format!("phase6-{suffix}"))
        .execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        r#"INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
           VALUES($1,'local','Local',1,0,1,$2,0,'{"$type":"Docker"}'::json,'Online',0)"#,
    )
    .bind(platform_id).bind(format!("phase6-platform-{suffix}"))
    .execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let admin = ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name: format!("phase6-{suffix}"),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".to_owned()],
    };

    let created_input = json!({
        "name": format!("web-{suffix}"),
        "platformId": platform_id,
        "description": "initial",
        "spec": {
            "image": {"$type":"Local", "imageId":"sha256:test"},
            "updateBehavior":"Disabled"
        }
    });
    let created_response = request(
        &app,
        Method::POST,
        "/api/v1/deployments",
        Some(admin.clone()),
        Some(created_input.clone()),
    )
    .await;
    let created_status = created_response.status();
    let created = response_json(created_response).await;
    assert_eq!(created_status, StatusCode::OK, "{created}");
    let deployment_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    assert_eq!(created["status"], "Created");
    assert!(created["spec"].get("lifeCycleSpec").is_none());
    assert!(created.get("containerId").is_none());

    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/deployments",
            Some(admin.clone()),
            Some(created_input),
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );

    let external_response = request(
        &app,
        Method::POST,
        "/api/v1/deployments",
        Some(admin.clone()),
        Some(json!({
            "name": format!("external-{suffix}"),
            "platformId": platform_id,
            "spec": {
                "image": {
                    "$type":"External",
                    "registryId":"00000000-0000-0000-0000-000000000100",
                    "imageTag":"nginx"
                },
                "updateBehavior":"Notify",
                "ports":[],
                "networks":["frontend"]
            }
        })),
    )
    .await;
    assert_eq!(external_response.status(), StatusCode::OK);
    let external = response_json(external_response).await;
    assert!(external.get("description").is_none());
    assert!(external["spec"]["image"].get("resolvedDigest").is_none());

    for invalid in [
        json!({
            "name": format!("internal-{suffix}"), "platformId": platform_id,
            "spec": {
                "image": {"$type":"Internal", "registryId":"00000000-0000-0000-0000-000000000100", "imageTag":"nginx"},
                "updateBehavior":"AutoDeploy"
            }
        }),
        json!({
            "name": format!("pinned-{suffix}"), "platformId": platform_id,
            "spec": {
                "image": {"$type":"External", "registryId":"00000000-0000-0000-0000-000000000100", "imageTag":"nginx@sha256:abc"},
                "updateBehavior":"AutoDeploy"
            }
        }),
    ] {
        assert_eq!(
            request(
                &app,
                Method::POST,
                "/api/v1/deployments",
                Some(admin.clone()),
                Some(invalid),
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }

    let full_patch = request(
        &app,
        Method::PATCH,
        &format!("/api/v1/deployments/{deployment_id}"),
        Some(admin.clone()),
        Some(json!({
            "platformId": platform_id,
            "spec": {
                "updateBehavior":"Disabled",
                "image": {
                    "$type":"Local",
                    "registryId":"00000000-0000-0000-0000-000000000100",
                    "imageTag":"nginx",
                    "imageId":"019b6552-be84-7649-a6b0-c3a73a2df59c"
                },
                "ports":["2220-27017/tcp"],
                "networks":["frontend"],
                "resourceSpec":{"nanoCpus":0.25,"memoryLimit":256},
                "lifeCycleSpec":{"stopSignal":"SIGKILL","stopTimeout":15},
                "labels":{"key1":"val1"},
                "command":["--housekeeping_interval=5s"]
            }
        })),
    )
    .await;
    assert_eq!(full_patch.status(), StatusCode::OK);
    let full_patch = response_json(full_patch).await;
    assert_eq!(full_patch["spec"]["lifeCycleSpec"]["restartPolicy"], "No");
    assert_eq!(full_patch["spec"]["labels"]["key1"], "val1");

    let partial_patch = request(
        &app,
        Method::PATCH,
        &format!("/api/v1/deployments/{deployment_id}"),
        Some(admin.clone()),
        Some(json!({"spec":{"updateBehavior":"disabled"}})),
    )
    .await;
    assert_eq!(partial_patch.status(), StatusCode::OK);
    let partial_patch = response_json(partial_patch).await;
    assert_eq!(partial_patch["spec"]["ports"][0], "2220-27017/tcp");
    assert_eq!(partial_patch["spec"]["updateBehavior"], "Disabled");

    for invalid_patch in [
        json!({"platformId":Uuid::now_v7()}),
        json!({"name":"forged-audit-name"}),
        json!([]),
        json!({"spec":{"image":{"$type":"Internal","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx"}}}),
        json!({"spec":{"image":{"$type":"External","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx@sha256:abc"},"updateBehavior":"Notify"}}),
    ] {
        assert_eq!(
            request(
                &app,
                Method::PATCH,
                &format!("/api/v1/deployments/{deployment_id}"),
                Some(admin.clone()),
                Some(invalid_patch),
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>("SELECT platformid FROM deployments WHERE id=$1")
            .bind(deployment_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        platform_id
    );

    let draft_response = request(
        &app,
        Method::GET,
        &format!("/api/v1/deployments/{deployment_id}/duplicate-draft"),
        Some(admin.clone()),
        None,
    )
    .await;
    assert_eq!(draft_response.status(), StatusCode::OK);
    let draft = response_json(draft_response).await;
    assert!(draft["draft"]["name"].as_str().unwrap().ends_with("-copy"));
    assert_eq!(
        draft["draft"]["duplicateSource"]["resourceId"],
        deployment_id.to_string()
    );
    assert!(
        draft["draft"]["spec"]["image"]
            .get("resolvedDigest")
            .is_none()
    );
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/deployments",
            Some(admin.clone()),
            Some(draft["draft"].clone()),
        )
        .await
        .status(),
        StatusCode::OK
    );

    let invalid_source_name = format!("invalid-source-{suffix}");
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/deployments",
            Some(admin.clone()),
            Some(json!({
                "name":invalid_source_name.clone(), "platformId":platform_id,
                "spec":{"image":{"$type":"Local","imageId":"sha256:test"},"updateBehavior":"Disabled"},
                "duplicateSource":{"resourceType":"Stack","resourceId":deployment_id,"resourceName":"forged"}
            })),
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let missing_source_name = format!("missing-source-{suffix}");
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/deployments",
            Some(admin.clone()),
            Some(json!({
                "name":missing_source_name.clone(), "platformId":platform_id,
                "spec":{"image":{"$type":"Local","imageId":"sha256:test"},"updateBehavior":"Disabled"},
                "duplicateSource":{"resourceType":"Deployment","resourceId":Uuid::now_v7(),"resourceName":"missing"}
            })),
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deployments WHERE name=ANY($1::text[])")
            .bind(vec![invalid_source_name, missing_source_name])
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );

    let reader_actor_id = Uuid::now_v7();
    let reader_user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
        .bind(reader_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdat,createdbyactorid,email,name) VALUES($1,$2,CURRENT_TIMESTAMP,$3,$4,$5)")
        .bind(reader_user_id)
        .bind(reader_actor_id)
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("reader-{suffix}@example.test"))
        .bind(format!("reader-{suffix}"))
        .execute(&pool)
        .await
        .unwrap();
    let reader = ActorPrincipal {
        subject_id: reader_user_id,
        actor_id: ActorId::new(reader_actor_id),
        name: format!("reader-{suffix}"),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: Vec::new(),
    };
    assert_eq!(
        request(
            &app,
            Method::POST,
            "/api/v1/deployments",
            Some(reader.clone()),
            Some(json!({
                "name":format!("forbidden-{suffix}"), "platformId":platform_id,
                "spec":{"image":{"$type":"Local","imageId":"sha256:test"},"updateBehavior":"Disabled"}
            })),
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    for path in [
        format!("/api/v1/deployments/{deployment_id}"),
        format!("/api/v1/deployments/{deployment_id}/_cfg"),
        format!("/api/v1/deployments/{deployment_id}/duplicate-draft"),
    ] {
        assert_eq!(
            request(&app, Method::GET, &path, Some(reader.clone()), None)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }

    let team_creator_actor_id = Uuid::now_v7();
    let team_creator_user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
        .bind(team_creator_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdat,createdbyactorid,email,name) VALUES($1,$2,CURRENT_TIMESTAMP,$3,$4,$5)")
        .bind(team_creator_user_id)
        .bind(team_creator_actor_id)
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("team-creator-{suffix}@example.test"))
        .bind(format!("team-creator-{suffix}"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorteammemberships(memberactorid,teamid) VALUES($1,'20000000-0000-0000-0000-000000000001')")
        .bind(team_creator_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    let team_creator = ActorPrincipal {
        subject_id: team_creator_user_id,
        actor_id: ActorId::new(team_creator_actor_id),
        name: format!("team-creator-{suffix}"),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: Vec::new(),
    };
    let team_created_response = request(
        &app,
        Method::POST,
        "/api/v1/deployments",
        Some(team_creator),
        Some(json!({
            "name":format!("team-created-{suffix}"), "platformId":platform_id,
            "spec":{
                "image":{"$type":"External","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx"},
                "updateBehavior":"Notify"
            }
        })),
    )
    .await;
    assert_eq!(team_created_response.status(), StatusCode::OK);
    let team_created_id = Uuid::parse_str(
        response_json(team_created_response).await["id"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/deployments/{team_created_id}"),
            Some(reader.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );

    sqlx::query(
        "INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,1,0)",
    )
    .bind(Uuid::now_v7())
    .bind(reader_actor_id)
    .bind(deployment_id)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/deployments/{deployment_id}"),
            Some(reader.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::OK
    );
    sqlx::query("UPDATE resourceaccesses SET permissionlevel=2 WHERE actorid=$1 AND resourceid=$2")
        .bind(reader_actor_id)
        .bind(deployment_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/deployments/{deployment_id}"),
            Some(reader.clone()),
            Some(json!({"spec":{"ports":["9000:90"]}})),
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        sqlx::query_scalar::<_, Value>("SELECT spec FROM deployments WHERE id=$1")
            .bind(deployment_id)
            .fetch_one(&pool)
            .await
            .unwrap()["Ports"][0],
        "2220-27017/tcp"
    );

    assert_eq!(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/deployments/{deployment_id}/_cfg"),
            Some(admin.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::OK
    );
    let renamed = request(
        &app,
        Method::POST,
        "/api/v1/deployments/rename",
        Some(admin.clone()),
        Some(json!({"id":deployment_id,"name":format!("renamed-{suffix}")})),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    let patched = request(
        &app,
        Method::PATCH,
        &format!("/api/v1/deployments/{deployment_id}/_metadata"),
        Some(admin.clone()),
        Some(json!({"description":null,"tags":[]})),
    )
    .await;
    assert_eq!(patched.status(), StatusCode::OK);
    assert_eq!(response_json(patched).await["description"], Value::Null);

    for (docker_id, updated) in [("docker-container", 1_i64), ("older-container", 0_i64)] {
        sqlx::query(
            r#"INSERT INTO containers(id,created,deploymentid,dockercontainerid,dockerimageid,issystem,name,platformid,ports,state,updated)
               VALUES($1,1,$2,$3,'sha256:test',FALSE,'web',$4,'[]'::json,'running',$5)"#,
        )
        .bind(Uuid::now_v7())
        .bind(deployment_id)
        .bind(docker_id)
        .bind(platform_id)
        .bind(updated)
        .execute(&pool)
        .await
        .unwrap();
    }
    let deleted_response = request(
        &app,
        Method::DELETE,
        "/api/v1/deployments",
        Some(admin.clone()),
        Some(json!([deployment_id])),
    )
    .await;
    assert_eq!(deleted_response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        &*deleted.lock().unwrap(),
        &["docker-container", "older-container"]
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deployments WHERE id=$1")
            .bind(deployment_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM containers WHERE deploymentid=$1")
            .bind(deployment_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );

    let failed_create = request(
        &app,
        Method::POST,
        "/api/v1/deployments",
        Some(admin.clone()),
        Some(json!({
            "name": format!("failed-delete-{suffix}"), "platformId": platform_id,
            "description": null,
            "spec": {"image":{"$type":"Local","imageId":"sha256:test"},"updateBehavior":"Disabled"}
        })),
    )
    .await;
    let failed_id =
        Uuid::parse_str(response_json(failed_create).await["id"].as_str().unwrap()).unwrap();
    sqlx::query(
        r#"INSERT INTO containers(id,created,deploymentid,dockercontainerid,dockerimageid,issystem,name,platformid,ports,state,updated)
           VALUES($1,1,$2,'docker-failure','sha256:test',FALSE,'failed',$3,'[]'::json,'running',1)"#,
    )
    .bind(Uuid::now_v7())
    .bind(failed_id)
    .bind(platform_id)
    .execute(&pool)
    .await
    .unwrap();
    fail_runtime.store(true, Ordering::Release);
    let failed_delete = request(
        &app,
        Method::DELETE,
        "/api/v1/deployments",
        Some(admin.clone()),
        Some(json!([failed_id])),
    )
    .await;
    assert_eq!(failed_delete.status(), StatusCode::CONFLICT);
    let state: (String, String) =
        sqlx::query_as("SELECT status,controlstate FROM deployments WHERE id=$1")
            .bind(failed_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(state, ("Created".to_owned(), "Idle".to_owned()));
    sqlx::query("DELETE FROM deployments WHERE id=$1")
        .bind(failed_id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("DELETE FROM deployments WHERE platformid=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(reader_user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(team_creator_user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE createdbyactorid=$1")
        .bind(team_creator_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(team_creator_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(reader_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE createdbyactorid=$1")
        .bind(actor_id)
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
