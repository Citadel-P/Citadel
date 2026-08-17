use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, OnceLock};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{Method, Request, Response, StatusCode};
use chrono::{DateTime, Duration, Utc};
use citadel_adapters::crypto::{
    AesGcmSecretProtector, Argon2PasswordHasher, JwtSessionTokenCodec,
    OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_adapters::mfa::{HmacRecoveryCodeService, PostgresMfaStore};
use citadel_database::MigrationRunner;
use citadel_domain::{ActorId, AuthenticatedPrincipalType, MfaPolicy};
use citadel_identity::{ADMIN_ROLE_ID, ActorPrincipal, SYSTEM_ACTOR_ID};
use citadel_identity::{
    IdentityError, IdentityService, MfaConfiguration, MfaService,
    NoopServiceAccountLastUsedTracker, PasswordHasher, SystemClock, TotpService, TotpSetup,
};
use citadel_server::Readiness;
use citadel_server::identity_http::{self, IdentityHttpState};
use serde_json::{Value, json};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::{Mutex, OwnedMutexGuard};
use tower::ServiceExt;
use uuid::Uuid;

const PASSWORD: &str = "correct-horse-battery-staple";
const MFA_CHALLENGE_COOKIE: &str = "citadel_mfa_challenge";
const MFA_SETUP_COOKIE: &str = "citadel_mfa_setup";
const REFRESH_COOKIE: &str = "refresh_token";
static TEST_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

#[derive(Debug, Default)]
struct DeterministicTotp;

impl TotpService for DeterministicTotp {
    fn create_setup(
        &self,
        issuer: &str,
        account_name: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<TotpSetup, IdentityError> {
        let secret = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP".to_owned();
        Ok(TotpSetup {
            otp_auth_uri: self.otp_auth_uri(issuer, account_name, &secret),
            secret,
            expires_at,
        })
    }

    fn verify(&self, _secret: &str, code: &str, _now: DateTime<Utc>) -> Option<i64> {
        let step = code.parse::<i64>().ok()?;
        (step > 0).then_some(step)
    }

    fn otp_auth_uri(&self, issuer: &str, account_name: &str, secret: &str) -> String {
        format!("otpauth://totp/{issuer}:{account_name}?secret={secret}&issuer={issuer}")
    }
}

struct Fixture {
    app: Router,
    pool: PgPool,
    identity: Arc<IdentityService>,
    principal: ActorPrincipal,
    _guard: OwnedMutexGuard<()>,
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn mfa_endpoints_preserve_login_enrollment_recovery_replay_and_reset_semantics() {
    let fixture = fixture(MfaPolicy::Optional).await;

    let initial_login = login(&fixture.app, &fixture.principal.name).await;
    assert_eq!(initial_login.status(), StatusCode::OK);
    assert!(cookie_value(&initial_login, REFRESH_COOKIE).is_some());
    assert_eq!(json_body(initial_login).await["nextStep"], "Completed");

    let setup = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/profile/mfa/setup",
        json!({ "password": PASSWORD }),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(setup.status(), StatusCode::OK);
    assert!(
        json_body(setup).await["otpAuthUri"]
            .as_str()
            .unwrap()
            .starts_with("otpauth://totp/Citadel:")
    );

    let enrollment_login = login(&fixture.app, &fixture.principal.name).await;
    let refresh = cookie_value(&enrollment_login, REFRESH_COOKIE).unwrap();
    let confirmed = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/profile/mfa/setup/confirm",
        json!({ "code": "111111" }),
        Some(fixture.principal.clone()),
        Some(&refresh),
    )
    .await;
    assert_eq!(confirmed.status(), StatusCode::OK);
    let recovery_codes = json_body(confirmed).await["recoveryCodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(recovery_codes.len(), 10);

    let challenge = login(&fixture.app, &fixture.principal.name).await;
    assert!(cookie_value(&challenge, REFRESH_COOKIE).is_none());
    let challenge_cookie = cookie_value(&challenge, MFA_CHALLENGE_COOKIE).unwrap();
    assert_eq!(json_body(challenge).await["nextStep"], "VerifyMfa");
    let rejected = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/authentication/mfa/verify",
        json!({ "code": "000000" }),
        None,
        Some(&challenge_cookie),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    let verified = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/authentication/mfa/verify",
        json!({ "code": "222222" }),
        None,
        Some(&challenge_cookie),
    )
    .await;
    assert_eq!(verified.status(), StatusCode::OK);
    assert!(cookie_value(&verified, REFRESH_COOKIE).is_some());

    let (first_login, second_login) = tokio::join!(
        login(&fixture.app, &fixture.principal.name),
        login(&fixture.app, &fixture.principal.name)
    );
    assert_eq!(first_login.status(), StatusCode::OK);
    assert_eq!(second_login.status(), StatusCode::OK);
    assert_eq!(
        active_challenge_count(&fixture.pool, fixture.principal.subject_id).await,
        1
    );

    let recovery_challenge = login(&fixture.app, &fixture.principal.name).await;
    let recovery_challenge_cookie =
        cookie_value(&recovery_challenge, MFA_CHALLENGE_COOKIE).unwrap();
    let recovered = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/authentication/mfa/verify",
        json!({ "recoveryCode": recovery_codes[0] }),
        None,
        Some(&recovery_challenge_cookie),
    )
    .await;
    assert_eq!(recovered.status(), StatusCode::OK);

    let replay_challenge = login(&fixture.app, &fixture.principal.name).await;
    let replay_cookie = cookie_value(&replay_challenge, MFA_CHALLENGE_COOKIE).unwrap();
    let replayed = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/authentication/mfa/verify",
        json!({ "recoveryCode": recovery_codes[0] }),
        None,
        Some(&replay_cookie),
    )
    .await;
    assert_eq!(replayed.status(), StatusCode::BAD_REQUEST);

    let locked_challenge = login(&fixture.app, &fixture.principal.name).await;
    let locked_cookie = cookie_value(&locked_challenge, MFA_CHALLENGE_COOKIE).unwrap();
    for _ in 0..5 {
        let rejected = send_json(
            &fixture.app,
            Method::POST,
            "/api/v1/authentication/mfa/verify",
            json!({ "code": "000000" }),
            None,
            Some(&locked_cookie),
        )
        .await;
        assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    }
    let locked_out = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/authentication/mfa/verify",
        json!({ "code": "333333" }),
        None,
        Some(&locked_cookie),
    )
    .await;
    assert_eq!(locked_out.status(), StatusCode::BAD_REQUEST);

    let concurrent_challenge = login(&fixture.app, &fixture.principal.name).await;
    let concurrent_cookie = cookie_value(&concurrent_challenge, MFA_CHALLENGE_COOKIE).unwrap();
    let first = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/authentication/mfa/verify",
        json!({ "code": "333333" }),
        None,
        Some(&concurrent_cookie),
    );
    let second = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/authentication/mfa/verify",
        json!({ "code": "333333" }),
        None,
        Some(&concurrent_cookie),
    );
    let (first, second) = tokio::join!(first, second);
    assert_eq!(
        [first.status(), second.status()]
            .into_iter()
            .filter(|status| *status == StatusCode::OK)
            .count(),
        1
    );

    let regenerated = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/profile/mfa/recovery-codes",
        json!({ "password": PASSWORD, "code": "444444" }),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(regenerated.status(), StatusCode::OK);
    assert_eq!(
        json_body(regenerated).await["recoveryCodes"]
            .as_array()
            .unwrap()
            .len(),
        10
    );

    let status = send(
        &fixture.app,
        Method::GET,
        "/api/v1/profile/mfa",
        None,
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(json_body(status).await["enabled"], true);

    let disabled = send_json(
        &fixture.app,
        Method::POST,
        "/api/v1/profile/mfa/disable",
        json!({ "password": PASSWORD, "code": "555555" }),
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        mfa_settings_count(&fixture.pool, fixture.principal.subject_id).await,
        0
    );

    let required_app = router(
        fixture.identity.clone(),
        fixture.pool.clone(),
        MfaPolicy::RequiredForAllUsers,
    );
    let enrollment = login(&required_app, &fixture.principal.name).await;
    assert!(cookie_value(&enrollment, REFRESH_COOKIE).is_none());
    let setup_cookie = cookie_value(&enrollment, MFA_SETUP_COOKIE).unwrap();
    assert_eq!(json_body(enrollment).await["nextStep"], "EnrollMfa");
    let setup = send(
        &required_app,
        Method::GET,
        "/api/v1/authentication/mfa/setup",
        None,
        None,
        Some(&setup_cookie),
    )
    .await;
    assert_eq!(setup.status(), StatusCode::OK);
    let completed = send_json(
        &required_app,
        Method::POST,
        "/api/v1/authentication/mfa/setup/confirm",
        json!({ "code": "666666" }),
        None,
        Some(&setup_cookie),
    )
    .await;
    assert_eq!(completed.status(), StatusCode::OK);
    assert!(cookie_value(&completed, REFRESH_COOKIE).is_some());

    let reset = send(
        &required_app,
        Method::DELETE,
        &format!("/api/v1/users/{}/mfa", fixture.principal.subject_id),
        None,
        Some(fixture.principal.clone()),
        None,
    )
    .await;
    assert_eq!(reset.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        mfa_settings_count(&fixture.pool, fixture.principal.subject_id).await,
        0
    );
    assert_eq!(
        refresh_token_count(&fixture.pool, fixture.principal.subject_id).await,
        0
    );
}

