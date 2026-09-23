use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{
        Method, Request, Response, StatusCode,
        header::{CONTENT_TYPE, LOCATION, SET_COOKIE},
    },
};
use chrono::{Duration, Utc};
use citadel_adapters::{
    persistence::postgres::identity::{
        authentication::store::{PostgresIdentityStore, StaticEntitlementService},
        mfa::store::PostgresMfaStore,
        oidc::store::PostgresOidcStore,
    },
    security::identity::{
        crypto::{
            AesGcmSecretProtector, Argon2PasswordHasher, JwtSessionTokenCodec,
            OpaqueServiceAccountTokenCodec,
        },
        mfa::{HmacRecoveryCodeService, Sha1TotpService},
    },
};
use citadel_database::MigrationRunner;
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, AuthenticatedPrincipalType, IdentityError, IdentityService,
    MfaConfiguration, MfaPolicy, MfaService, NoopServiceAccountLastUsedTracker, OidcDiscovery,
    OidcIdentity, OidcProtocol, OidcProvider, OidcService, PasswordHasher, SYSTEM_ACTOR_ID,
    SystemClock,
};
use citadel_primitives::ActorId;
use citadel_server::{
    Readiness,
    api::routes::{
        authentication as identity_http, authentication::IdentityHttpState, oidc as oidc_http,
        oidc::OidcHttpState,
    },
};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, OnceLock},
};
use tokio::sync::{Mutex, OwnedMutexGuard, RwLock};
use tower::ServiceExt;
use url::Url;
use uuid::Uuid;

const PASSWORD: &str = "correct-horse-battery-staple";
const CLIENT_SECRET: &str = "super-secret-client-value";
const VIEWER_ROLE_ID: Uuid = Uuid::from_u128(0x30000000000000000000000000000003);
static TEST_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

#[derive(Clone)]
struct MockOidcProtocol {
    identity: Arc<RwLock<OidcIdentity>>,
}

impl MockOidcProtocol {
    fn new(identity: OidcIdentity) -> Self {
        Self {
            identity: Arc::new(RwLock::new(identity)),
        }
    }

    async fn set_identity(&self, identity: OidcIdentity) {
        *self.identity.write().await = identity;
    }
}

impl OidcProtocol for MockOidcProtocol {
    fn discover<'a>(
        &'a self,
        issuer: &'a str,
    ) -> BoxFuture<'a, Result<OidcDiscovery, IdentityError>> {
        Box::pin(async move {
            let issuer = issuer.trim_end_matches('/');
            Ok(OidcDiscovery {
                issuer: issuer.to_owned(),
                authorization_endpoint: format!("{issuer}/authorize"),
                token_endpoint: format!("{issuer}/token"),
                jwks_uri: format!("{issuer}/jwks"),
            })
        })
    }

    fn exchange_and_validate<'a>(
        &'a self,
        _provider: &'a OidcProvider,
        _discovery: &'a OidcDiscovery,
        _client_secret: &'a str,
        _code: &'a str,
        _redirect_uri: &'a str,
        _code_verifier: &'a str,
        _nonce: &'a str,
    ) -> BoxFuture<'a, Result<OidcIdentity, IdentityError>> {
        Box::pin(async move { Ok(self.identity.read().await.clone()) })
    }
}

struct Fixture {
    app: Router,
    pool: PgPool,
    protocol: MockOidcProtocol,
    principal: ActorPrincipal,
    email: String,
    _guard: OwnedMutexGuard<()>,
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn oidc_endpoints_preserve_admin_login_link_provision_and_replay_semantics() {
    let fixture = fixture().await;

    let local_login = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/authentication/login",
        json!({ "emailOrName": fixture.email.clone(), "password": PASSWORD }),
        None,
    )
    .await;
    assert_eq!(local_login.status(), StatusCode::OK);

