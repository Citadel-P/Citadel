use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use citadel_adapters::{
    persistence::postgres::{
        bindings::{PostgresBindingRepository, secret_resolver::PostgresSecretValueResolver},
        identity::authentication::store::{PostgresIdentityStore, StaticEntitlementService},
    },
    security::identity::crypto::{
        AesGcmSecretProtector, Argon2PasswordHasher, JwtSessionTokenCodec,
        OpaqueServiceAccountTokenCodec,
    },
};
use citadel_bindings::SecretService;
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, AuthenticatedPrincipalType, IdentityService, Login,
    NoopServiceAccountLastUsedTracker, PasswordHasher, SYSTEM_ACTOR_ID, SessionMetadata,
    SystemClock,
};
use citadel_primitives::ActorId;
use citadel_server::api::routes::{bindings, git_repositories as git_catalog, registries, tags};
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
pub struct VerifiedProvider {
    pub id: Uuid,
    pub identity: Arc<IdentityService>,
    pub bearer: String,
}

pub async fn verify(
    pool: &PgPool,
    protector: Arc<AesGcmSecretProtector>,
    address: &str,
) -> VerifiedProvider {
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
    let password = Argon2PasswordHasher::default()
        .hash(super::PASSWORD)
        .unwrap();
    sqlx::query("UPDATE users SET password=$1 WHERE id=$2")
        .bind(password)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
    let (_, session) = identity
        .login(
            Login {
                email_or_name: format!("vault-{user_id}@example.test"),
                password: super::PASSWORD.into(),
            },
            SessionMetadata {
                user_agent: None,
                ip_address: None,
            },
        )
        .await
        .unwrap();
    let resources = Arc::new(
        SecretService::new(
            Arc::new(PostgresBindingRepository::new(pool.clone())),
            protector.clone(),
        )
        .with_secret_provider_tester(Arc::new(
            PostgresSecretValueResolver::new(pool.clone(), protector).unwrap(),
        )),
    );
    let app = tags::router(tags::TagsHttpState {
        identity: identity.clone(),
        tags: Arc::new(citadel_adapters::persistence::postgres::tags::PostgresTagRepository::new(pool.clone())),
        realtime: None,
    })
    .merge(registries::router(registries::RegistriesHttpState {
        registry_connections: Arc::new(citadel_adapters::connectors::registries::browser::RegistryBrowser::with_endpoints("http://127.0.0.1:1", "http://127.0.0.1:1").unwrap()),
        identity: identity.clone(),
        registries: Arc::new(
            citadel_adapters::persistence::postgres::registries::PostgresRegistryRepository::new(pool.clone()),
        ),
        realtime: None,
    }))
    .merge(bindings::router(bindings::BindingsHttpState {
        identity: identity.clone(),
        secrets: resources,
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
    }));
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
    VerifiedProvider {
        id,
        identity,
        bearer: session.access_token,
    }
}
