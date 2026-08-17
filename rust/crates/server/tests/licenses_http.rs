use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, Response, StatusCode};
use chrono::{DateTime, Duration, Utc};
use citadel_adapters::crypto::{
    Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_adapters::license::PostgresLicenseStore;
use citadel_application::{
    LicenseService, LicenseTransitionMonitor, LicenseValidationPersistence, LicenseVerifier,
};
use citadel_database::MigrationRunner;
use citadel_domain::{
    ActorId, AuthenticatedPrincipalType, CURRENT_LICENSE_SCHEMA, CitadelInstanceIdentity,
    LicenseCapability, LicenseCustomer, LicensePayload, LicenseStatus, LicenseVerificationResult,
    VerifiedLicense,
};
use citadel_identity::{ADMIN_ROLE_ID, ActorPrincipal, SYSTEM_ACTOR_ID};
use citadel_identity::{
    AccessTokenClaims, Clock, IdentityService, NoopServiceAccountLastUsedTracker,
    SessionTokenCodec, SystemClock,
};
use citadel_server::license_http::{self, LicenseHttpState};
use citadel_server::license_realtime::{LicenseRealtimeHub, LicenseRealtimeService};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::{Mutex, OwnedMutexGuard};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    app: Router,
    pool: PgPool,
    administrator: ActorPrincipal,
    viewer: ActorPrincipal,
    _guard: OwnedMutexGuard<()>,
}

#[derive(Default)]
struct FixtureVerifier;

#[derive(Clone, Copy)]
struct FixedClock(DateTime<Utc>);

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

struct TransitionVerifier;

impl LicenseVerifier for TransitionVerifier {
    fn verify(
        &self,
        raw_license: &str,
        identity: &CitadelInstanceIdentity,
        now: DateTime<Utc>,
    ) -> LicenseVerificationResult {
        LicenseVerificationResult {
            status: LicenseStatus::Expired,
            license: Some(VerifiedLicense {
                raw_license: raw_license.to_owned(),
                fingerprint: "fingerprint-transition".to_owned(),
                payload: LicensePayload {
                    schema: CURRENT_LICENSE_SCHEMA,
                    product: "citadel".to_owned(),
                    issuer: "citadel-p".to_owned(),
                    audience: "citadel-core".to_owned(),
                    license_id: "lic-transition".to_owned(),
                    replaced_license_id: None,
                    customer: LicenseCustomer {
                        id: "customer".to_owned(),
                        name: "Customer".to_owned(),
                    },
                    edition: "Team".to_owned(),
                    instance_id: identity.instance_id,
                    issued_at: now - Duration::days(4),
                    not_before: now - Duration::days(3),
                    expires_at: now - Duration::days(2),
                    grace_until: Some(now - Duration::days(1)),
                    limits: Default::default(),
                    capabilities: vec!["custom-access-control".to_owned()],
                },
                known_capabilities: BTreeSet::from([LicenseCapability::CustomAccessControl]),
                warnings: Vec::new(),
            }),
            error_code: None,
            error_message: None,
        }
    }
}

