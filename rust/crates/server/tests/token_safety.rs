//! Exercise credential restrictions with real signed/opaque tokens and websocket messages.
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::Extension,
    http::{Request, StatusCode},
    middleware,
};
use citadel_adapters::{
    crypto::{Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec},
    identity_store::{PostgresIdentityStore, StaticEntitlementService},
};
use citadel_database::MigrationRunner;
use citadel_domain::{ActorId, AuthenticatedPrincipalType};
use citadel_identity::{
    AccessTokenClaims, ActorPrincipal, IdentityService, NoopServiceAccountLastUsedTracker,
    SYSTEM_ACTOR_ID, ServiceAccountTokenCodec, SessionTokenCodec, SystemClock,
};
use citadel_server::{
    identity_http::authentication_middleware,
    license_realtime::{LicenseRealtimeHub, LicenseRealtimeService},
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{sync::Arc, time::Duration};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires a disposable CITADEL_TOKEN_SAFETY_DATABASE_URL"]
async fn http_and_websocket_guards_reject_restricted_credentials() {
    let url = std::env::var("CITADEL_TOKEN_SAFETY_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(codec()),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        chrono::Duration::minutes(15),
        chrono::Duration::days(30),
    ));
    let (user, user_actor) = seed(&pool, false).await;
    let (account, account_actor) = seed(&pool, true).await;
    let human = jwt(user, user_actor, AuthenticatedPrincipalType::User, false);
    let run = jwt(user, user_actor, AuthenticatedPrincipalType::User, true);
    let account_run = jwt(
        account,
        account_actor,
        AuthenticatedPrincipalType::ServiceAccount,
        true,
    );
    let credential = Uuid::now_v7();
    let (account_token, hash) = OpaqueServiceAccountTokenCodec.issue(credential).unwrap();
    sqlx::query("INSERT INTO serviceaccounttokens (id,serviceaccountid,name,secrethash,expiresatutc,createdbyactorid,createdatutc) VALUES ($1,$2,'fixture',$3,now()+interval '1 hour',$4,now())")
        .bind(credential).bind(account).bind(hash.as_slice()).bind(SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    for token in [&human, &run, &account_token, &account_run] {
        identity
            .authenticate_bearer_context(token)
            .await
            .expect("fixture credential is valid");
    }
    let app = Router::new()
        .fallback(|principal: Option<Extension<ActorPrincipal>>| async move {
            if principal.is_some() {
                StatusCode::OK
            } else {
                StatusCode::UNAUTHORIZED
            }
        })
        .layer(middleware::from_fn_with_state(
            identity.clone(),
            authentication_middleware,
        ));
    for token in [&account_token, &account_run] {
        for path in [
            "/api/v1/authentication/login",
            "/api/v1/profile/sessions",
            "/hubs/citadel",
            "/API/V1/PROFILE",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
            assert_eq!(
                response.headers()["content-type"],
                "application/problem+json"
            );
            let body: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap())
                    .unwrap();
            assert_eq!(body["status"], 403);
        }
    }
    for token in [&run, &account_run] {
        for path in [
            "/api/v1/automation/actions",
            "/api/v1/resourceBindings/secrets",
            "/api/v1/platforms/id/containers/id/terminal",
        ] {
            assert_eq!(status(&app, token, path).await, StatusCode::FORBIDDEN);
        }
    }
    assert_eq!(
        status(&app, &human, "/api/v1/profile").await,
        StatusCode::OK
    );
    for token in [&human, &run, &account_token, &account_run] {
        assert_eq!(
            status(&app, token, "/api/v1/stacks/id/apply").await,
            StatusCode::OK
        );
    }
    let shutdown = CancellationToken::new();
    let realtime =
        LicenseRealtimeService::new(identity, LicenseRealtimeHub::default(), shutdown.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn({
        let shutdown = shutdown.clone();
        async move {
            axum::serve(listener, realtime.router())
                .with_graceful_shutdown(shutdown.cancelled_owned())
                .await
                .unwrap();
        }
    });
    // Supply credentials in the first websocket message, without an HTTP bearer header.
    for (token, allowed) in [
        (&human, true),
        (&run, false),
        (&account_token, false),
        (&account_run, false),
    ] {
        let (mut socket, _) =
            tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
                .await
                .unwrap();
        socket
            .send(Message::Text(
                json!({"protocolVersion":1,"kind":"subscribe","accessToken":token})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        let message = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .unwrap();
        if allowed {
            let Some(Ok(Message::Text(text))) = message else {
                panic!("normal user subscription rejected")
            };
            assert_eq!(
                serde_json::from_str::<Value>(&text).unwrap()["kind"],
                "subscribed"
            );
            socket.close(None).await.unwrap();
        } else {
            assert!(
                matches!(message, Some(Ok(Message::Close(_))) | None),
                "restricted credential received websocket data"
            );
        }
    }
    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    pool.close().await;
}

async fn status(app: &Router, token: &str, path: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}
fn codec() -> JwtSessionTokenCodec {
    JwtSessionTokenCodec::new(&[57; 32], "token-safety".into(), "token-safety".into()).unwrap()
}
fn jwt(subject: Uuid, actor: Uuid, kind: AuthenticatedPrincipalType, run: bool) -> String {
    let now = chrono::Utc::now();
    codec()
        .encode_access(&AccessTokenClaims {
            subject_id: subject,
            actor_id: ActorId::new(actor),
            principal_type: kind,
            issued_at: now,
            expires_at: now + chrono::Duration::minutes(5),
            automation_run_id: run.then(Uuid::now_v7),
        })
        .unwrap()
}
async fn seed(pool: &PgPool, account: bool) -> (Uuid, Uuid) {
    let actor = Uuid::now_v7();
    let subject = Uuid::now_v7();
    let name = format!("fixture-{subject}");
    sqlx::query("INSERT INTO actors (id,isenabled,type) VALUES ($1,true,$2)")
        .bind(actor)
        .bind(if account { "ServiceAccount" } else { "User" })
        .execute(pool)
        .await
        .unwrap();
    let sql = if account {
        "INSERT INTO serviceaccounts (id,actorid,name,createdbyactorid,createdat,updatedat) VALUES ($1,$2,$3,$4,now(),now())"
    } else {
        "INSERT INTO users (id,actorid,name,createdbyactorid,createdat,email) VALUES ($1,$2,$3,$4,now(),$3 || '@example.test')"
    };
    sqlx::query(sql)
        .bind(subject)
        .bind(actor)
        .bind(name)
        .bind(SYSTEM_ACTOR_ID)
        .execute(pool)
        .await
        .unwrap();
    (subject, actor)
}
