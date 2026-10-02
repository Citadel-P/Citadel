//! Native registry contracts exercised against Axum and disposable PostgreSQL.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use citadel_adapters::{
    persistence::postgres::{
        identity::authentication::store::{PostgresIdentityStore, StaticEntitlementService},
        registries::PostgresRegistryRepository,
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
use citadel_server::api::routes::registries::{RegistriesHttpState, router};
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires disposable CITADEL_REGISTRIES_DATABASE_URL"]
async fn native_registry_contracts_preserve_credentials_permissions_and_default_protection() {
    let url = std::env::var("CITADEL_REGISTRIES_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let db = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(db.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(JwtSessionTokenCodec::new(&[93; 32], "fixture".into(), "fixture".into()).unwrap()),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        chrono::Duration::minutes(15),
        chrono::Duration::days(30),
    ));

    let app = router(RegistriesHttpState {
        identity,
        registries: Arc::new(PostgresRegistryRepository::new(db.clone())),
        registry_connections: Arc::new(
            citadel_adapters::connectors::registries::browser::RegistryBrowser::with_endpoints(
                "http://127.0.0.1:1",
                "http://127.0.0.1:1",
            )
            .unwrap(),
        ),
        realtime: None,
    });
    let admin = actor(&db, true).await;
    let denied = actor(&db, false).await;
    let response = request(&app, Method::POST, "/api/v1/registries", Some(admin.clone()), Some(json!({
        "name":"native-custom", "registryHost":"registry.example.test", "status":"Active",
        "configuration":{"$type":"Custom","authEnabled":true,"userName":"operator","password":"original-secret"}
    }))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let registry = response_json(response).await;
    assert_eq!(registry["type"], "Custom");
    assert!(registry.get("configuration").is_none());
    let id = registry["id"].as_str().unwrap();
    let path = format!("/api/v1/registries/{id}");
    let cfg_path = format!("{path}/_cfg");
    for (principal, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(denied.clone()), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            request(&app, Method::GET, &cfg_path, principal.clone(), None)
                .await
                .status(),
            expected
        );
        assert_eq!(
            request(
                &app,
                Method::PATCH,
                &path,
                principal,
                Some(json!({"configuration":{"password":"blocked-secret"}}))
            )
            .await
            .status(),
            expected
        );
    }
    let reader = actor(&db, false).await;
    sqlx::query("INSERT INTO resourceaccesses (id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES ($1,$2,$3,$4,$5,0)")
        .bind(Uuid::now_v7()).bind(reader.actor_id.value())
        .bind(citadel_primitives::PermissionLevel::Read as i32)
        .bind(Uuid::parse_str(id).unwrap()).bind(citadel_primitives::ResourceType::Registry as i32)
        .execute(&db).await.unwrap();
    let read_response = request(&app, Method::GET, &cfg_path, Some(reader.clone()), None).await;
    assert_eq!(read_response.status(), StatusCode::OK);
    let public = response_json(read_response).await;
    assert!(public["configuration"].get("password").is_none());
    assert_eq!(public["configuration"]["hasPassword"], true);
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &path,
            Some(reader),
            Some(json!({"configuration":{"password":"blocked-secret"}}))
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let cfg =
        response_json(request(&app, Method::GET, &cfg_path, Some(admin.clone()), None).await).await;
    assert!(cfg["configuration"].get("password").is_none());
    assert_eq!(cfg["configuration"]["hasPassword"], true);
    // Editing public settings must preserve the server-side credential.
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"configuration":{"userName":"operator"}}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    let stored: Value = sqlx::query_scalar("SELECT configuration FROM registries WHERE id=$1")
        .bind(Uuid::parse_str(id).unwrap())
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(stored["password"], "original-secret");
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"configuration":{"password":"rotated-secret"}}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    let cfg =
        response_json(request(&app, Method::GET, &cfg_path, Some(admin.clone()), None).await).await;
    assert_eq!(
        cfg["configuration"],
        json!({"$type":"Custom","authEnabled":true,"userName":"operator","hasPassword":true})
    );
    let stored: Value = sqlx::query_scalar("SELECT configuration FROM registries WHERE id=$1")
        .bind(Uuid::parse_str(id).unwrap())
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(stored["password"], "rotated-secret");
    for invalid in [
        json!({"configuration":{"authEnabled":"yes"}}),
        json!({"configuration":{"$type":"Unknown"}}),
        json!({"configuration":null}),
        json!({"configuration":{"password":null}}),
    ] {
        assert_eq!(
            request(
                &app,
                Method::PATCH,
                &path,
                Some(admin.clone()),
                Some(invalid)
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    // Explicit null removes optional credentials once authentication is disabled.
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"configuration":{"authEnabled":false,"password":null}}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"status":"Deprecated"}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    let cfg =
        response_json(request(&app, Method::GET, &cfg_path, Some(admin.clone()), None).await).await;
    assert_eq!(cfg["status"], "Deprecated");
    assert_eq!(cfg["configuration"]["hasPassword"], false);
    assert_eq!(cfg["configuration"]["userName"], "operator");
    assert!(cfg["configuration"].get("password").is_none());
    let list = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/registries",
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert!(
        list["registries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == id)
    );
    assert!(
        list["registries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r.get("configuration").is_none())
    );
    let default_id = Uuid::from_u128(0x100).to_string();
    let default = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/registries/{default_id}/_cfg"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(default["configuration"]["$type"], "DockerHub");
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/registries/{default_id}"),
            Some(admin.clone()),
            Some(json!({"status":"Disabled"}))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            "/api/v1/registries",
            Some(admin.clone()),
            Some(json!({"ids":[default_id]}))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            "/api/v1/registries",
            Some(admin.clone()),
            Some(json!({"ids":[id]}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, Method::GET, &path, Some(admin), None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    db.close().await;
}
async fn actor(db: &PgPool, admin: bool) -> ActorPrincipal {
    let id = Uuid::now_v7();
    let user = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(id)
        .execute(db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,email,name) VALUES($1,$2,$3,$4,'Registry HTTP')").bind(user).bind(id).bind(SYSTEM_ACTOR_ID).bind(format!("{user}@example.test")).execute(db).await.unwrap();
    if admin {
        sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
            .bind(id)
            .bind(ADMIN_ROLE_ID)
            .execute(db)
            .await
            .unwrap();
    }
    ActorPrincipal {
        subject_id: user,
        actor_id: ActorId::new(id),
        name: "Registry HTTP".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: if admin { vec!["Admin".into()] } else { vec![] },
    }
}
async fn request(
    app: &Router,
    method: Method,
    path: &str,
    actor: Option<ActorPrincipal>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut request = Request::builder().method(method).uri(path);
    if body.is_some() {
        request = request.header("content-type", "application/json");
    }
    if let Some(actor) = actor {
        request = request.extension(actor);
    }
    app.clone()
        .oneshot(
            request
                .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
                .unwrap(),
        )
        .await
        .unwrap()
}
async fn response_json(response: axum::response::Response) -> Value {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("{status}: {error}: {}", String::from_utf8_lossy(&bytes)))
}
