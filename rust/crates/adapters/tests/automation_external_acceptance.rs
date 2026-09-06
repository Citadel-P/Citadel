//! Real Deno and Shoutrrr execution with PostgreSQL and a local HTTP receiver.
//! Ports the process/result/alert cases of AutomationActionIntegrationTests;
//! unlike the .NET process mock, this gate executes the packaged tools.
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use citadel_adapters::{
    alert_store::{PostgresAlertStore, ShoutrrrAlertDelivery},
    automation_store::PostgresAutomationStore,
};
use citadel_alerts::{
    AlertChannelInput, AlertDeliveryService, AlertEventFilter, AlertRuleInput, AlertStore,
};
use citadel_automation::{
    AutomationActionInput, AutomationError, AutomationRunTokenIssuer, AutomationRuntimeConfig,
    AutomationService, AutomationStore,
};
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const RUN_TOKEN: &str = "fixture-run-token-that-must-not-be-persisted";
struct Tokens(ActorId);
impl AutomationRunTokenIssuer for Tokens {
    fn issue<'a>(
        &'a self,
        actor: ActorId,
        _run: Uuid,
        _lifetime: Duration,
    ) -> BoxFuture<'a, Result<String, AutomationError>> {
        Box::pin(async move {
            assert_eq!(actor, self.0);
            Ok(RUN_TOKEN.into())
        })
    }
}