async fn fixture(policy: MfaPolicy) -> Fixture {
    let guard = TEST_LOCK
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone()
        .lock_owned()
        .await;
    let database_url = std::env::var("CITADEL_PHASE3_DATABASE_URL")
        .expect("CITADEL_PHASE3_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url)
        .await
        .unwrap();
    let passwords = Arc::new(Argon2PasswordHasher::default());
    let principal = seed_administrator(&pool, passwords.as_ref()).await;
    let identity = identity(&pool, passwords);
    let app = router(identity.clone(), pool.clone(), policy);
    Fixture {
        app,
        pool,
        identity,
        principal,
        _guard: guard,
    }
}

fn identity(pool: &PgPool, passwords: Arc<Argon2PasswordHasher>) -> Arc<IdentityService> {
    Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        passwords,
        Arc::new(
            JwtSessionTokenCodec::new(
                &[7_u8; 32],
                "mfa-http-fixture".to_owned(),
                "mfa-http-fixture".to_owned(),
            )
            .unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ))
}

fn router(identity: Arc<IdentityService>, pool: PgPool, policy: MfaPolicy) -> Router {
    let key = [8_u8; 32];
    let mfa = Arc::new(MfaService::new(
        Arc::new(PostgresMfaStore::new(pool)),
        identity.clone(),
        Arc::new(DeterministicTotp),
        Arc::new(AesGcmSecretProtector::new(&key).unwrap()),
        Arc::new(HmacRecoveryCodeService::new(&key).unwrap()),
        Arc::new(SystemClock),
        MfaConfiguration {
            policy,
            challenge_lifetime: Duration::minutes(5),
            setup_lifetime: Duration::minutes(10),
            maximum_failed_attempts: 5,
            recovery_code_count: 10,
        },
    ));
    identity_http::router(IdentityHttpState {
        identity,
        mfa,
        readiness: Arc::new(Readiness::default()),
        secure_cookies: false,
    })
}

