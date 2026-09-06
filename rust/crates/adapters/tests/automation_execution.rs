use chrono::{Duration, Timelike, Utc};
use citadel_adapters::automation_store::PostgresAutomationStore;
use citadel_automation::{AutomationActionInput, AutomationRunResult, AutomationStore};
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn automation_claim_is_exclusive_and_interrupted_runs_recover() {
    let database_url = std::env::var("CITADEL_PHASE7_DATABASE_URL")
        .expect("CITADEL_PHASE7_DATABASE_URL is required");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'System')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    let store = PostgresAutomationStore::new(pool.clone());
    let mut input = AutomationActionInput {
        name: format!("phase7-{}", Uuid::now_v7().simple()),
        description: None,
        code: "console.log('ok')".to_owned(),
        default_args_json: Some("{}".to_owned()),
        enabled: true,
        schedule_enabled: false,
        schedule_cron: None,
        schedule_time_zone: None,
        webhook: None,
        timeout_seconds: Some(30),
        alert_on_failure: true,
        run_as_actor_id: None,
        tag_ids: vec![],
    };
    input.validate(actor).unwrap();
    let action = store.create(actor, &input).await.unwrap();
    let created: serde_json::Value = sqlx::query_scalar(
        "SELECT info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='ActionCreated'",
    )
    .bind(action.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(created["Action"]["Name"], input.name);
    let queued = store
        .enqueue(actor, action.id, "Manual", &json!({"safe":true}), None)
        .await
        .unwrap();
    assert!(
        store
            .enqueue(actor, action.id, "Manual", &json!({}), None)
            .await
            .is_err()
    );
    let rejected = store.list_runs(action.id, 10).await.unwrap();
    assert_eq!(
        rejected
            .iter()
            .filter(|run| run.status == "Rejected")
            .count(),
        1
    );
    assert_eq!(
        store.get(action.id).await.unwrap().current_run_id,
        Some(queued.id)
    );
    let claim = store
        .claim_next(Utc::now() - Duration::minutes(10))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claim.run.id, queued.id);
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(10))
            .await
            .unwrap()
            .is_none()
    );
    store
        .finish(
            &claim,
            &AutomationRunResult {
                status: "Succeeded",
                exit_code: Some(0),
                logs: "done".to_owned(),
                error: None,
            },
        )
        .await
        .unwrap();
    let persisted = store.list_runs(action.id, 10).await.unwrap();
    assert_eq!(
        persisted
            .iter()
            .find(|run| run.id == queued.id)
            .unwrap()
            .status,
        "Succeeded"
    );
    // A repeated or late completion must not generate a second activity.
    let committed = store
        .finish(
            &claim,
            &AutomationRunResult {
                status: "Failed",
                exit_code: Some(1),
                logs: "late".into(),
                error: Some("late".into()),
            },
        )
        .await
        .unwrap();
    assert!(
        !committed,
        "late completion must not notify or raise failure alerts"
    );
    let completed: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='ActionRunSucceeded'")
        .bind(action.id).fetch_one(&pool).await.unwrap();
    assert_eq!(completed, 1);

    let interrupted = store
        .enqueue(actor, action.id, "Manual", &json!({}), None)
        .await
        .unwrap();
    let _ = store
        .claim_next(Utc::now() - Duration::minutes(10))
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE actionruns SET startedat=CURRENT_TIMESTAMP-INTERVAL '1 hour' WHERE id=$1")
        .bind(interrupted.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(10))
            .await
            .unwrap()
            .is_none()
    );
    let recovered = store.list_runs(action.id, 10).await.unwrap();
    assert_eq!(recovered[0].status, "Failed");
    assert!(
        recovered[0]
            .error_message
            .as_deref()
            .unwrap()
            .contains("restart")
    );

    sqlx::query("UPDATE actions SET scheduleenabled=true,schedulecron='* * * * *',controlstate='Idle',currentrunid=NULL WHERE id=$1")
        .bind(action.id)
        .execute(&pool)
        .await
        .unwrap();
    let scheduled_minute = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let (first, second) = tokio::join!(
        store.enqueue_scheduled(action.id, scheduled_minute),
        store.enqueue_scheduled(action.id, scheduled_minute)
    );
    assert_eq!(
        usize::from(first.unwrap().is_some()) + usize::from(second.unwrap().is_some()),
        1
    );
    let scheduled = store
        .get_run(
            action.id,
            store.list_runs(action.id, 1).await.unwrap()[0].id,
        )
        .await
        .unwrap();
    assert_eq!(scheduled.trigger, "Schedule");
    store.cancel(action.id, scheduled.id).await.unwrap();
    let events: Vec<String> = sqlx::query_scalar(
        "SELECT eventtype FROM activityevents WHERE resourceid=$1 ORDER BY createdat,id",
    )
    .bind(action.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    for expected in [
        "ActionCreated",
        "ActionRunQueued",
        "ActionRunRejected",
        "ActionRunStarted",
        "ActionRunSucceeded",
        "ActionRunFailed",
        "ActionRunCancelled",
    ] {
        assert!(
            events.iter().any(|event| event == expected),
            "missing {expected}: {events:?}"
        );
    }
    let current = store.get(action.id).await.unwrap();
    input.webhook = Some(json!({"enabled":true,"secret":"do-not-audit"}));
    let updated = store.update(&current, &input, actor, false).await.unwrap();
    assert!(
        store.update(&current, &input, actor, false).await.is_err(),
        "stale edit must not overwrite state"
    );
    let audit: serde_json::Value = sqlx::query_scalar(
        "SELECT info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='ActionUpdated'",
    )
    .bind(action.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit["NewAction"]["Webhook"]["secret"], "********");
    assert!(!audit.to_string().contains("do-not-audit"));
    assert_eq!(updated.webhook.as_ref().unwrap()["secret"], "do-not-audit");

    // Audit persistence failure rolls the resource change back.
    assert!(
        store
            .rename(action.id, "must-rollback", ActorId::new(Uuid::now_v7()))
            .await
            .is_err()
    );
    assert_eq!(store.get(action.id).await.unwrap().name, action.name);

    sqlx::query("DELETE FROM actions WHERE id=$1")
        .bind(action.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
        .bind(action.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
}
