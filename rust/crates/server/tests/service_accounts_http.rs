use std::sync::{Arc, OnceLock};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, Response, StatusCode, header};
use chrono::{Duration, Utc};
use citadel_adapters::crypto::{
    Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_adapters::service_account_store::PostgresServiceAccountStore;
use citadel_database::MigrationRunner;
use citadel_identity::AuthenticatedPrincipalType;
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, IdentityService, NoopServiceAccountLastUsedTracker,
    SYSTEM_ACTOR_ID, ServiceAccountService, SystemClock,
};
use citadel_primitives::ActorId;
use citadel_server::service_accounts_http::{self, ServiceAccountHttpState};
use serde_json::Value;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::{Mutex, OwnedMutexGuard};
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    app: Router,
    pool: PgPool,
    administrator: ActorPrincipal,
    identity: Arc<IdentityService>,
    _guard: OwnedMutexGuard<()>,
}

static TEST_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn usages_resolve_the_account_actor_and_include_inactive_execution_dependencies() {
    let f = fixture(true).await;
    let account = create_account(&f, &format!("usage-{}", Uuid::now_v7()), true).await;
    let actor: Uuid = sqlx::query_scalar("SELECT actorid FROM serviceaccounts WHERE id=$1")
        .bind(account)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    let path = format!("/api/v1/serviceAccounts/{account}/usages");
    assert_eq!(
        json(send(&f, Method::GET, &path, None, Some(f.administrator.clone())).await).await,
        serde_json::json!([])
    );
    let action = Uuid::now_v7();
    let action_name = format!("Action-{action}");
    sqlx::query("INSERT INTO actions(id,name,code,enabled,scheduleenabled,scheduletimezone,timeoutseconds,alertonfailure,createdbyactorid,runasactorid) VALUES($1,$4,'',true,false,'UTC',60,false,$2,$3)")
        .bind(action).bind(f.administrator.actor_id.value()).bind(actor).bind(&action_name).execute(&f.pool).await.unwrap();
    let secret = Uuid::now_v7();
    let repository = Uuid::now_v7();
    let policy = Uuid::now_v7();
    sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'Internal')")
        .bind(secret)
        .bind(secret.to_string())
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO backuprepositories(id,name,normalizedname,passwordsecretid,spec,status,type,createdbyactorid) VALUES($1,$2,$2,$3,'{}','Unknown','FileSystem',$4)")
        .bind(repository).bind(repository.to_string()).bind(secret).bind(f.administrator.actor_id.value()).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO backuppolicies(id,name,normalizedname,backuprepositoryid,createdbyactorid,runasactorid,source,enabled,archivedat) VALUES($1,'Policy',$2,$3,$4,$5,'{}',true,CURRENT_TIMESTAMP)")
        .bind(policy).bind(policy.to_string()).bind(repository).bind(f.administrator.actor_id.value()).bind(actor).execute(&f.pool).await.unwrap();
    let expected = serde_json::json!([
        {"id":action,"name":action_name,"resourceType":"AutomationAction","isActive":true},
        {"id":policy,"name":"Policy","resourceType":"BackupPolicy","isActive":false}
    ]);
    let response = send(&f, Method::GET, &path, None, Some(f.administrator.clone())).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await, expected);
    assert_eq!(
        send(&f, Method::GET, &path, None, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let mut reader = f.administrator.clone();
    reader.roles.clear();
    assert_eq!(
        send(&f, Method::GET, &path, None, Some(reader))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            &f,
            Method::GET,
            &format!("/api/v1/serviceAccounts/{}/usages", Uuid::now_v7()),
            None,
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    let other = create_account(&f, &format!("unrelated-{}", Uuid::now_v7()), true).await;
    assert_eq!(
        json(
            send(
                &f,
                Method::GET,
                &format!("/api/v1/serviceAccounts/{other}/usages"),
                None,
                Some(f.administrator.clone())
            )
            .await
        )
        .await,
        serde_json::json!([])
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn lifecycle_routes_persist_assignments_archive_and_safe_activities() {
    let fixture = fixture(true).await;
    let marker = Uuid::now_v7().simple().to_string();
    let role_id = seed_role(&fixture.pool, &format!("runner-{marker}")).await;
    let team_id = seed_team(&fixture.pool, &format!("automation-{marker}")).await;
    let target_id = Uuid::now_v7();

    let created = send_json(
        &fixture,
        Method::POST,
        "/api/v1/serviceAccounts",
        serde_json::json!({
            "name": format!("ci-{marker}"),
            "description": "CI runner",
            "isEnabled": true,
            "teamIds": [team_id],
            "roleIds": [role_id],
            "resourceAccesses": [{
                "resourceType": "Platform",
                "resourceId": target_id,
                "permissionLevel": "Read",
                "specificPermissions": ["Inspect"]
            }]
        }),
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    let created = json(created).await;
    let account_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    assert_eq!(created["roles"][0]["id"], role_id.to_string());
    assert_eq!(created["teams"][0]["id"], team_id.to_string());
    assert_eq!(
        created["resourceAccesses"][0]["resourceId"],
        target_id.to_string()
    );
    assert_eq!(
        activity_count(&fixture.pool, account_id, "ServiceAccountCreated").await,
        1
    );

    let renamed = send_json(
        &fixture,
        Method::POST,
        "/api/v1/serviceAccounts/rename",
        serde_json::json!({ "id": account_id, "name": format!("renamed-{marker}") }),
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    assert_eq!(
        activity_count(&fixture.pool, account_id, "ServiceAccountRenamed").await,
        1
    );

    let disabled = send_json(
        &fixture,
        Method::PATCH,
        &format!("/api/v1/serviceAccounts/{account_id}"),
        serde_json::json!({ "isEnabled": false }),
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::OK);
    assert_eq!(
        activity_count(&fixture.pool, account_id, "ServiceAccountDisabled").await,
        1
    );

    let archived = send_json(
        &fixture,
        Method::DELETE,
        "/api/v1/serviceAccounts",
        serde_json::json!({ "ids": [account_id] }),
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(archived.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        activity_count(&fixture.pool, account_id, "ServiceAccountArchived").await,
        1
    );
    let activity_info: Vec<String> = sqlx::query_scalar(
        "SELECT info FROM activityevents WHERE resourceid = $1 ORDER BY createdat",
    )
    .bind(account_id)
    .fetch_all(&fixture.pool)
    .await
    .unwrap();
    assert!(activity_info.iter().all(|info| {
        let info = info.to_ascii_lowercase();
        !info.contains("token") && !info.contains("secret") && !info.contains("password")
    }));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn token_is_returned_once_persisted_as_digest_and_revocation_is_idempotent() {
    let fixture = fixture(true).await;
    let account_id = create_account(
        &fixture,
        &format!("token-owner-{}", Uuid::now_v7().simple()),
        true,
    )
    .await;
    let response = send_json(
        &fixture,
        Method::POST,
        &format!("/api/v1/serviceAccounts/{account_id}/tokens"),
        serde_json::json!({ "name": "pipeline", "neverExpires": true }),
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()[header::PRAGMA], "no-cache");
    let created = json(response).await;
    let plaintext = created["token"].as_str().unwrap().to_owned();
    let token_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    assert!(plaintext.starts_with("cit_sa_"));
    let digest: Vec<u8> =
        sqlx::query_scalar("SELECT secrethash FROM serviceaccounttokens WHERE id = $1")
            .bind(token_id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(digest.len(), 32);
    assert_ne!(digest, plaintext.as_bytes());
    assert_eq!(
        activity_count(&fixture.pool, account_id, "ServiceAccountTokenCreated").await,
        1
    );
    assert_eq!(
        fixture
            .identity
            .authenticate_bearer(&plaintext)
            .await
            .unwrap()
            .subject_id,
        account_id
    );

    for _ in 0..2 {
        let revoked = send_json(
            &fixture,
            Method::DELETE,
            &format!("/api/v1/serviceAccounts/{account_id}/tokens/{token_id}"),
            Value::Null,
            Some(fixture.administrator.clone()),
        )
        .await;
        assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
    }
    assert!(
        fixture
            .identity
            .authenticate_bearer(&plaintext)
            .await
            .is_err()
    );
    assert_eq!(
        activity_count(&fixture.pool, account_id, "ServiceAccountTokenRevoked").await,
        1
    );
    let token_info: Vec<String> = sqlx::query_scalar(
        "SELECT info FROM activityevents WHERE resourceid = $1 AND eventtype LIKE 'ServiceAccountToken%'",
    )
    .bind(account_id)
    .fetch_all(&fixture.pool)
    .await
    .unwrap();
    assert!(token_info.iter().all(|info| !info.contains(&plaintext)));
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn token_validation_applies_default_never_and_maximum_lifetimes() {
    let fixture = fixture(true).await;
    let account_id = create_account(
        &fixture,
        &format!("lifetimes-{}", Uuid::now_v7().simple()),
        true,
    )
    .await;
    let limits = send(
        &fixture,
        Method::GET,
        "/api/v1/serviceAccounts/limits",
        None,
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(limits.status(), StatusCode::OK);
    let limits = json(limits).await;
    assert_eq!(limits["defaultTokenLifetimeDays"], 90);
    assert_eq!(limits["maximumTokenLifetimeDays"], 365);

    let defaulted = create_token(&fixture, account_id, "defaulted", serde_json::json!({})).await;
    let expires = chrono::DateTime::parse_from_rfc3339(defaulted["expiresAtUtc"].as_str().unwrap())
        .unwrap()
        .with_timezone(&Utc);
    assert!(expires > Utc::now() + Duration::days(89));

    let never = create_token(
        &fixture,
        account_id,
        "never",
        serde_json::json!({ "neverExpires": true }),
    )
    .await;
    assert!(never["expiresAtUtc"].is_null());

    let invalid = send_json(
        &fixture,
        Method::POST,
        &format!("/api/v1/serviceAccounts/{account_id}/tokens"),
        serde_json::json!({
            "name": "invalid",
            "neverExpires": true,
            "expiresAtUtc": (Utc::now() + Duration::days(1)).to_rfc3339()
        }),
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let too_long = send_json(
        &fixture,
        Method::POST,
        &format!("/api/v1/serviceAccounts/{account_id}/tokens"),
        serde_json::json!({
            "name": "too-long",
            "expiresAtUtc": (Utc::now() + Duration::days(366)).to_rfc3339()
        }),
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(too_long.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn license_human_and_enabled_account_boundaries_are_enforced() {
    let unlicensed = fixture(false).await;
    let denied = send_json(
        &unlicensed,
        Method::POST,
        "/api/v1/serviceAccounts",
        serde_json::json!({ "name": "unlicensed" }),
        Some(unlicensed.administrator.clone()),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    drop(unlicensed);

    let fixture = fixture(true).await;
    let marker = Uuid::now_v7().simple().to_string();
    let disabled_id = create_account(&fixture, &format!("disabled-{marker}"), false).await;
    let no_token = send_json(
        &fixture,
        Method::POST,
        &format!("/api/v1/serviceAccounts/{disabled_id}/tokens"),
        serde_json::json!({ "name": "not-issued" }),
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(no_token.status(), StatusCode::NOT_FOUND);

    let active_id = create_account(&fixture, &format!("active-{marker}"), true).await;
    let token = create_token(&fixture, active_id, "self", serde_json::json!({})).await;
    let principal = fixture
        .identity
        .authenticate_bearer(token["token"].as_str().unwrap())
        .await
        .unwrap();
    let human_only = send_json(
        &fixture,
        Method::POST,
        &format!("/api/v1/serviceAccounts/{active_id}/tokens"),
        serde_json::json!({ "name": "nested" }),
        Some(principal),
    )
    .await;
    assert_eq!(human_only.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn concurrent_names_are_case_sensitive_and_exact_duplicates_conflict() {
    let fixture = fixture(true).await;
    let marker = Uuid::now_v7().simple().to_string();
    let lower = format!("runner-{marker}");
    let upper = lower.to_ascii_uppercase();
    let (first, second) = tokio::join!(
        create_account_response(&fixture, &lower, true),
        create_account_response(&fixture, &upper, true),
    );
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);

    let duplicate = format!("duplicate-{marker}");
    let (first, second) = tokio::join!(
        create_account_response(&fixture, &duplicate, true),
        create_account_response(&fixture, &duplicate, true),
    );
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
        .max_connections(6)
        .connect(&database_url)
        .await
        .unwrap();
    let administrator = seed_administrator(&pool).await;
    let identity_store = Arc::new(PostgresIdentityStore::new(pool.clone()));
    let entitlements = Arc::new(StaticEntitlementService::new(custom_access));
    let clock = Arc::new(SystemClock);
    let token_codec = Arc::new(OpaqueServiceAccountTokenCodec);
    let identity = Arc::new(IdentityService::new(
        identity_store,
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(
                &[44_u8; 32],
                "service-accounts-http".to_owned(),
                "service-accounts-http".to_owned(),
            )
            .unwrap(),
        ),
        token_codec.clone(),
        entitlements.clone(),
        clock.clone(),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let service_accounts = Arc::new(ServiceAccountService::new(
        Arc::new(PostgresServiceAccountStore::new(pool.clone())),
        token_codec,
        entitlements,
        clock,
    ));
    let app = service_accounts_http::router(ServiceAccountHttpState {
        identity: identity.clone(),
        service_accounts,
    });
    Fixture {
        app,
        pool,
        administrator,
        identity,
        _guard: guard,
    }
}

async fn seed_administrator(pool: &PgPool) -> ActorPrincipal {
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let name = format!("service-account-admin-{}", user_id.simple());
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name) VALUES ($1, $2, $3, $4, $5, $6)",
    )
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

async fn seed_role(pool: &PgPool, name: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO roles (id, name, roletype) VALUES ($1, $2, 'Custom')")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn seed_team(pool: &PgPool, name: &str) -> Uuid {
    let actor_id = Uuid::now_v7();
    let id = Uuid::now_v7();
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'Team')")
        .bind(actor_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams (id, actorid, name) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(actor_id)
        .bind(name)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    id
}

async fn create_account(fixture: &Fixture, name: &str, enabled: bool) -> Uuid {
    let response = create_account_response(fixture, name, enabled).await;
    assert_eq!(response.status(), StatusCode::OK);
    Uuid::parse_str(json(response).await["id"].as_str().unwrap()).unwrap()
}

async fn create_account_response(fixture: &Fixture, name: &str, enabled: bool) -> Response<Body> {
    send_json(
        fixture,
        Method::POST,
        "/api/v1/serviceAccounts",
        serde_json::json!({ "name": name, "isEnabled": enabled }),
        Some(fixture.administrator.clone()),
    )
    .await
}

async fn create_token(fixture: &Fixture, account_id: Uuid, name: &str, extra: Value) -> Value {
    let mut body = serde_json::json!({ "name": name });
    body.as_object_mut()
        .unwrap()
        .extend(extra.as_object().cloned().unwrap_or_default());
    let response = send_json(
        fixture,
        Method::POST,
        &format!("/api/v1/serviceAccounts/{account_id}/tokens"),
        body,
        Some(fixture.administrator.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    json(response).await
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

async fn send(
    fixture: &Fixture,
    method: Method,
    uri: &str,
    body: Option<Value>,
    principal: Option<ActorPrincipal>,
) -> Response<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    let mut request = builder
        .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
        .unwrap();
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    fixture.app.clone().oneshot(request).await.unwrap()
}

async fn send_json(
    fixture: &Fixture,
    method: Method,
    uri: &str,
    body: Value,
    principal: Option<ActorPrincipal>,
) -> Response<Body> {
    send(fixture, method, uri, Some(body), principal).await
}

async fn json(response: Response<Body>) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
