use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use citadel_adapters::{
    crypto::{
        AesGcmSecretProtector, Argon2PasswordHasher, JwtSessionTokenCodec,
        OpaqueServiceAccountTokenCodec,
    },
    identity_store::{PostgresIdentityStore, StaticEntitlementService},
    resource_metadata_store::PostgresResourceMetadataStore,
    secret_value_resolver::PostgresSecretValueResolver,
};
use citadel_domain::{ActorId, AuthenticatedPrincipalType};
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, IdentityService, NoopServiceAccountLastUsedTracker,
    SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_resources::ResourceMetadataService;
use citadel_server::resources_http::{self, ResourcesHttpState};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

async fn request(
    app: &Router,
    actor: &ActorPrincipal,
    method: Method,
    path: &str,
    input: Value,
) -> Value {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(input.to_string()))
        .unwrap();
    request.extensions_mut().insert(actor.clone());
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    for secret in [super::TOKEN, super::VALUE] {
        assert!(!body.to_string().contains(secret));
    }
    body
}

/// Ports VaultKvV2_ShouldValidateResolveInjectAndRedactSecret through real HTTP
/// handlers, PostgreSQL and Vault; the enclosing fixture proves runtime injection.
pub async fn verify(pool: &PgPool, protector: Arc<AesGcmSecretProtector>, address: &str) -> Uuid {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdat,createdbyactorid,email,name) VALUES($1,$2,now(),$3,$4,$4)")
        .bind(user_id).bind(actor_id).bind(SYSTEM_ACTOR_ID).bind(format!("vault-{user_id}@example.test")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor_id)
        .bind(ADMIN_ROLE_ID)
        .execute(pool)
        .await
        .unwrap();
    let actor = ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name: "Vault acceptance".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".into()],
    };
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(JwtSessionTokenCodec::new(&[23; 32], "fixture".into(), "fixture".into()).unwrap()),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        chrono::Duration::minutes(15),
        chrono::Duration::days(30),
    ));
    let resources = Arc::new(
        ResourceMetadataService::new(
            Arc::new(PostgresResourceMetadataStore::new(pool.clone())),
            protector.clone(),
        )
        .with_secret_provider_tester(Arc::new(
            PostgresSecretValueResolver::new(pool.clone(), protector).unwrap(),
        )),
    );
    let app = resources_http::router(ResourcesHttpState {
        identity,
        resources,
        realtime: None,
    });
    let connection = "/api/v1/resourceBindings/secret-providers/vault-kv2/test";
    for (token, success) in [(super::TOKEN, true), ("invalid-token", false)] {
        let result = request(
            &app,
            &actor,
            Method::POST,
            connection,
            json!({"address":address,"mountPath":"secret","token":token}),
        )
        .await;
        assert_eq!(result["success"], success);
    }
    let created = request(&app,&actor,Method::POST,"/api/v1/resourceBindings/secret-providers/vault-kv2",json!({"name":format!("vault-{user_id}"),"address":address,"mountPath":"secret","token":super::TOKEN})).await;
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    for (path, key, version, success) in [
        ("citadel/acceptance", "api_key", 1, true),
        ("missing", "api_key", 1, false),
        ("citadel/acceptance", "missing", 1, false),
        ("citadel/acceptance", "api_key", 99, false),
    ] {
        let result = request(&app,&actor,Method::POST,"/api/v1/resourceBindings/secrets/external/test",json!({"providerId":id,"externalPath":path,"externalKey":key,"externalVersion":version})).await;
        assert_eq!(result["success"], success);
    }
    let before: String =
        sqlx::query_scalar("SELECT configuration FROM secretproviders WHERE id=$1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
    request(
        &app,
        &actor,
        Method::PATCH,
        &format!("/api/v1/resourceBindings/secret-providers/vault-kv2/{id}"),
        json!({"token":""}),
    )
    .await;
    let after: String = sqlx::query_scalar("SELECT configuration FROM secretproviders WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    let stored = request(
        &app,
        &actor,
        Method::POST,
        connection,
        json!({"providerId":id,"address":address,"mountPath":"secret"}),
    )
    .await;
    assert_eq!(stored["success"], true);
    assert!(stored["message"].as_str().unwrap().contains("stored token"));
    let replaced = request(
        &app,
        &actor,
        Method::POST,
        connection,
        json!({"providerId":id,"address":address,"mountPath":"secret","token":"invalid-token"}),
    )
    .await;
    assert_eq!(replaced["success"], false);
    assert!(!after.contains(super::TOKEN));
    id
}