    let unauthorized = send(
        &fixture.app,
        Method::GET,
        "/api/v1/oidcProviders",
        None,
        None,
    )
    .await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let created = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/oidcProviders",
        provider_input("primary", true, true, true, None),
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    let created = json_body(created).await;
    let provider_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    assert_eq!(created["hasClientSecret"], true);
    assert!(created.get("clientSecret").is_none());
    let fetched = send(
        &fixture.app,
        Method::GET,
        &format!("/api/v1/oidcProviders/{provider_id}"),
        None,
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(fetched.status(), StatusCode::OK);
    assert_eq!(json_body(fetched).await["id"], provider_id.to_string());
    let listed = send(
        &fixture.app,
        Method::GET,
        "/api/v1/oidcProviders",
        None,
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = json_body(listed).await;
    let provider_id_text = provider_id.to_string();
    assert_eq!(
        listed["providers"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|provider| provider["id"].as_str() == Some(provider_id_text.as_str()))
            .count(),
        1
    );
    let ciphertext: String =
        sqlx::query_scalar("SELECT clientsecretciphertext FROM oidcproviders WHERE id = $1")
            .bind(provider_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_ne!(ciphertext, CLIENT_SECRET);
    assert!(!ciphertext.contains(CLIENT_SECRET));

    let duplicate = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/oidcProviders",
        provider_input("primary", true, true, true, None),
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);

    let patched = send_json(
        &fixture.app,
        Method::PATCH,
        &format!("/api/v1/oidcProviders/{provider_id}"),
        json!({ "description": "Changed", "clientSecret": null }),
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(patched.status(), StatusCode::OK);
    assert_eq!(json_body(patched).await["description"], "Changed");
    let retained_ciphertext: String =
        sqlx::query_scalar("SELECT clientsecretciphertext FROM oidcproviders WHERE id = $1")
            .bind(provider_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(retained_ciphertext, ciphertext);

    let renamed = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/oidcProviders/rename",
        json!({ "id": provider_id, "name": "primary-renamed" }),
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    assert_eq!(json_body(renamed).await["name"], "primary-renamed");

    let metadata = send_json(
        &fixture.app,
        Method::PATCH,
        &format!("/api/v1/oidcProviders/{provider_id}/_metadata"),
        json!({ "description": "Metadata", "tags": [] }),
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(metadata.status(), StatusCode::OK);
    assert_eq!(json_body(metadata).await["description"], "Metadata");

    let discovery = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/oidcProviders/testDiscovery",
        json!({ "providerId": null, "issuer": "https://issuer.example.test" }),
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(discovery.status(), StatusCode::OK);
    assert_eq!(
        json_body(discovery).await["authorizationEndpoint"],
        "https://issuer.example.test/authorize"
    );
    let persisted_discovery = send(
        &fixture.app,
        Method::POST,
        &format!("/api/v1/oidcProviders/{provider_id}/testDiscovery"),
        None,
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(persisted_discovery.status(), StatusCode::OK);

    let login_providers = send(
        &fixture.app,
        Method::GET,
        "/api/v1/authentication/oidc/providers",
        None,
        None,
    )
    .await;
    assert_eq!(login_providers.status(), StatusCode::OK);
    let login_providers = json_body(login_providers).await;
    assert_eq!(
        login_providers["providers"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|provider| provider["id"].as_str() == Some(provider_id_text.as_str()))
            .count(),
        1
    );

    let rejected_return = send(
        &fixture.app,
        Method::GET,
        &format!(
            "/api/v1/authentication/oidc/{provider_id}/login?returnUrl=https%3A%2F%2Fevil.example"
        ),
        None,
        None,
    )
    .await;
    assert_eq!(rejected_return.status(), StatusCode::BAD_REQUEST);

    let start = begin_login(&fixture.app, provider_id, "/after-login").await;
    let authorization_url = Url::parse(location(&start)).unwrap();
    assert_eq!(start.status(), StatusCode::FOUND);
    assert_eq!(
        authorization_url
            .query_pairs()
            .find(|(name, _)| name == "code_challenge_method")
            .unwrap()
            .1,
        "S256"
    );
    let state = authorization_url
        .query_pairs()
        .find(|(name, _)| name == "state")
        .unwrap()
        .1
        .into_owned();
    let persisted_state: String =
        sqlx::query_scalar("SELECT statehash FROM oidcloginstates WHERE providerid = $1")
            .bind(provider_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_ne!(persisted_state, state);

    let callback_uri = format!(
        "/api/v1/authentication/oidc/{provider_id}/callback?code=accepted&state={state}&session_state=keycloak-session&iss=http%3A%2F%2Fkeycloak%3A18080%2Frealms%2Fcitadel-e2e"
    );
    let callback = send(&fixture.app, Method::GET, &callback_uri, None, None).await;
    assert_eq!(callback.status(), StatusCode::FOUND);
    assert_eq!(location(&callback), "/after-login");
    let refresh_cookie = cookie_pair(&callback, "refresh_token").unwrap();
    let refreshed = send_cookie(
        &fixture.app,
        Method::GET,
        "/api/v1/authentication/refresh",
        None,
        None,
        &refresh_cookie,
    )
    .await;
    assert_eq!(refreshed.status(), StatusCode::OK);
    let refreshed_cookie = cookie_pair(&refreshed, "refresh_token").unwrap();
    let logged_out = send_cookie(
        &fixture.app,
        Method::POST,
        "/api/v1/authentication/logout",
        None,
        None,
        &refreshed_cookie,
    )
    .await;
    assert_eq!(logged_out.status(), StatusCode::NO_CONTENT);
    let linked_user_id: Uuid = sqlx::query_scalar(
        "SELECT userid FROM oidcexternallogins WHERE providerid = $1 AND subject = 'existing-subject'",
    )
    .bind(provider_id)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(linked_user_id, fixture.principal.subject_id);

    let replay = send(&fixture.app, Method::GET, &callback_uri, None, None).await;
    assert_eq!(replay.status(), StatusCode::BAD_REQUEST);

    let actor_id: Uuid = sqlx::query_scalar("SELECT actorid FROM users WHERE id = $1")
        .bind(fixture.principal.subject_id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE actors SET isenabled = FALSE WHERE id = $1")
        .bind(actor_id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let disabled_start = begin_login(&fixture.app, provider_id, "/disabled").await;
    let disabled_url = Url::parse(location(&disabled_start)).unwrap();
    let disabled_state = disabled_url
        .query_pairs()
        .find(|(name, _)| name == "state")
        .unwrap()
        .1
        .into_owned();
    let disabled_callback = send(
        &fixture.app,
        Method::GET,
        &format!(
            "/api/v1/authentication/oidc/{provider_id}/callback?code=accepted&state={disabled_state}"
        ),
        None,
        None,
    )
    .await;
    assert_eq!(disabled_callback.status(), StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE actors SET isenabled = TRUE WHERE id = $1")
        .bind(actor_id)
        .execute(&fixture.pool)
        .await
        .unwrap();

    let required_claim = send_json(
        &fixture.app,
        Method::PATCH,
        &format!("/api/v1/oidcProviders/{provider_id}"),
        json!({
            "requiredClaimName": "groups",
            "requiredClaimValues": "operators"
        }),
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(required_claim.status(), StatusCode::OK);
    let claim_start = begin_login(&fixture.app, provider_id, "/claim").await;
    let claim_url = Url::parse(location(&claim_start)).unwrap();
    let claim_state = claim_url
        .query_pairs()
        .find(|(name, _)| name == "state")
        .unwrap()
        .1
        .into_owned();
    let claim_callback = send(
        &fixture.app,
        Method::GET,
        &format!(
            "/api/v1/authentication/oidc/{provider_id}/callback?code=accepted&state={claim_state}"
        ),
        None,
        None,
    )
    .await;
    assert_eq!(claim_callback.status(), StatusCode::BAD_REQUEST);
    let clear_claim = send_json(
        &fixture.app,
        Method::PATCH,
        &format!("/api/v1/oidcProviders/{provider_id}"),
        json!({ "requiredClaimName": null, "requiredClaimValues": null }),
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(clear_claim.status(), StatusCode::OK);

    let expired_start = begin_login(&fixture.app, provider_id, "/expired").await;
    let expired_url = Url::parse(location(&expired_start)).unwrap();
    let expired_state = expired_url
        .query_pairs()
        .find(|(name, _)| name == "state")
        .unwrap()
        .1
        .into_owned();
    sqlx::query("UPDATE oidcloginstates SET expiresat = $1 WHERE providerid = $2")
        .bind(Utc::now() - Duration::minutes(1))
        .bind(provider_id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let expired_callback = send(
        &fixture.app,
        Method::GET,
        &format!(
            "/api/v1/authentication/oidc/{provider_id}/callback?code=accepted&state={expired_state}"
        ),
        None,
        None,
    )
    .await;
    assert_eq!(expired_callback.status(), StatusCode::BAD_REQUEST);

    let concurrent_start = begin_login(&fixture.app, provider_id, "/concurrent").await;
    let concurrent_url = Url::parse(location(&concurrent_start)).unwrap();
    let concurrent_state = concurrent_url
        .query_pairs()
        .find(|(name, _)| name == "state")
        .unwrap()
        .1
        .into_owned();
    let concurrent_callback = format!(
        "/api/v1/authentication/oidc/{provider_id}/callback?code=accepted&state={concurrent_state}"
    );
    let first = send(&fixture.app, Method::GET, &concurrent_callback, None, None);
    let second = send(&fixture.app, Method::GET, &concurrent_callback, None, None);
    let (first, second) = tokio::join!(first, second);
    assert_eq!(
        [first.status(), second.status()]
            .into_iter()
            .filter(|status| *status == StatusCode::FOUND)
            .count(),
        1
    );

    fixture
        .protocol
        .set_identity(identity("provisioned-subject", "new-user@example.test"))
        .await;
    let provisioned = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/oidcProviders",
        provider_input("provisioning", true, false, true, Some(VIEWER_ROLE_ID)),
        Some(fixture.principal.clone()),
    )
    .await;
    let provisioned_id =
        Uuid::parse_str(json_body(provisioned).await["id"].as_str().unwrap()).unwrap();
    let provisioned_start = begin_login(&fixture.app, provisioned_id, "/").await;
    let provisioned_url = Url::parse(location(&provisioned_start)).unwrap();
    let provisioned_state = provisioned_url
        .query_pairs()
        .find(|(name, _)| name == "state")
        .unwrap()
        .1
        .into_owned();
    let provisioned_callback = send(
        &fixture.app,
        Method::GET,
        &format!(
            "/api/v1/authentication/oidc/{provisioned_id}/callback?code=accepted&state={provisioned_state}"
        ),
        None,
        None,
    )
    .await;
    assert_eq!(provisioned_callback.status(), StatusCode::FOUND);
    let provisioned_role_count: i64 = sqlx::query_scalar(
        r#"
SELECT COUNT(*)
FROM oidcexternallogins login
JOIN users u ON u.id = login.userid
JOIN actorroles ar ON ar.actorid = u.actorid
WHERE login.providerid = $1 AND login.subject = 'provisioned-subject' AND ar.roleid = $2
"#,
    )
    .bind(provisioned_id)
    .bind(VIEWER_ROLE_ID)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(provisioned_role_count, 1);

    fixture
        .protocol
        .set_identity(identity("closed-subject", "closed@example.test"))
        .await;
    let closed = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/oidcProviders",
        provider_input("closed", true, false, false, None),
        Some(fixture.principal.clone()),
    )
    .await;
    let closed_id = Uuid::parse_str(json_body(closed).await["id"].as_str().unwrap()).unwrap();
    let closed_start = begin_login(&fixture.app, closed_id, "/").await;
    let closed_url = Url::parse(location(&closed_start)).unwrap();
    let closed_state = closed_url
        .query_pairs()
        .find(|(name, _)| name == "state")
        .unwrap()
        .1
        .into_owned();
    let closed_callback = send(
        &fixture.app,
        Method::GET,
        &format!(
            "/api/v1/authentication/oidc/{closed_id}/callback?code=accepted&state={closed_state}"
        ),
        None,
        None,
    )
    .await;
    assert_eq!(closed_callback.status(), StatusCode::NOT_FOUND);

    let disabled = send_json(
        &fixture.app,
        Method::PATCH,
        &format!("/api/v1/oidcProviders/{provider_id}"),
        json!({ "enabled": false }),
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::OK);
    let disabled_login = begin_login(&fixture.app, provider_id, "/").await;
    assert_eq!(disabled_login.status(), StatusCode::NOT_FOUND);
    let visible = send(
        &fixture.app,
        Method::GET,
        "/api/v1/authentication/oidc/providers",
        None,
        None,
    )
    .await;
    let visible = json_body(visible).await;
    let disabled_provider_id = provider_id.to_string();
    assert!(
        visible["providers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|provider| provider["id"].as_str() != Some(disabled_provider_id.as_str()))
    );

    let deleted = send(
        &fixture.app,
        Method::DELETE,
        &format!("/api/v1/oidcProviders/{provider_id}"),
        None,
        Some(fixture.principal.clone()),
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activityevents WHERE resourcetype = 'OidcProvider' AND resourceid = $1",
    )
    .bind(provider_id)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert!(activity_count >= 4);
}

async fn fixture() -> Fixture {
    let guard = TEST_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone()
        .lock_owned()
        .await;
    let database_url = std::env::var("CITADEL_PHASE3_DATABASE_URL")
        .expect("CITADEL_PHASE3_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&database_url)
        .await
        .unwrap();
    let passwords = Arc::new(Argon2PasswordHasher::default());
    let (principal, email) = seed_administrator(&pool, passwords.as_ref()).await;
    let identity_service = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        passwords,
        Arc::new(
            JwtSessionTokenCodec::new(
                &[7_u8; 32],
                "oidc-http-fixture".to_owned(),
                "oidc-http-fixture".to_owned(),
            )
            .unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let protocol = MockOidcProtocol::new(identity("existing-subject", &email));
    let key = [8_u8; 32];
    let protector = Arc::new(AesGcmSecretProtector::new(&key).unwrap());
    let service = Arc::new(OidcService::new(
        Arc::new(PostgresOidcStore::new(pool.clone())),
        Arc::new(protocol.clone()),
        protector.clone(),
        identity_service.clone(),
        Arc::new(SystemClock),
        Duration::minutes(10),
    ));
    let mfa = Arc::new(MfaService::new(
        Arc::new(PostgresMfaStore::new(pool.clone())),
        identity_service.clone(),
        Arc::new(Sha1TotpService),
        protector,
        Arc::new(HmacRecoveryCodeService::new(&key).unwrap()),
        Arc::new(SystemClock),
        MfaConfiguration {
            policy: MfaPolicy::Optional,
            challenge_lifetime: Duration::minutes(5),
            setup_lifetime: Duration::minutes(10),
            maximum_failed_attempts: 5,
            recovery_code_count: 10,
        },
    ));
    let app = oidc_http::router(OidcHttpState {
        oidc: service,
        public_url: Url::parse("http://citadel.test").unwrap(),
        allowed_return_origins: Vec::new(),
        secure_cookies: false,
    })
    .merge(identity_http::router(IdentityHttpState {
        identity: identity_service,
        mfa,
        readiness: Arc::new(Readiness::default()),
        secure_cookies: false,
    }));
    Fixture {
        app,
        pool,
        protocol,
        principal,
        email,
        _guard: guard,
    }
}

fn identity(subject: &str, email: &str) -> OidcIdentity {
    OidcIdentity {
        subject: subject.to_owned(),
        email: Some(email.to_owned()),
        email_verified: true,
        name: Some(email.split('@').next().unwrap().to_owned()),
        claims: BTreeMap::new(),
    }
}

fn provider_input(
    name: &str,
    enabled: bool,
    allow_email_auto_link: bool,
    auto_provision_users: bool,
    default_role_id: Option<Uuid>,
) -> Value {
    json!({
        "name": name,
        "description": null,
        "displayName": format!("{name} display"),
        "issuer": "https://issuer.example.test",
        "clientId": "citadel-client",
        "clientSecret": CLIENT_SECRET,
        "scopes": "openid profile email",
        "enabled": enabled,
        "autoProvisionUsers": auto_provision_users,
        "allowEmailAutoLink": allow_email_auto_link,
        "requireEmailVerified": true,
        "allowedEmailDomains": "example.test",
        "requiredClaimName": null,
        "requiredClaimValues": null,
        "defaultRoleId": default_role_id
    })
}

async fn begin_login(app: &Router, provider_id: Uuid, return_url: &str) -> Response<Body> {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("returnUrl", return_url)
        .finish();
    send(
        app,
        Method::GET,
        &format!("/api/v1/authentication/oidc/{provider_id}/login?{query}"),
        None,
        None,
    )
    .await
}

async fn seed_administrator(
    pool: &PgPool,
    passwords: &dyn PasswordHasher,
) -> (ActorPrincipal, String) {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let name = format!("oidc-admin-{}", user_id.simple());
    let email = format!("{name}@example.test");
    let password_hash = passwords.hash(PASSWORD).unwrap();
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name, password) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(user_id)
    .bind(actor_id)
    .bind(Utc::now())
    .bind(SYSTEM_ACTOR_ID)
    .bind(&email)
    .bind(&name)
    .bind(password_hash)
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
    (
        ActorPrincipal {
            subject_id: user_id,
            actor_id: ActorId::new(actor_id),
            name,
            principal_type: AuthenticatedPrincipalType::User,
            credential_id: None,
            roles: vec!["Admin".to_owned()],
        },
        email,
    )
}

async fn send_json(
    app: &Router,
    method: Method,
    uri: &str,
    body: Value,
    principal: Option<ActorPrincipal>,
) -> Response<Body> {
    send(app, method, uri, Some(body), principal).await
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    principal: Option<ActorPrincipal>,
) -> Response<Body> {
    send_request(app, method, uri, body, principal, None).await
}

async fn send_cookie(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    principal: Option<ActorPrincipal>,
    cookie: &str,
) -> Response<Body> {
    send_request(app, method, uri, body, principal, Some(cookie)).await
}

async fn send_request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    principal: Option<ActorPrincipal>,
    cookie: Option<&str>,
) -> Response<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header(CONTENT_TYPE, "application/json");
    }
    if let Some(cookie) = cookie {
        builder = builder.header(axum::http::header::COOKIE, cookie);
    }
    let mut request = builder
        .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        12345,
    )));
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    app.clone().oneshot(request).await.unwrap()
}

fn location(response: &Response<Body>) -> &str {
    response.headers().get(LOCATION).unwrap().to_str().unwrap()
}

fn cookie_pair(response: &Response<Body>, name: &str) -> Option<String> {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| {
            let pair = value.split(';').next()?;
            let (candidate, secret) = pair.split_once('=')?;
            (candidate == name && !secret.is_empty()).then(|| pair.to_owned())
        })
}

async fn json_body(response: Response<Body>) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}