async fn seed_administrator(pool: &PgPool, passwords: &dyn PasswordHasher) -> ActorPrincipal {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let name = format!("mfa-admin-{}", user_id.simple());
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
    .bind(format!("{name}@example.test"))
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
    ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name,
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Admin".to_owned()],
    }
}

async fn login(app: &Router, name: &str) -> Response<Body> {
    send_json(
        app,
        Method::POST,
        "/api/v1/authentication/login",
        json!({ "emailOrName": name, "password": PASSWORD }),
        None,
        None,
    )
    .await
}

async fn send_json(
    app: &Router,
    method: Method,
    uri: &str,
    body: Value,
    principal: Option<ActorPrincipal>,
    cookie: Option<&str>,
) -> Response<Body> {
    send(app, method, uri, Some(body), principal, cookie).await
}

async fn send(
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
        builder = builder.header(COOKIE, cookie);
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

fn cookie_value(response: &Response<Body>, name: &str) -> Option<String> {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| {
            value
                .split(';')
                .next()
                .and_then(|pair| pair.split_once('='))
                .filter(|(candidate, value)| *candidate == name && !value.is_empty())
                .map(|(candidate, value)| format!("{candidate}={value}"))
        })
}

async fn json_body(response: Response<Body>) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

async fn mfa_settings_count(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM usermfasettings WHERE userid = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn refresh_token_count(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM refreshtokens WHERE userid = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn active_challenge_count(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM mfachallenges WHERE userid = $1 AND consumedat IS NULL AND expiresat > $2",
    )
    .bind(user_id)
    .bind(Utc::now())
    .fetch_one(pool)
    .await
    .unwrap()
}
