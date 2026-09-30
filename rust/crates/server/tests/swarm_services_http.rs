#[path = "swarm_services_http/contracts.rs"]
mod contracts;
use citadel_server::api::routes::swarm_services as swarm_services_http;
use std::sync::Arc;

#[path = "swarm_services_http/metadata.rs"]
mod metadata;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use chrono::Duration;
use citadel_adapters::{
    persistence::postgres::{
        identity::authentication::store::{PostgresIdentityStore, StaticEntitlementService},
        swarm_services::PostgresSwarmServiceRepository,
    },
    security::identity::crypto::{
        Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
    },
};
use citadel_database::MigrationRunner;
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, AuthenticatedPrincipalType, IdentityService,
    NoopServiceAccountLastUsedTracker, SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_primitives::ActorId;
use citadel_server::api::routes::swarm_services::SwarmServicesHttpState;
use citadel_swarm_services::{
    RuntimeServiceResult, ServiceOperationClaim, SwarmServiceError, SwarmServiceRuntime,
    SwarmServiceService,
};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "swarm_services_http/operations.rs"]
mod operations;
#[path = "swarm_services_http/updates.rs"]
mod updates;
#[path = "swarm_services_http/webhooks.rs"]
mod webhooks;

struct CompletingRuntime;
impl SwarmServiceRuntime for CompletingRuntime {
    fn apply<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>> {
        Box::pin(async move { Ok(completed(claim)) })
    }
    fn scale<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        _replicas: i32,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>> {
        Box::pin(async move { Ok(completed(claim)) })
    }
    fn force_update<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>> {
        Box::pin(async move { Ok(completed(claim)) })
    }
    fn delete<'a>(
        &'a self,
        _platform_id: Uuid,
        _docker_service_id: &'a str,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async { Ok(()) })
    }
    fn observe<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<RuntimeServiceResult>, SwarmServiceError>> {
        Box::pin(async move { Ok(Some(completed(claim))) })
    }
}

fn completed(claim: &ServiceOperationClaim) -> RuntimeServiceResult {
    RuntimeServiceResult {
        docker_service_id: claim
            .docker_service_id
            .clone()
            .unwrap_or_else(|| "docker-service".to_owned()),
        version_index: 1,
        accepted: true,
        rollout_complete: true,
        rollout_error: None,
        runtime_hash: claim.desired_hash.clone(),
        applied_digest: Some("sha256:fixture".to_owned()),
        warnings: Vec::new(),
    }
}

#[path = "swarm_services_http/task_ownership.rs"]
mod task_ownership;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn managed_swarm_service_endpoints_enforce_auth_and_persist_lifecycle() {
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
            JwtSessionTokenCodec::new(&[71_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let digests = Arc::new(updates::DigestFixture::default());
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
        .with_image_digests(digests.clone()),
    );
    let app = swarm_services_http::router(SwarmServicesHttpState {
        identity: identity.clone(),
        services: services.clone(),
    });
    assert_eq!(
        request(&app, Method::GET, "/api/v1/swarmServices", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let platform_id = Uuid::now_v7();
    let registry_id = Uuid::now_v7();
    let suffix = user_id.simple().to_string();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
        .bind(actor_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdat,createdbyactorid,email,name) VALUES($1,$2,CURRENT_TIMESTAMP,$3,$4,$5)")
        .bind(user_id).bind(actor_id).bind(SYSTEM_ACTOR_ID)
        .bind(format!("swarm-{suffix}@example.test")).bind(format!("swarm-{suffix}"))
        .execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(r#"INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
        VALUES($1,$2,'Local',1,0,1,$2,0,'{"$type":"DockerSwarm","clusterId":"fixture","controlAvailable":true}'::json,'Online',0)"#)
        .bind(platform_id).bind(format!("swarm-{suffix}")).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{}'::json,$2,$3,'docker.io','Enabled')")
        .bind(registry_id).bind(SYSTEM_ACTOR_ID).bind(format!("registry-{suffix}"))
        .execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let admin = ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name: format!("swarm-{suffix}"),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".to_owned()],
    };
    let spec = json!({
        "image":{"$type":"External","registryId":registry_id,"imageTag":"redis:7-alpine"},
        "updateBehavior":"Disabled","schedulingMode":"Replicated","replicas":1
    });
    let created_response = request(
        &app,
        Method::POST,
        "/api/v1/swarmServices",
        Some(admin.clone()),
        Some(json!({"name":format!("redis-{suffix}"),"platformId":platform_id,"spec":spec})),
    )
    .await;
    let status = created_response.status();
    let created = response_json(created_response).await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    contracts::verify(&app, &admin, id).await;
    let mut invalid_spec = created["spec"].clone();
    invalid_spec["replicas"] = json!(-1);
    for (method, path, body, explanation) in [
        (
            Method::PATCH,
            format!("/api/v1/swarmServices/{id}"),
            json!({"rowVersion":"bad"}),
            "rowVersion",
        ),
        (
            Method::PATCH,
            format!("/api/v1/swarmServices/{id}"),
            json!({"spec":invalid_spec,"rowVersion":created["rowVersion"]}),
            "replica",
        ),
    ] {
        let response = request(&app, method, &path, Some(admin.clone()), Some(body)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response_json(response).await;
        assert!(
            body["errors"]
                .to_string()
                .to_lowercase()
                .contains(&explanation.to_lowercase()),
            "{body}"
        );
    }
    metadata::verify(&app, &pool, &admin, id).await;
    assert_eq!(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/swarmServices/{id}/check-updates"),
            Some(admin.clone()),
            None
        )
        .await
        .status(),
        StatusCode::CONFLICT,
        "a saved image is not an applied baseline"
    );

    let apply = request(
        &app,
        Method::POST,
        &format!("/api/v1/swarmServices/{id}/apply"),
        Some(admin.clone()),
        None,
    )
    .await;
    assert_eq!(apply.status(), StatusCode::OK);
    let progress = response_json(apply).await;
    assert_eq!(
        progress.as_array().unwrap().last().unwrap()["isCompleted"],
        true
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT health FROM swarmservices WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Healthy"
    );
    assert!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM activityevents WHERE resourceid=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap()
            >= 2
    );

    operations::verify(&app, &pool, &admin, id).await;
    updates::exercise_update_checks(&app, &services, &pool, &admin, id, &digests).await;
    webhooks::verify(&pool, identity, &admin, id).await;

    task_ownership::verify(&pool, admin.actor_id, id).await;
    let deleted = request(
        &app,
        Method::DELETE,
        "/api/v1/swarmServices",
        Some(admin),
        Some(json!([id])),
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

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
    sqlx::query("DELETE FROM registries WHERE id=$1")
        .bind(registry_id)
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