impl LicenseVerifier for FixtureVerifier {
    fn verify(
        &self,
        raw_license: &str,
        identity: &CitadelInstanceIdentity,
        now: DateTime<Utc>,
    ) -> LicenseVerificationResult {
        if raw_license == "invalid" {
            return LicenseVerificationResult {
                status: LicenseStatus::Invalid,
                license: None,
                error_code: Some("LICENSE_INVALID"),
                error_message: Some("License is not valid.".to_owned()),
            };
        }
        let (license_id, replaced_license_id, status) = match raw_license.trim() {
            "first" => ("lic-first", None, LicenseStatus::Valid),
            "replacement" => ("lic-replacement", Some("lic-first"), LicenseStatus::Valid),
            "mismatch" => (
                "lic-mismatch",
                Some("another-license"),
                LicenseStatus::Valid,
            ),
            "future" => ("lic-future", Some("lic-first"), LicenseStatus::NotYetValid),
            _ => {
                return LicenseVerificationResult {
                    status: LicenseStatus::Invalid,
                    license: None,
                    error_code: Some("LICENSE_INVALID"),
                    error_message: Some("License is not valid.".to_owned()),
                };
            }
        };
        let not_before = if status == LicenseStatus::NotYetValid {
            now + Duration::days(1)
        } else {
            now - Duration::days(1)
        };
        LicenseVerificationResult {
            status,
            license: Some(VerifiedLicense {
                raw_license: raw_license.trim().to_owned(),
                fingerprint: format!("fingerprint-{license_id}"),
                payload: LicensePayload {
                    schema: CURRENT_LICENSE_SCHEMA,
                    product: "citadel".to_owned(),
                    issuer: "citadel-p".to_owned(),
                    audience: "citadel-core".to_owned(),
                    license_id: license_id.to_owned(),
                    replaced_license_id: replaced_license_id.map(str::to_owned),
                    customer: LicenseCustomer {
                        id: "customer".to_owned(),
                        name: "Customer".to_owned(),
                    },
                    edition: "Team".to_owned(),
                    instance_id: identity.instance_id,
                    issued_at: now - Duration::days(2),
                    not_before,
                    expires_at: now + Duration::days(30),
                    grace_until: Some(now + Duration::days(44)),
                    limits: Default::default(),
                    capabilities: vec!["custom-access-control".to_owned()],
                },
                known_capabilities: BTreeSet::from([LicenseCapability::CustomAccessControl]),
                warnings: Vec::new(),
            }),
            error_code: None,
            error_message: None,
        }
    }
}

