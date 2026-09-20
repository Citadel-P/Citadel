//! .NET AutomationActionIntegrationTests execution cases over Axum, PostgreSQL
//! and real Deno: progress, permissions, cancellation, outcomes and audit state.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use citadel_adapters::{
    automation_token::IdentityAutomationRunTokenIssuer,
    crypto::{Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec},
    identity_store::{PostgresIdentityStore, StaticEntitlementService},
    postgres::automation::PostgresAutomationRepository,
};
use citadel_automation::{AutomationRepository, AutomationRuntimeConfig, AutomationService};
use citadel_database::MigrationRunner;
use citadel_identity::AuthenticatedPrincipalType;
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, IdentityService, NoopServiceAccountLastUsedTracker,
    SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_primitives::ActorId;
use citadel_server::api::automation::{self as automation_http, AutomationHttpState};
use futures_util::StreamExt;
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;
use uuid::Uuid;

#[path = "automation_http_execution/lifecycle.rs"]
mod lifecycle;

#[path = "automation_http_execution/drafts.rs"]
mod drafts;
#[path = "automation_http_execution/patches.rs"]
mod patches;
#[path = "automation_http_execution/webhooks.rs"]
mod webhooks;

#[tokio::test]
#[ignore = "requires disposable CITADEL_PHASE7_DATABASE_URL and CITADEL_DENO_PATH"]
async fn automation_http_streams_executes_cancels_and_persists_real_process_results() {
    let url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let db = PgPoolOptions::new()
        .max_connections(5)
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
    let root = std::env::temp_dir().join(format!("citadel-automation-http-{}", Uuid::now_v7()));
    let store = Arc::new(PostgresAutomationRepository::new(db.clone()));
    let webhook_license = Arc::new(webhooks::Entitlement::default());
    let automation_shutdown = tokio_util::sync::CancellationToken::new();
    let automation_tasks = citadel_application::DynamicTasks::new(automation_shutdown.clone());

    let service = Arc::new(
        AutomationService::new(
            Arc::new(
                citadel_server::api::automation::TrackedAutomationTasks::new(
                    automation_tasks.clone(),
                ),
            ),
            automation_shutdown.clone(),
            store.clone(),
            Arc::new(IdentityAutomationRunTokenIssuer::new(identity.clone())),
            AutomationRuntimeConfig {
                deno_path: std::env::var_os("CITADEL_DENO_PATH").unwrap(),
                work_root: root.clone(),
                internal_base_url: "http://127.0.0.1:8000".into(),
                endpoint_catalog_json: "[]".into(),
                maximum_log_bytes: 16 * 1024,
                stale_after: Duration::from_secs(600),
            },
        )
        .with_entitlements(webhook_license.clone()),
    );
    let app = automation_http::router(AutomationHttpState {
        identity,
        automation: service.clone(),
    })
    .merge(webhooks::router(db.clone(), service.clone()));
    let admin = actor(&db, true).await;
    let denied = actor(&db, false).await;
    patches::verify(&app, &admin, &denied, &store).await;
    let defaulted = request(&app, Method::POST, "/api/v1/automation/actions", Some(admin.clone()), Some(json!({
        "name":format!("default-timeout-{}",Uuid::now_v7()),"webhook":{},"code":"console.log('ok');","enabled":true,"scheduleEnabled":false,"alertOnFailure":false
    }))).await;
    assert_eq!(defaulted.status(), StatusCode::OK);
    let defaulted = body(defaulted).await;
    assert_eq!(defaulted["timeoutSeconds"], 300);
    assert_eq!(defaulted["webhook"]["enabled"], false);
    assert_eq!(defaulted["webhook"]["provider"], "GitHub");
    assert_eq!(defaulted["webhook"]["authScheme"], "GitHubHmacSha256");
    let oversized = request(&app, Method::POST, "/api/v1/automation/actions", Some(admin.clone()), Some(json!({
        "name":format!("invalid-timeout-{}",Uuid::now_v7()),"code":"console.log('ok');","enabled":true,"scheduleEnabled":false,"alertOnFailure":false,"timeoutSeconds":1801
    }))).await;
    assert_eq!(oversized.status(), StatusCode::BAD_REQUEST);
    webhooks::verify(&app, &admin, &db, &store, &service, &webhook_license).await;
    let action = create(&app, &admin, "console.log('first', args.message); console.log(__citadelToken); await new Promise(r=>setTimeout(r,1500)); console.log('last');", true, 10).await;
    let id = Uuid::parse_str(action["id"].as_str().unwrap()).unwrap();
    let path = format!("/api/v1/automation/actions/{id}/run");
    for (principal, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(denied.clone()), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            request(&app, Method::POST, &path, principal, Some(json!({})))
                .await
                .status(),
            expected
        );
    }
    assert!(store.list_runs(id, 10).await.unwrap().is_empty());
    let response = request(
        &app,
        Method::POST,
        &path,
        Some(admin.clone()),
        Some(json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let mut stream = response.into_body().into_data_stream();
    let mut wire = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(chunk) = stream.next().await {
            wire.extend_from_slice(&chunk.unwrap());
            if String::from_utf8_lossy(&wire).contains("first default") {
                break;
            }
        }
    })
    .await
    .unwrap();
    let running = store.list_runs(id, 1).await.unwrap().remove(0);
    assert_eq!(
        running.status, "Running",
        "output must arrive before process exit"
    );
    // The same resource remains exclusively claimed while its HTTP stream is open.
    let rejected = body(
        request(
            &app,
            Method::POST,
            &path,
            Some(admin.clone()),
            Some(json!({})),
        )
        .await,
    )
    .await;
    assert_eq!(rejected[0]["error"]["code"], 409);
    while let Some(chunk) = stream.next().await {
        wire.extend_from_slice(&chunk.unwrap());
    }
    let progress: Value = serde_json::from_slice(&wire).unwrap();
    assert_eq!(progress[0]["status"], "Queued");
    assert_eq!(
        progress.as_array().unwrap().last().unwrap()["status"],
        "Succeeded"
    );
    assert!(String::from_utf8_lossy(&wire).contains("[redacted]"));
    let run = store.get_run(id, running.id).await.unwrap();
    assert_eq!(run.exit_code, Some(0));
    assert!(run.logs.unwrap().contains("last"));
    assert_eq!(store.get(id).await.unwrap().control_state, "Idle");
    assert!(!root.join(run.id.to_string()).exists());

    for (code, expected, seconds) in [
        ("Deno.exit(7);", "Failed", 10),
        (
            "await new Promise(()=>setInterval(()=>{},1000));",
            "TimedOut",
            1,
        ),
    ] {
        let action = create(&app, &admin, code, true, seconds).await;
        let id = action["id"].as_str().unwrap();
        let result = body(
            request(
                &app,
                Method::POST,
                &format!("/api/v1/automation/actions/{id}/run"),
                Some(admin.clone()),
                Some(json!({})),
            )
            .await,
        )
        .await;
        let terminal = result.as_array().unwrap().last().unwrap();
        assert_eq!(terminal["status"], expected);
        assert!(terminal["error"]["code"].as_i64().unwrap() >= 400);
        let run = store
            .list_runs(Uuid::parse_str(id).unwrap(), 1)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(run.status, expected);
        if expected == "Failed" {
            assert_eq!(run.exit_code, Some(7));
        }
    }

    let latest = body(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/automation/actions/{id}"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(latest["latestRun"]["status"], "Rejected");
    assert!(latest["latestRun"]["codeSnapshot"].is_null());
    assert!(latest["latestRun"]["logs"].is_null());
    assert_eq!(latest["capabilities"]["canExecute"], true);

    let disabled = create(&app, &admin, "console.log('disabled test');", false, 10).await;
    let id = disabled["id"].as_str().unwrap();
    let rejected = body(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/automation/actions/{id}/run"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(rejected[0]["error"]["code"], 409);
    assert!(
        store
            .list_runs(Uuid::parse_str(id).unwrap(), 1)
            .await
            .unwrap()
            .is_empty()
    );
    let tested = body(
        request(
            &app,
            Method::POST,
            &format!("/api/v1/automation/actions/{id}/test"),
            Some(admin.clone()),
            Some(json!({"timeoutSeconds":1})),
        )
        .await,
    )
    .await;
    assert_eq!(
        tested.as_array().unwrap().last().unwrap()["status"],
        "Succeeded"
    );
    assert_eq!(
        store
            .list_runs(Uuid::parse_str(id).unwrap(), 1)
            .await
            .unwrap()[0]
            .timeout_seconds,
        10,
        ".NET Test uses the saved timeout, not a request override"
    );
    let draft_action = create(&app, &admin, "console.log('disabled test');", false, 10).await;
    drafts::verify(&app, &db, &admin, &store, &draft_action).await;

    let action = create(
        &app,
        &admin,
        "console.log('cancel-ready'); await new Promise(()=>setInterval(()=>{},1000));",
        true,
        30,
    )
    .await;
    let id = Uuid::parse_str(action["id"].as_str().unwrap()).unwrap();
    let response = request(
        &app,
        Method::POST,
        &format!("/api/v1/automation/actions/{id}/run"),
        Some(admin.clone()),
        None,
    )
    .await;
    let mut stream = response.into_body().into_data_stream();
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(chunk) = stream.next().await {
            if String::from_utf8_lossy(&chunk.unwrap()).contains("cancel-ready") {
                break;
            }
        }
    })
    .await
    .unwrap();
    let run = store.list_runs(id, 1).await.unwrap().remove(0);
    assert_eq!(run.status, "Running");
    drop(stream);
    tokio::time::timeout(Duration::from_secs(5), async {
        while store.get_run(id, run.id).await.unwrap().status == "Running" {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(store.get_run(id, run.id).await.unwrap().status, "Cancelled");
    assert!(!root.join(run.id.to_string()).exists());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='ActionRunCancelled'").bind(id).fetch_one(&db).await.unwrap();
    assert_eq!(count, 1);
    let stalled = create(
        &app,
        &admin,
        "for(let n=0;;n++){ console.log('progress',n); await new Promise(r=>setTimeout(r,10)); }",
        true,
        30,
    )
    .await;
    let stalled_id = Uuid::parse_str(stalled["id"].as_str().unwrap()).unwrap();
    let response = request(
        &app,
        Method::POST,
        &format!("/api/v1/automation/actions/{stalled_id}/run"),
        Some(admin.clone()),
        Some(json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    // Keeping the HTTP response open without consuming progress must not hold
    // an execution slot indefinitely.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let runs = store.list_runs(stalled_id, 1).await.unwrap();
            if runs[0].status == "Cancelled" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("A stalled progress consumer must be cancelled and persisted");
    drop(response);
    lifecycle::verify_request_drop_during_claim(
        &service,
        &store,
        admin.actor_id,
        Uuid::parse_str(draft_action["id"].as_str().unwrap()).unwrap(),
        &db,
        &automation_tasks,
    )
    .await;
    lifecycle::verify(
        &service,
        &store,
        admin.actor_id,
        Uuid::parse_str(draft_action["id"].as_str().unwrap()).unwrap(),
        &automation_shutdown,
        &automation_tasks,
    )
    .await;
    automation_shutdown.cancel();
    automation_tasks
        .drain(Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(automation_tasks.active(), 0);
    std::fs::remove_dir_all(&root).unwrap();
}

async fn create(
    app: &Router,
    actor: &ActorPrincipal,
    code: &str,
    enabled: bool,
    timeout: i32,
) -> Value {
    let response = request(app, Method::POST, "/api/v1/automation/actions", Some(actor.clone()), Some(json!({
        "name":format!("stream-{}",Uuid::now_v7()),"code":code,"enabled":enabled,"scheduleEnabled":false,"alertOnFailure":false,"timeoutSeconds":timeout,"defaultArgsJson":"{\"message\":\"default\"}"
    }))).await;
    assert_eq!(response.status(), StatusCode::OK);
    body(response).await
}
async fn actor(db: &PgPool, admin: bool) -> ActorPrincipal {
    let id = Uuid::now_v7();
    let user = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(id)
        .execute(db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,email,name) VALUES($1,$2,$3,$4,'Automation HTTP')").bind(user).bind(id).bind(SYSTEM_ACTOR_ID).bind(format!("{user}@example.test")).execute(db).await.unwrap();
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
        name: "Automation HTTP".into(),
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
async fn body(response: axum::response::Response) -> Value {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("{status}: {error}: {}", String::from_utf8_lossy(&bytes)))
}