#[derive(Clone)]
struct Receiver {
    calls: Arc<AtomicUsize>,
    notifications: tokio::sync::mpsc::Sender<String>,
}
async fn echo(headers: HeaderMap) -> Json<Value> {
    assert_eq!(headers["authorization"], format!("Bearer {RUN_TOKEN}"));
    Json(json!({"message":"round-trip"}))
}
async fn notify(State(state): State<Receiver>, body: String) -> StatusCode {
    state.notifications.try_send(body).unwrap();
    if state.calls.fetch_add(1, Ordering::SeqCst) == 0 {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    }
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PHASE7_DATABASE_URL, CITADEL_DENO_PATH and CITADEL_SHOUTRRR_PATH"]
async fn real_automation_execution_persists_results_and_retries_failure_notifications() {
    let database = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    let deno = std::env::var_os("CITADEL_DENO_PATH").expect("real Deno required");
    let shoutrrr = std::env::var_os("CITADEL_SHOUTRRR_PATH").expect("real Shoutrrr required");
    MigrationRunner::migrate(&database).await.unwrap();
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database)
        .await
        .unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor.value())
        .execute(&db)
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (sent, mut notifications) = tokio::sync::mpsc::channel(8);
    let calls = Arc::new(AtomicUsize::new(0));
    let receiver = Router::new()
        .route("/api/v1/echo", get(echo))
        .route("/notifications", post(notify))
        .with_state(Receiver {
            calls: calls.clone(),
            notifications: sent,
        });
    let shutdown = CancellationToken::new();
    let server_stop = shutdown.clone();
    let http = tokio::spawn(async move {
        axum::serve(listener, receiver)
            .with_graceful_shutdown(server_stop.cancelled_owned())
            .await
            .unwrap();
    });
    let root = std::env::temp_dir().join(format!("citadel-deno-{}", Uuid::now_v7()));
    let store = Arc::new(PostgresAutomationStore::new(db.clone()));
    let alerts = Arc::new(PostgresAlertStore::new(db.clone()));
    let service = Arc::new(
        AutomationService::new(
            store.clone(),
            Arc::new(Tokens(actor)),
            AutomationRuntimeConfig {
                deno_path: deno,
                work_root: root.clone(),
                internal_base_url: format!("http://{address}"),
                endpoint_catalog_json: "[]".into(),
                maximum_log_bytes: 4096,
                stale_after: Duration::from_secs(600),
            },
        )
        .with_alerts(alerts.clone()),
    );
    let channel = alerts
        .create_channel(
            actor,
            &AlertChannelInput {
                name: format!("receiver-{actor:?}"),
                alert_destination: "Generic".into(),
                url: format!("generic://{address}/notifications?disabletls=yes"),
                is_active: true,
            },
        )
        .await
        .unwrap();
    let mut rule_input = AlertRuleInput {
        name: format!("failure-{actor:?}"),
        description: None,
        alert_type: "AutomationActionRunFailed".into(),
        severity: "Warning".into(),
        cooldown_seconds: Some(0),
        required_matches: None,
        threshold: None,
        status: "Enabled".into(),
        channel_ids: vec![channel.id],
        limited_to: vec![],
        quiet_hours: vec![],
    };

    for (code, expected, timeout) in [
        (
            "console.log((await citadel.get('/api/v1/echo')).message, args.message); console.log(__citadelToken);",
            "Succeeded",
            10,
        ),
        (
            "console.error('execution failed'); Deno.exit(7);",
            "Failed",
            10,
        ),
        (
            "await new Promise(() => setInterval(() => {}, 1000));",
            "TimedOut",
            1,
        ),
        ("console.log('x'.repeat(100000));", "Succeeded", 10),
        ("await Deno.readTextFile('/etc/passwd');", "Failed", 10),
    ] {
        let mut input = AutomationActionInput {
            name: format!("real-deno-{}", Uuid::now_v7()),
            description: None,
            code: code.into(),
            default_args_json: Some("{}".into()),
            enabled: true,
            schedule_enabled: false,
            schedule_cron: None,
            schedule_time_zone: None,
            webhook: None,
            timeout_seconds: Some(timeout),
            alert_on_failure: expected == "Failed" && code.contains("Deno.exit"),
            run_as_actor_id: None,
            tag_ids: vec![],
        };
        input.validate(actor).unwrap();
        let action = store.create(actor, &input).await.unwrap();
        let rule = if input.alert_on_failure {
            rule_input.limited_to =
                vec![json!({"resourceId":action.id,"resourceType":"AutomationAction"})];
            Some(alerts.create_rule(actor, &rule_input).await.unwrap())
        } else {
            None
        };
        let queued = store
            .enqueue(
                actor,
                action.id,
                "Manual",
                &json!({"message":"arguments"}),
                None,
            )
            .await
            .unwrap();
        assert!(service.process_one(&shutdown).await.unwrap());
        let run = store.get_run(action.id, queued.id).await.unwrap();
        assert_eq!(run.status, expected, "{:?}", run.error_message);
        let events: Vec<(String, String, serde_json::Value)> = sqlx::query_as(
            "SELECT eventtype,status,info::jsonb FROM activityevents WHERE resourceid=$1 ORDER BY createdat,id",
        ).bind(action.id).fetch_all(&db).await.unwrap();
        assert_eq!(
            events.len(),
            4,
            "create, queue, start and one terminal activity"
        );
        assert_eq!(events[2].0, "ActionRunStarted");
        assert_eq!(events[3].0, format!("ActionRun{expected}"));
        assert_eq!(
            events[3].1,
            if expected == "Succeeded" {
                "Success"
            } else {
                "Failure"
            }
        );
        assert_eq!(events[3].2["RunId"], queued.id.to_string());
        assert!(
            !run.logs.as_deref().unwrap_or_default().contains(RUN_TOKEN),
            "raw run token leaked into persisted output"
        );
        assert!(run.logs.as_deref().unwrap_or_default().len() <= 8192 + 128);
        assert!(
            !root.join(queued.id.to_string()).exists(),
            "transient credential-bearing source must be removed"
        );
        if code.contains("citadel.get") {
            assert!(
                run.logs
                    .as_deref()
                    .unwrap()
                    .contains("round-trip arguments")
            );
        }
        if code.contains("100000") {
            assert!(run.logs.as_deref().unwrap().contains("[output truncated]"));
        }
        if code.contains("Deno.exit") {
            assert_eq!(run.exit_code, Some(7));
            let events = alerts
                .list_events(
                    actor,
                    true,
                    &AlertEventFilter {
                        resource_id: Some(action.id),
                        page: 1,
                        page_size: 10,
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
            let rule = rule.as_ref().unwrap();
            assert_eq!(
                events
                    .items
                    .iter()
                    .filter(|event| event.alert_rule_id == rule.id)
                    .count(),
                1
            );
        }
    }

    // Cancellation must kill/reap real Deno and release the durable claim.
    let mut input: AutomationActionInput = serde_json::from_value(json!({
        "name":format!("cancel-deno-{}",Uuid::now_v7()), "code":"await new Promise(() => setInterval(() => {}, 1000));",
        "enabled":true,"scheduleEnabled":false,"alertOnFailure":false,"timeoutSeconds":30
    })).unwrap();
    input.validate(actor).unwrap();
    let action = store.create(actor, &input).await.unwrap();
    let run = store
        .enqueue(actor, action.id, "Manual", &json!({}), None)
        .await
        .unwrap();
    let execution = {
        let service = service.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { service.process_one(&shutdown).await })
    };
    let directory = root.join(run.id.to_string());
    tokio::time::timeout(Duration::from_secs(5), async {
        while !directory.join("action.ts").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    service.cancel(action.id, run.id).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(5), execution)
            .await
            .unwrap()
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        store.get_run(action.id, run.id).await.unwrap().status,
        "Cancelled"
    );
    assert_eq!(store.get(action.id).await.unwrap().control_state, "Idle");
    let cancelled: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='ActionRunCancelled' AND status='Warning'")
        .bind(action.id).fetch_one(&db).await.unwrap();
    assert_eq!(cancelled, 1);
    assert!(!directory.exists());

    let delivery = AlertDeliveryService::new(
        alerts,
        Arc::new(ShoutrrrAlertDelivery::new(
            shoutrrr,
            Duration::from_secs(10),
        )),
    );
    assert!(delivery.process_one(&shutdown).await.unwrap());
    let payload = tokio::time::timeout(Duration::from_secs(3), notifications.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(payload.contains("Deno exited with code 7."));
    assert!(!payload.contains(RUN_TOKEN));
    let row: (String, i32) = sqlx::query_as(
        "SELECT status,attemptcount FROM alertdeliveryoutbox WHERE alertchannelid=$1",
    )
    .bind(channel.id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(row, ("Pending".into(), 1));
    sqlx::query("UPDATE alertdeliveryoutbox SET nextattemptat=now() WHERE alertchannelid=$1")
        .bind(channel.id)
        .execute(&db)
        .await
        .unwrap();
    assert!(delivery.process_one(&shutdown).await.unwrap());
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM alertdeliveryoutbox WHERE alertchannelid=$1")
            .bind(channel.id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(remaining, 0, "successful delivery removes the outbox entry");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    shutdown.cancel();
    http.await.unwrap();
    std::fs::remove_dir(&root).unwrap();
}