static TEST_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn community_views_are_safe_and_instance_identity_survives_service_reconstruction() {
    let fixture = fixture().await;
    let entitlements = send_as(
        &fixture,
        fixture.viewer.clone(),
        Method::GET,
        "/api/v1/license/entitlements",
        None,
    )
    .await;
    assert_eq!(entitlements.status(), StatusCode::OK);
    let entitlements = json(entitlements).await;
    assert_eq!(entitlements["status"], "Community");
    assert_eq!(entitlements["capabilities"].as_array().unwrap().len(), 5);
    assert!(entitlements.get("instanceId").is_none());
    assert!(entitlements.get("customerName").is_none());

    let request = send(&fixture, Method::GET, "/api/v1/license/request", None).await;
    assert_eq!(request.status(), StatusCode::OK);
    let request = json(request).await;
    let instance_id = request["instanceId"].as_str().unwrap().to_owned();
    assert_eq!(request["product"], "citadel");

    let rebuilt = license_router(&fixture.pool);
    let second = send_to(
        rebuilt,
        fixture.administrator.clone(),
        Method::GET,
        "/api/v1/license/request",
        None,
    )
    .await;
    assert_eq!(json(second).await["instanceId"], instance_id);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn administrator_endpoints_enforce_license_permissions_but_entitlements_do_not() {
    let fixture = fixture().await;
    let denied = send_as(
        &fixture,
        fixture.viewer.clone(),
        Method::GET,
        "/api/v1/license",
        None,
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let anonymous = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/license/entitlements")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn install_persists_the_raw_source_but_never_exposes_it_and_writes_a_safe_activity() {
    let fixture = fixture().await;
    let response = send(
        &fixture,
        Method::POST,
        "/api/v1/license",
        Some(serde_json::json!({ "license": "  first\n" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["status"], "Valid");
    assert_eq!(body["effectiveEdition"], "Team");
    assert_eq!(body["licenseId"], "lic-first");
    assert!(body.get("rawLicense").is_none());
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT rawlicense FROM installedlicenses WHERE id = 1")
            .fetch_one(&fixture.pool)
            .await
            .unwrap(),
        "first"
    );
    let info = sqlx::query_scalar::<_, String>(
        "SELECT info FROM activityevents WHERE eventtype = 'LicenseInstalled' ORDER BY createdat DESC LIMIT 1",
    )
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert!(info.contains("\"$type\":\"LicenseInstalled\""));
    assert!(!info.contains("rawLicense"));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn replacement_requires_the_installed_license_id_and_rejects_future_takeover() {
    let fixture = fixture().await;
    assert_eq!(
        send(
            &fixture,
            Method::POST,
            "/api/v1/license",
            Some(serde_json::json!({ "license": "first" })),
        )
        .await
        .status(),
        StatusCode::OK
    );
    let mismatch = send(
        &fixture,
        Method::POST,
        "/api/v1/license",
        Some(serde_json::json!({ "license": "mismatch" })),
    )
    .await;
    assert_eq!(mismatch.status(), StatusCode::CONFLICT);
    assert_eq!(
        json(mismatch).await["type"],
        "https://citadel.local/problems/license-replacement-mismatch"
    );
    let future = send(
        &fixture,
        Method::POST,
        "/api/v1/license",
        Some(serde_json::json!({ "license": "future" })),
    )
    .await;
    assert_eq!(future.status(), StatusCode::CONFLICT);
    assert_eq!(
        json(future).await["type"],
        "https://citadel.local/problems/license-replacement-not-yet-effective"
    );
    let replacement = send(
        &fixture,
        Method::POST,
        "/api/v1/license",
        Some(serde_json::json!({ "license": "replacement" })),
    )
    .await;
    assert_eq!(replacement.status(), StatusCode::OK);
    assert_eq!(json(replacement).await["licenseId"], "lic-replacement");
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn removal_is_idempotent_and_records_only_one_activity() {
    let fixture = fixture().await;
    send(
        &fixture,
        Method::POST,
        "/api/v1/license",
        Some(serde_json::json!({ "license": "first" })),
    )
    .await;
    for _ in 0..2 {
        let response = send(&fixture, Method::DELETE, "/api/v1/license", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json(response).await["status"], "Community");
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM activityevents WHERE eventtype = 'LicenseRemoved'"
        )
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        1
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn concurrent_initial_installs_cannot_bypass_replacement_rules() {
    let fixture = fixture().await;
    let first = send_to(
        fixture.app.clone(),
        fixture.administrator.clone(),
        Method::POST,
        "/api/v1/license",
        Some(serde_json::json!({ "license": "first" })),
    );
    let second = send_to(
        fixture.app.clone(),
        fixture.administrator.clone(),
        Method::POST,
        "/api/v1/license",
        Some(serde_json::json!({ "license": "first" })),
    );

    let (first, second) = tokio::join!(first, second);
    let statuses = [first.status(), second.status()];

    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CONFLICT)
            .count(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM activityevents WHERE eventtype = 'LicenseInstalled'"
        )
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        1
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn concurrent_transition_checks_persist_status_and_one_system_activity_atomically() {
    let fixture = fixture().await;
    let now = Utc::now();
    sqlx::query(
        r#"
INSERT INTO installedlicenses (
    id, rawlicense, fingerprint, installedat, installedbyactorid,
    lastvalidatedat, lastvalidationstatus, lastvalidationerrorcode)
VALUES (1, 'transition', 'fingerprint-transition', $1, $2, $1, 'Valid', NULL)
"#,
    )
    .bind(now - Duration::days(10))
    .bind(fixture.administrator.actor_id.value())
    .execute(&fixture.pool)
    .await
    .unwrap();
    let store = Arc::new(PostgresLicenseStore::new(fixture.pool.clone()));
    let monitor = LicenseTransitionMonitor::new(
        store,
        Arc::new(TransitionVerifier),
        Arc::new(FixedClock(now)),
    );

    let (first, second) = tokio::join!(monitor.check_once(), monitor.check_once());
    let persistences = [first.unwrap().persistence, second.unwrap().persistence];
    assert_eq!(
        persistences
            .iter()
            .filter(|value| **value == LicenseValidationPersistence::Transitioned)
            .count(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT lastvalidationstatus FROM installedlicenses WHERE id = 1"
        )
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        "Expired"
    );
    let row = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT createdbyactorid, info FROM activityevents WHERE eventtype = 'LicenseExpired'",
    )
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(row.0, SYSTEM_ACTOR_ID);
    assert!(row.1.contains("\"$type\":\"LicenseExpired\""));
    assert!(!row.1.contains("rawLicense"));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn realtime_transition_notification_is_authenticated_versioned_and_metadata_free() {
    let fixture = fixture().await;
    let identity = identity_service(&fixture.pool);
    let hub = LicenseRealtimeHub::default();
    let shutdown = CancellationToken::new();
    let app = LicenseRealtimeService::new(identity, hub.clone(), shutdown.clone()).router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn({
        let shutdown = shutdown.clone();
        async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown.cancelled_owned())
                .await
        }
    });
    let codec = JwtSessionTokenCodec::new(
        &[13_u8; 32],
        "licenses-http".to_owned(),
        "licenses-http".to_owned(),
    )
    .unwrap();
    let now = Utc::now();
    let token = codec
        .encode_access(&AccessTokenClaims {
            subject_id: fixture.administrator.subject_id,
            actor_id: fixture.administrator.actor_id,
            principal_type: AuthenticatedPrincipalType::User,
            issued_at: now,
            expires_at: now + Duration::minutes(5),
        })
        .unwrap();
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    socket
        .send(Message::Text(
            serde_json::json!({
                "protocolVersion": 1,
                "kind": "subscribe",
                "accessToken": token,
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    let acknowledged = tokio::time::timeout(std::time::Duration::from_secs(5), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let Message::Text(acknowledged) = acknowledged else {
        panic!("expected a subscription acknowledgement");
    };
    assert_eq!(
        serde_json::from_str::<Value>(acknowledged.as_str()).unwrap()["kind"],
        "subscribed"
    );
    let instance_id = Uuid::now_v7();
    hub.publish(instance_id);
    let message = tokio::time::timeout(std::time::Duration::from_secs(5), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let Message::Text(message) = message else {
        panic!("expected a text notification");
    };
    let envelope: Value = serde_json::from_str(message.as_str()).unwrap();
    assert_eq!(envelope["protocolVersion"], 1);
    assert_eq!(envelope["resourceType"], "License");
    assert_eq!(envelope["resourceId"], instance_id.to_string());
    assert_eq!(envelope["eventKind"], "licenseStateChanged");
    assert_eq!(envelope["payload"], serde_json::json!({}));
    let serialized = message.as_str();
    assert!(!serialized.contains("customer"));
    assert!(!serialized.contains("fingerprint"));
    assert!(!serialized.contains("edition"));
    assert!(!serialized.contains("status"));
    assert!(!serialized.contains("rawLicense"));

    socket.close(None).await.unwrap();
    shutdown.cancel();
    server.await.unwrap().unwrap();
}

async fn fixture() -> Fixture {
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
    sqlx::query("DELETE FROM installedlicenses")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM citadelinstanceidentity")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE resourcetype = 'License'")
        .execute(&pool)
        .await
        .unwrap();
    let administrator = seed_user(&pool, true).await;
    let viewer = seed_user(&pool, false).await;
    let app = license_router(&pool);
    Fixture {
        app,
        pool,
        administrator,
        viewer,
        _guard: guard,
    }
}

fn license_router(pool: &PgPool) -> Router {
    let identity = identity_service(pool);
    let licenses = Arc::new(LicenseService::new(
        Arc::new(PostgresLicenseStore::new(pool.clone())),
        Arc::new(FixtureVerifier),
        Arc::new(SystemClock),
        "1.0.0".to_owned(),
    ));
    license_http::router(LicenseHttpState { identity, licenses })
}

fn identity_service(pool: &PgPool) -> Arc<IdentityService> {
    Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(
                &[13_u8; 32],
                "licenses-http".to_owned(),
                "licenses-http".to_owned(),
            )
            .unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(false)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ))
}

async fn seed_user(pool: &PgPool, administrator: bool) -> ActorPrincipal {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let name = format!("license-user-{}", user_id.simple());
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
    if administrator {
        sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
            .bind(actor_id)
            .bind(ADMIN_ROLE_ID)
            .execute(&mut *transaction)
            .await
            .unwrap();
    }
    transaction.commit().await.unwrap();
    ActorPrincipal {
        subject_id: user_id,
        actor_id: ActorId::new(actor_id),
        name,
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: if administrator {
            vec!["Admin".to_owned()]
        } else {
            vec!["Viewer".to_owned()]
        },
    }
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
    send_to(fixture.app.clone(), principal, method, uri, body).await
}

async fn send_to(
    app: Router,
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
        .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
        .unwrap();
    request.extensions_mut().insert(principal);
    app.oneshot(request).await.unwrap()
}

async fn json(response: Response<Body>) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
