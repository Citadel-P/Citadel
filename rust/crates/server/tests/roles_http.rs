use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, Response, StatusCode},
};
use chrono::{Duration, Utc};
use citadel_adapters::{
    persistence::postgres::identity::{
        authentication::store::{PostgresIdentityStore, StaticEntitlementService},
        roles::repository::PostgresRoleRepository,
    },
    security::identity::crypto::{
        Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
    },
};
use citadel_database::MigrationRunner;
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, AuthenticatedPrincipalType, IdentityService,
    NoopServiceAccountLastUsedTracker, RoleMutationService, RoleReadService, SYSTEM_ACTOR_ID,
    SystemClock,
};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType};
use citadel_server::api::{
    resources::roles::requests::{CreateRoleRequest, RolePermissionInput},
    routes::{roles as roles_http, roles::RolesHttpState},
};
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex, OwnedMutexGuard};
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    app: Router,
    pool: PgPool,
    administrator: ActorPrincipal,
    mutations: Arc<RoleMutationService>,
    _guard: OwnedMutexGuard<()>,
}

static TEST_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn list_and_get_return_persisted_roles_permissions_and_capabilities() {
    let fixture = fixture(true).await;
    let marker = Uuid::now_v7().simple().to_string();
    let role_id = seed_role(&fixture.pool, &format!("reader-{marker}"), "Custom").await;
    seed_permission(
        &fixture.pool,
        role_id,
        ResourceType::Deployment,
        PermissionLevel::Read,
        1,
    )
    .await;

    let list = send(&fixture, Method::GET, "/api/v1/roles", None).await;
    assert_eq!(list.status(), StatusCode::OK);
    let list = json(list).await;
    assert_eq!(list["capabilities"]["canExecute"], true);
    let projected = list["roles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|role| role["id"] == role_id.to_string())
        .unwrap();
    assert_eq!(projected["roleType"], "Custom");
    assert_eq!(projected["permissions"][0]["resourceType"], "Deployment");
    assert_eq!(
        projected["permissions"][0]["specificPermissions"][0],
        "Logs"
    );

    let get = send(
        &fixture,
        Method::GET,
        &format!("/api/v1/roles/{role_id}"),
        None,
    )
    .await;
    assert_eq!(get.status(), StatusCode::OK);
    assert_eq!(json(get).await["id"], role_id.to_string());
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn create_is_transactional_and_writes_a_safe_activity() {
    let fixture = fixture(true).await;
    let name = format!("operator-{}", Uuid::now_v7().simple());
    let response = send_json(
        &fixture,
        Method::POST,
        "/api/v1/roles",
        serde_json::json!({
            "name": name,
            "permissions": [{
                "resourceType": "Deployment", "permissionLevel": "Read",
                "specificPermissions": ["Logs", "Logs"]
            }, {
                "resourceType": "Registry", "permissionLevel": "Write",
                "specificPermissions": []
            }]
        }),
        "application/json",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    let role_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    assert_eq!(body["permissions"].as_array().unwrap().len(), 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM permissions WHERE roleid = $1")
            .bind(role_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        2
    );
    let info: String = sqlx::query_scalar(
        "SELECT info FROM activityevents WHERE resourceid = $1 AND eventtype = 'RoleCreated'",
    )
    .bind(role_id)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert!(info.contains("\"$type\":\"RoleCreated\""));
    assert!(info.contains("\"RoleType\":\"Custom\""));
    assert!(!info.contains("password"));
    assert!(!info.contains("token"));

    let invalid_name = format!("invalid-{}", Uuid::now_v7().simple());
    let invalid = send_json(
        &fixture,
        Method::POST,
        "/api/v1/roles",
        serde_json::json!({
            "name": invalid_name,
            "permissions": [{
                "resourceType": "Role", "permissionLevel": "Read",
                "specificPermissions": ["Logs"]
            }]
        }),
        "application/json",
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM roles WHERE name = $1")
            .bind(invalid_name)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        0
    );

    let malformed = send_json(
        &fixture,
        Method::POST,
        "/api/v1/roles",
        serde_json::json!({
            "name": format!("malformed-{}", Uuid::now_v7().simple()),
            "permissions": [{
                "resourceType": "Registry", "permissionLevel": "Nope",
                "specificPermissions": []
            }]
        }),
        "application/json",
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

    let nullable_specifics = send_json(
        &fixture,
        Method::POST,
        "/api/v1/roles",
        serde_json::json!({
            "name": format!("nullable-specifics-{}", Uuid::now_v7().simple()),
            "permissions": [{
                "resourceType": "Registry", "permissionLevel": "Read",
                "specificPermissions": null
            }]
        }),
        "application/json",
    )
    .await;
    assert_eq!(nullable_specifics.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn patch_replaces_permissions_and_license_downgrade_allows_only_reduction() {
    let licensed = fixture(true).await;
    let role_id = seed_role(
        &licensed.pool,
        &format!("patch-{}", Uuid::now_v7().simple()),
        "Custom",
    )
    .await;
    seed_permission(
        &licensed.pool,
        role_id,
        ResourceType::Registry,
        PermissionLevel::Execute,
        0,
    )
    .await;

    let patch = send_json(
        &licensed,
        Method::PATCH,
        &format!("/api/v1/roles/{role_id}/permissions"),
        serde_json::json!({ "permissions": [{
            "resourceType": "Registry", "permissionLevel": "Write",
            "specificPermissions": []
        }, {
            "resourceType": "Role", "permissionLevel": "Read",
            "specificPermissions": []
        }] }),
        "application/merge-patch+json",
    )
    .await;
    assert_eq!(patch.status(), StatusCode::OK);
    assert_eq!(
        activity_count(&licensed.pool, role_id, "RoleUpdated").await,
        1
    );
    let null_patch = send_json(
        &licensed,
        Method::PATCH,
        &format!("/api/v1/roles/{role_id}/permissions"),
        serde_json::json!({ "permissions": null }),
        "application/merge-patch+json",
    )
    .await;
    assert_eq!(null_patch.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        activity_count(&licensed.pool, role_id, "RoleUpdated").await,
        1
    );
    drop(licensed);

    let unlicensed = fixture(false).await;
    let reduction = send_json(
        &unlicensed,
        Method::PATCH,
        &format!("/api/v1/roles/{role_id}/permissions"),
        serde_json::json!({ "permissions": [{
            "resourceType": "Registry", "permissionLevel": "Read",
            "specificPermissions": []
        }] }),
        "application/merge-patch+json",
    )
    .await;
    assert_eq!(reduction.status(), StatusCode::OK);

    let expansion = send_json(
        &unlicensed,
        Method::PATCH,
        &format!("/api/v1/roles/{role_id}/permissions"),
        serde_json::json!({ "permissions": [{
            "resourceType": "Registry", "permissionLevel": "Execute",
            "specificPermissions": []
        }] }),
        "application/merge-patch+json",
    )
    .await;
    assert_eq!(expansion.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        sqlx::query_scalar::<_, i32>(
            "SELECT permissionlevel FROM permissions WHERE roleid = $1 AND resourcetype = $2"
        )
        .bind(role_id)
        .bind(ResourceType::Registry as i32)
        .fetch_one(&unlicensed.pool)
        .await
        .unwrap(),
        PermissionLevel::Read as i32
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn system_roles_are_immutable_but_custom_roles_can_be_renamed_and_deleted() {
    let fixture = fixture(true).await;
    let system_id = seed_role(
        &fixture.pool,
        &format!("system-{}", Uuid::now_v7().simple()),
        "System",
    )
    .await;
    let custom_id = seed_role(
        &fixture.pool,
        &format!("custom-{}", Uuid::now_v7().simple()),
        "Custom",
    )
    .await;

    let system_patch = send_json(
        &fixture,
        Method::PATCH,
        &format!("/api/v1/roles/{system_id}/permissions"),
        serde_json::json!({ "permissions": [] }),
        "application/merge-patch+json",
    )
    .await;
    assert_eq!(system_patch.status(), StatusCode::CONFLICT);
    let system_rename = send_json(
        &fixture,
        Method::POST,
        "/api/v1/roles/rename",
        serde_json::json!({ "id": system_id, "name": "not-allowed" }),
        "application/json",
    )
    .await;
    assert_eq!(system_rename.status(), StatusCode::CONFLICT);
    let system_delete = send_json(
        &fixture,
        Method::DELETE,
        "/api/v1/roles",
        serde_json::json!({ "ids": [system_id] }),
        "application/json",
    )
    .await;
    assert_eq!(system_delete.status(), StatusCode::CONFLICT);

    let renamed = format!("renamed-{}", Uuid::now_v7().simple());
    let rename = send_json(
        &fixture,
        Method::POST,
        "/api/v1/roles/rename",
        serde_json::json!({ "id": custom_id, "name": renamed }),
        "application/json",
    )
    .await;
    assert_eq!(rename.status(), StatusCode::OK);
    assert_eq!(
        activity_count(&fixture.pool, custom_id, "RoleRenamed").await,
        1
    );
    let delete = send_json(
        &fixture,
        Method::DELETE,
        "/api/v1/roles",
        serde_json::json!({ "ids": [custom_id] }),
        "application/json",
    )
    .await;
    assert_eq!(delete.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        activity_count(&fixture.pool, custom_id, "RoleDeleted").await,
        1
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn license_authorization_and_request_bounds_are_enforced() {
    let fixture = fixture(false).await;
    let denied = send_json(
        &fixture,
        Method::POST,
        "/api/v1/roles",
        serde_json::json!({
            "name": format!("unlicensed-{}", Uuid::now_v7().simple()),
            "permissions": []
        }),
        "application/json",
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let unauthenticated = send_without_principal(&fixture, Method::GET, "/api/v1/roles").await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let regular = ActorPrincipal {
        subject_id: Uuid::now_v7(),
        actor_id: ActorId::new(Uuid::now_v7()),
        name: "regular".to_owned(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: Vec::new(),
    };
    let forbidden = send_as(&fixture, regular, Method::GET, "/api/v1/roles", None).await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    let empty_delete = send_json(
        &fixture,
        Method::DELETE,
        "/api/v1/roles",
        serde_json::json!({ "ids": [] }),
        "application/json",
    )
    .await;
    assert_eq!(empty_delete.status(), StatusCode::BAD_REQUEST);
    let missing = send(
        &fixture,
        Method::GET,
        &format!("/api/v1/roles/{}", Uuid::now_v7()),
        None,
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn permission_matrix_is_anonymous_keyed_and_matches_known_capabilities() {
    let fixture = fixture(false).await;
    let response =
        send_without_principal(&fixture, Method::GET, "/api/v1/roles/permissions/matrix").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert!(body.is_object());
    assert_eq!(body["Deployment"]["maximumLevel"], "Execute");
    assert_eq!(body["Deployment"]["specificPermissions"]["Apply"], "Read");
    assert_eq!(body["ServiceAccount"]["label"], "ServiceAccount");
    assert!(
        body["User"]["specificPermissions"]
            .as_object()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn concurrent_same_name_creates_are_serialized() {
    let fixture = fixture(true).await;
    let name = format!("concurrent-{}", Uuid::now_v7().simple());
    let request = CreateRoleRequest {
        name: name.clone(),
        permissions: Some(vec![RolePermissionInput {
            resource_type: ResourceType::Registry,
            permission_level: PermissionLevel::Read,
            specific_permissions: None,
        }]),
    };
    let first = fixture.mutations.clone();
    let second = fixture.mutations.clone();
    let actor_id = fixture.administrator.actor_id;
    let (first_result, second_result) = tokio::join!(
        first.create(request.clone().into(), actor_id),
        second.create(request.into(), actor_id)
    );
    assert_eq!(
        usize::from(first_result.is_ok()) + usize::from(second_result.is_ok()),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM roles WHERE name = $1")
            .bind(name)
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        1
    );
}

async fn fixture(custom_access: bool) -> Fixture {
    let guard = TEST_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone()
        .lock_owned()
        .await;
    let database_url = std::env::var("CITADEL_PHASE3_DATABASE_URL")
        .expect("CITADEL_PHASE3_DATABASE_URL is required");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let administrator = seed_administrator(&pool).await;
    let identity_store = Arc::new(PostgresIdentityStore::new(pool.clone()));
    let entitlements = Arc::new(StaticEntitlementService::new(custom_access));
    let clock = Arc::new(SystemClock);
    let identity = Arc::new(IdentityService::new(
        identity_store,
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(
                &[12_u8; 32],
                "roles-http".to_owned(),
                "roles-http".to_owned(),
            )
            .unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        entitlements.clone(),
        clock.clone(),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let store = Arc::new(PostgresRoleRepository::new(pool.clone()));
    let roles = Arc::new(RoleReadService::new(store.clone()));
    let mutations = Arc::new(RoleMutationService::new(store, entitlements, clock));
    let app = roles_http::router(RolesHttpState {
        identity,
        roles,
        mutations: mutations.clone(),
    });
    Fixture {
        app,
        pool,
        administrator,
        mutations,
        _guard: guard,
    }
}

async fn seed_administrator(pool: &PgPool) -> ActorPrincipal {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let name = format!("role-admin-{}", user_id.simple());
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(user_id)
        .bind(actor_id)
        .bind(Utc::now())
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("{name}@example.test"))
        .bind(&name)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name,
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".to_owned()],
    }
}

async fn seed_role(pool: &PgPool, name: &str, role_type: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO roles (id, name, roletype) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(name)
        .bind(role_type)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn seed_permission(
    pool: &PgPool,
    role_id: Uuid,
    resource_type: ResourceType,
    level: PermissionLevel,
    specifics: i32,
) {
    sqlx::query("INSERT INTO permissions (id, roleid, resourcetype, permissionlevel, specificpermissions) VALUES ($1, $2, $3, $4, $5)")
        .bind(Uuid::now_v7())
        .bind(role_id)
        .bind(resource_type as i32)
        .bind(level as i32)
        .bind(specifics)
        .execute(pool)
        .await
        .unwrap();
}

async fn activity_count(pool: &PgPool, id: Uuid, event: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM activityevents WHERE resourceid = $1 AND eventtype = $2",
    )
    .bind(id)
    .bind(event)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn send(fixture: &Fixture, method: Method, uri: &str, body: Option<Value>) -> Response<Body> {
    send_as(fixture, fixture.administrator.clone(), method, uri, body).await
}

async fn send_as(
    fixture: &Fixture,
    principal: ActorPrincipal,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> Response<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let mut request = builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap();
    request.extensions_mut().insert(principal);
    fixture.app.clone().oneshot(request).await.unwrap()
}

async fn send_json(
    fixture: &Fixture,
    method: Method,
    uri: &str,
    body: Value,
    content_type: &str,
) -> Response<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", content_type)
        .body(Body::from(body.to_string()))
        .unwrap();
    request
        .extensions_mut()
        .insert(fixture.administrator.clone());
    fixture.app.clone().oneshot(request).await.unwrap()
}

async fn send_without_principal(fixture: &Fixture, method: Method, uri: &str) -> Response<Body> {
    fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn json(response: Response<Body>) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
