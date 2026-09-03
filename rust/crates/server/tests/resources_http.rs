use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use chrono::{Duration, Utc};
use citadel_adapters::crypto::{
    AesGcmSecretProtector, Argon2PasswordHasher, JwtSessionTokenCodec,
    OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_adapters::resource_metadata_store::PostgresResourceMetadataStore;
use citadel_database::MigrationRunner;
use citadel_domain::{ActorId, AuthenticatedPrincipalType, PermissionLevel, ResourceType};
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, IdentityService, NoopServiceAccountLastUsedTracker,
    SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_resources::ResourceMetadataService;
use citadel_server::resources_http::{self, ResourcesHttpState};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE5_DATABASE_URL"]
async fn metadata_endpoints_enforce_authorization_and_persist_complete_lifecycles() {
    let database_url = std::env::var("CITADEL_PHASE5_DATABASE_URL")
        .expect("CITADEL_PHASE5_DATABASE_URL is required for this fixture");
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
            JwtSessionTokenCodec::new(&[23_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let app = resources_http::router(ResourcesHttpState {
        identity,
        resources: Arc::new(ResourceMetadataService::new(
            Arc::new(PostgresResourceMetadataStore::new(pool.clone())),
            Arc::new(AesGcmSecretProtector::new(&[29_u8; 32]).unwrap()),
        )),
        realtime: None,
    });
    assert_eq!(
        request(&app, Method::GET, "/api/v1/tags", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let administrator_actor_id = Uuid::now_v7();
    let administrator_user_id = Uuid::now_v7();
    let administrator_name = format!("phase5-admin-{}", administrator_user_id.simple());
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(administrator_actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(administrator_user_id)
        .bind(administrator_actor_id)
        .bind(Utc::now())
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("{administrator_name}@example.test"))
        .bind(&administrator_name)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
        .bind(administrator_actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    let administrator = ActorPrincipal {
        subject_id: administrator_user_id,
        actor_id: ActorId::new(administrator_actor_id),
        name: administrator_name,
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".into()],
    };
    let suffix = Uuid::now_v7().simple().to_string();
    let malformed = request_raw(
        &app,
        Method::POST,
        "/api/v1/tags",
        Some(administrator.clone()),
        "{".into(),
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        malformed
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );

    let tag_response = request(
        &app,
        Method::POST,
        "/api/v1/tags",
        Some(administrator.clone()),
        Some(json!({"name":format!("http-{suffix}"),"color":"#334455"})),
    )
    .await;
    assert_eq!(tag_response.status(), StatusCode::OK);
    let tag = response_json(tag_response).await;
    let tag_id = tag["id"].as_str().unwrap();

    let registry_response = request(
        &app,
        Method::POST,
        "/api/v1/registries",
        Some(administrator.clone()),
        Some(json!({
            "name":format!("registry-{suffix}"),
            "registryHost":"registry.example.test",
            "status":"Active",
            "configuration":{"$type":"Custom","Username":"user","Password":"password"},
            "tagIds":[tag_id]
        })),
    )
    .await;
    assert_eq!(registry_response.status(), StatusCode::OK);
    let registry = response_json(registry_response).await;
    let registry_id = registry["id"].as_str().unwrap();
    assert!(registry.get("configuration").is_none());

    let repository_response = request(
        &app,
        Method::POST,
        "/api/v1/gitRepositories",
        Some(administrator.clone()),
        Some(json!({
            "name":format!("repository-{suffix}"),
            "url":"https://git.example.test/team/repository.git",
            "defaultBranch":"main",
            "syncMode":"Manual",
            "tagIds":[tag_id]
        })),
    )
    .await;
    assert_eq!(repository_response.status(), StatusCode::OK);
    let repository = response_json(repository_response).await;
    let repository_id = repository["id"].as_str().unwrap();

    let actor_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(&pool)
        .await
        .unwrap();
    for (resource_type, resource_id, permission) in [
        (
            ResourceType::Tag,
            Uuid::parse_str(tag_id).unwrap(),
            PermissionLevel::Read,
        ),
        (
            ResourceType::Registry,
            Uuid::parse_str(registry_id).unwrap(),
            PermissionLevel::Execute,
        ),
        (
            ResourceType::GitRepository,
            Uuid::parse_str(repository_id).unwrap(),
            PermissionLevel::Execute,
        ),
    ] {
        sqlx::query(
            "INSERT INTO resourceaccesses (id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES ($1,$2,$3,$4,$5,0)",
        )
        .bind(Uuid::now_v7())
        .bind(actor_id)
        .bind(permission as i32)
        .bind(resource_id)
        .bind(resource_type as i32)
        .execute(&pool)
        .await
        .unwrap();
    }
    let resource_actor = ActorPrincipal {
        subject_id: actor_id,
        actor_id: ActorId::new(actor_id),
        name: "resource operator".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: Vec::new(),
    };

    let listed_tags = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/tags",
            Some(resource_actor.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(listed_tags["tags"].as_array().unwrap().len(), 1);
    let listed_registries = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/registries?includeDisabled=true",
            Some(resource_actor.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(listed_registries["registries"].as_array().unwrap().len(), 1);
    assert_eq!(
        listed_registries["registries"][0]["capabilities"]["canExecute"],
        true
    );
    assert_eq!(
        request(
            &app,
            Method::PATCH,
            &format!("/api/v1/registries/{registry_id}"),
            Some(resource_actor.clone()),
            Some(json!({"description":"resource-scoped update"})),
        )
        .await
        .status(),
        StatusCode::OK
    );
    let repository_detail = response_json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/gitRepositories/{repository_id}"),
            Some(resource_actor.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        repository_detail["latestActivityView"]["eventType"],
        "GitRepoCreated"
    );

    let binding_response = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/global",
        Some(administrator.clone()),
        Some(json!({"name":"REGION","kind":"Variable","value":"eu-west"})),
    )
    .await;
    assert_eq!(binding_response.status(), StatusCode::OK);
    let binding = response_json(binding_response).await;
    assert_eq!(binding["entries"][0]["value"], "eu-west");

    let internal_secret_response = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/secrets",
        Some(administrator.clone()),
        Some(json!({"name":format!("INTERNAL_{suffix}").to_uppercase(),"value":"protected-value"})),
    )
    .await;
    assert_eq!(internal_secret_response.status(), StatusCode::OK);
    let internal_secret = response_json(internal_secret_response).await;
    let internal_secret_id = internal_secret["id"].as_str().unwrap();

    let provider_response = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/secret-providers/vault-kv2",
        Some(administrator.clone()),
        Some(json!({
            "name":format!("provider-{suffix}"),
            "address":"https://vault.example.test/",
            "mountPath":"/secret/",
            "token":"provider-token"
        })),
    )
    .await;
    assert_eq!(provider_response.status(), StatusCode::OK);
    let provider = response_json(provider_response).await;
    let provider_id = provider["id"].as_str().unwrap();
    assert_eq!(provider["address"], "https://vault.example.test");
    assert!(provider.get("token").is_none());

    let provider_patch = request(
        &app,
        Method::PATCH,
        &format!("/api/v1/resourceBindings/secret-providers/vault-kv2/{provider_id}"),
        Some(administrator.clone()),
        Some(json!({"name":format!("provider-updated-{suffix}")})),
    )
    .await;
    assert_eq!(provider_patch.status(), StatusCode::OK);
    let provider = response_json(provider_patch).await;
    assert_eq!(provider["mountPath"], "secret");

    let external_secret_response = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/secrets/external",
        Some(administrator.clone()),
        Some(json!({
            "name":format!("EXTERNAL_{suffix}").to_uppercase(),
            "providerId":provider_id,
            "externalPath":"services/citadel",
            "externalKey":"token",
            "externalVersion":3
        })),
    )
    .await;
    assert_eq!(external_secret_response.status(), StatusCode::OK);
    let external_secret = response_json(external_secret_response).await;
    let external_secret_id = external_secret["id"].as_str().unwrap();
    let external_secret_patch = request(
        &app,
        Method::PATCH,
        &format!("/api/v1/resourceBindings/secrets/external/{external_secret_id}"),
        Some(administrator.clone()),
        Some(json!({"externalKey":"rotated-token"})),
    )
    .await;
    assert_eq!(external_secret_patch.status(), StatusCode::OK);
    let external_secret = response_json(external_secret_patch).await;
    assert_eq!(external_secret["externalKey"], "rotated-token");
    assert_eq!(external_secret["externalVersion"], 3);

    let secret_binding_response = request(
        &app,
        Method::POST,
        "/api/v1/resourceBindings/global",
        Some(administrator.clone()),
        Some(json!({
            "name":"EXTERNAL_TOKEN",
            "kind":"Secret",
            "secretId":external_secret_id,
            "secretDeliveryMode":"EnvironmentVariable"
        })),
    )
    .await;
    assert_eq!(secret_binding_response.status(), StatusCode::OK);
    let secret_bindings = response_json(secret_binding_response).await;
    let secret_binding_id = secret_bindings["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["secretId"] == external_secret_id)
        .and_then(|entry| entry["id"].as_str())
        .unwrap()
        .to_owned();

    let secret_list = response_json(
        request(
            &app,
            Method::GET,
            "/api/v1/resourceBindings/secrets",
            Some(administrator.clone()),
            None,
        )
        .await,
    )
    .await;
    assert!(
        secret_list["secrets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|secret| secret["id"] == external_secret_id)
    );

    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/resourceBindings/global/{secret_binding_id}"),
            Some(administrator.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::OK
    );

    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/resourceBindings/secrets/{external_secret_id}"),
            Some(administrator.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::NOT_FOUND,
        "deleting the last binding must delete its orphaned Secret definition"
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/resourceBindings/secrets/{internal_secret_id}"),
            Some(administrator.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/resourceBindings/secret-providers/{provider_id}"),
            Some(administrator.clone()),
            None,
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );

    assert_eq!(
        request(
            &app,
            Method::DELETE,
            "/api/v1/registries",
            Some(resource_actor.clone()),
            Some(json!({"ids":[registry_id]})),
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            "/api/v1/gitRepositories",
            Some(resource_actor),
            Some(json!({"ids":[repository_id]})),
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v1/tags/{tag_id}"),
            Some(administrator),
            None,
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
}

async fn request_raw(
    app: &Router,
    method: Method,
    uri: &str,
    principal: Option<ActorPrincipal>,
    body: Vec<u8>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    app.clone().oneshot(request).await.unwrap()
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
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
