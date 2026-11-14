use super::runtime::script_runs;
use citadel_adapters::persistence::postgres::{
    notifications::PostgresNotifications, scripts::PostgresScriptRepository,
};
use citadel_primitives::ActorId;
use citadel_runtime::{RuntimeSignal, admission::AdmissionGate};
use citadel_scripts::*;
use futures_util::future::BoxFuture;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct Tokens;
impl ScriptRunTokenIssuer for Tokens {
    fn issue<'a>(
        &'a self,
        _: ActorId,
        _: Uuid,
        _: Uuid,
        _: Duration,
    ) -> BoxFuture<'a, Result<String, ScriptError>> {
        Box::pin(async { Ok("fixture-token".into()) })
    }
}

struct Executor {
    started: AtomicUsize,
    complete: Semaphore,
    cleanup: Semaphore,
}
impl Executor {
    fn new() -> Self {
        Self {
            started: AtomicUsize::new(0),
            complete: Semaphore::new(0),
            cleanup: Semaphore::new(0),
        }
    }
    async fn wait_started(&self, count: usize) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while self.started.load(Ordering::SeqCst) < count {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("queue dispatch must not wait for the polling fallback");
    }
}
impl RemoteScriptExecutor for Executor {
    fn execute<'a>(
        &'a self,
        _: &'a ScriptRunClaim,
        _: sandbox::ScriptExecution,
        _: &'a tokio::sync::broadcast::Sender<ScriptProgress>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RemoteScriptOutcome, ScriptError>> {
        Box::pin(async move {
            self.started.fetch_add(1, Ordering::SeqCst);
            tokio::select! {
                () = cancellation.cancelled() => {
                    self.cleanup.acquire().await.unwrap().forget();
                    Ok(ScriptRunResult { status: ScriptRunStatus::Cancelled, exit_code: None, logs: String::new(), error: Some("fixture cancellation".into()) }.into())
                }
                permit = self.complete.acquire() => {
                    permit.unwrap().forget();
                    Ok(ScriptRunResult { status: ScriptRunStatus::Succeeded, exit_code: Some(0), logs: String::new(), error: None }.into())
                }
            }
        })
    }
    fn resolve<'a>(
        &'a self,
        _: &'a ScriptRun,
        _: ActorId,
        _: &'a str,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), ScriptError>> {
        Box::pin(async { Ok(()) })
    }
    fn reconcile<'a>(
        &'a self,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, ScriptError>> {
        Box::pin(async { Ok(false) })
    }
}

async fn fixture(
    options: ScriptOptions,
) -> (
    sqlx::PgPool,
    Arc<PostgresScriptRepository>,
    Arc<ScriptService>,
    Arc<Executor>,
) {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    let store = Arc::new(PostgresScriptRepository::new(pool.clone()));
    let token = CancellationToken::new();
    let tasks = citadel_runtime::DynamicTasks::new(token.clone());
    let executor = Arc::new(Executor::new());
    let service = Arc::new(
        ScriptService::new(
            Arc::new(citadel_processes::SystemProcess),
            Arc::new(
                citadel_adapters::filesystem::script_workspace::LocalScriptWorkspace::new(
                    tasks.clone(),
                ),
            ),
            Arc::new(crate::tasks::scripts::TrackedScriptTasks::new(tasks)),
            token,
            store.clone(),
            Arc::new(Tokens),
            ScriptRuntimeConfig {
                deno_path: "unused-in-dispatch-tests".into(),
                work_root: std::env::temp_dir()
                    .join(format!("citadel-dispatch-{}", Uuid::now_v7())),
                internal_base_url: "http://127.0.0.1:1".into(),
                maximum_log_bytes: 1024,
                stale_after: Duration::from_secs(600),
            },
        )
        .with_options(options)
        .unwrap()
        .with_remote(executor.clone(), "http://127.0.0.1:1".into()),
    );
    (pool, store, service, executor)
}

async fn enqueue(store: &PostgresScriptRepository, executor: ScriptExecutor) -> ScriptRun {
    let actor = ActorId::new(citadel_identity::SYSTEM_ACTOR_ID);
    let mut input = ScriptConfiguration {
        executor,
        duplicate_source_id: None,
        name: format!("dispatch-{}", Uuid::now_v7()),
        description: None,
        code: "console.log('fixture')".into(),
        default_args_json: None,
        enabled: true,
        schedule_enabled: false,
        schedule_cron: None,
        schedule_time_zone: None,
        webhook: None,
        timeout_seconds: Some(30),
        alert_on_failure: false,
        run_as_actor_id: None,
        tag_ids: vec![],
    };
    input.validate(actor).unwrap();
    let script = store.create(actor, &input).await.unwrap();
    store
        .enqueue(actor, script.id, "Manual", &serde_json::json!({}), None)
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn dispatcher_wakes_while_busy_respects_capacity_and_drains_cleanup() {
    let (pool, store, service, executor) = fixture(ScriptOptions {
        max_remote_runs: 2,
        ..ScriptOptions::default()
    })
    .await;
    let token = CancellationToken::new();
    let hub = super::super::notifications::DatabaseNotificationHub::new();
    let listener = PostgresNotifications::connect(&pool, RuntimeSignal::Script.channel())
        .await
        .unwrap();
    let wake = hub.subscribe(RuntimeSignal::Script);
    let notifications = tokio::spawn(hub.run(Box::new(listener), token.clone()));
    let remote = ScriptExecutor::Agent {
        agent_id: Uuid::now_v7(),
    };
    let first = enqueue(&store, remote.clone()).await;
    let admission = AdmissionGate::default();
    let fence = Uuid::now_v7();
    admission.close(fence).unwrap();
    let worker = tokio::spawn(script_runs(admission.clone(), token.clone(), service, wake));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(executor.started.load(Ordering::SeqCst), 0);
    admission.reopen(fence).unwrap();
    executor.wait_started(1).await;
    let second = enqueue(&store, remote.clone()).await;
    executor.wait_started(2).await; // committed notification while first is still running
    let third = enqueue(&store, remote).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(executor.started.load(Ordering::SeqCst), 2);
    assert_eq!(
        store
            .get_run(third.script_id, third.id)
            .await
            .unwrap()
            .status,
        ScriptRunStatus::Queued
    );
    executor.complete.add_permits(1);
    executor.wait_started(3).await; // completion frees a slot without a new notification
    admission.close(fence).unwrap();
    assert!(
        admission
            .drain(fence, Duration::from_millis(20), &token)
            .await
            .is_err()
    );
    token.cancel();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        !worker.is_finished(),
        "shutdown must await executor cleanup"
    );
    executor.cleanup.add_permits(2);
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    notifications.await.unwrap().unwrap();
    let mut completed = 0;
    let mut cancelled = 0;
    for run in [first, second, third] {
        match store.get_run(run.script_id, run.id).await.unwrap().status {
            ScriptRunStatus::Succeeded => completed += 1,
            ScriptRunStatus::Cancelled => cancelled += 1,
            other => panic!("unsettled run: {other:?}"),
        }
    }
    assert_eq!((completed, cancelled), (1, 2));
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn claimed_capacity_is_separate_for_local_and_remote_and_held_until_finalization() {
    let (pool, store, service, _) = fixture(ScriptOptions {
        max_parallel_runs: 1,
        max_remote_runs: 1,
        ..ScriptOptions::default()
    })
    .await;
    let mut runs = Vec::new();
    for _ in 0..2 {
        runs.push(enqueue(&store, ScriptExecutor::Core).await);
        runs.push(
            enqueue(
                &store,
                ScriptExecutor::Agent {
                    agent_id: Uuid::now_v7(),
                },
            )
            .await,
        );
    }
    let remote = service.claim_execution(true).await.unwrap().unwrap();
    assert_eq!(
        store
            .get_run(runs[0].script_id, runs[0].id)
            .await
            .unwrap()
            .status,
        ScriptRunStatus::Queued
    );
    assert_eq!(
        store
            .get_run(runs[1].script_id, runs[1].id)
            .await
            .unwrap()
            .status,
        ScriptRunStatus::Running
    );
    let local = service.claim_execution(false).await.unwrap().unwrap();
    assert!(service.claim_execution(false).await.unwrap().is_none());
    for (run, expected) in runs.iter().zip([
        ScriptRunStatus::Running,
        ScriptRunStatus::Running,
        ScriptRunStatus::Queued,
        ScriptRunStatus::Queued,
    ]) {
        assert_eq!(
            store.get_run(run.script_id, run.id).await.unwrap().status,
            expected
        );
    }
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    service.execute_claimed(local, &cancelled).await.unwrap();
    let next_local = service.claim_execution(false).await.unwrap().unwrap();
    assert!(service.claim_execution(false).await.unwrap().is_none());
    service.execute_claimed(remote, &cancelled).await.unwrap();
    let next_remote = service.claim_execution(false).await.unwrap().unwrap();
    service
        .execute_claimed(next_local, &cancelled)
        .await
        .unwrap();
    service
        .execute_claimed(next_remote, &cancelled)
        .await
        .unwrap();
    for run in runs {
        assert_eq!(
            store.get_run(run.script_id, run.id).await.unwrap().status,
            ScriptRunStatus::Cancelled
        );
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL with pg_stat_statements preloaded"]
async fn idle_dispatch_has_one_probe_per_execution_class_and_no_recovery_transactions() {
    let (pool, _, service, _) = fixture(ScriptOptions::default()).await;
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_stat_statements")
        .execute(&pool)
        .await
        .unwrap();
    async fn calls(pool: &sqlx::PgPool, prefix: &str) -> i64 {
        sqlx::query_scalar("SELECT COALESCE(sum(calls),0)::bigint FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND query LIKE $1")
            .bind(prefix).fetch_one(pool).await.unwrap()
    }
    let probe = "SELECT EXISTS(SELECT % FROM scriptruns WHERE status IN%";
    let recovery = "UPDATE scriptruns SET status=%WHERE status=%";
    let before = calls(&pool, probe).await;
    let recovery_before = calls(&pool, recovery).await;
    let token = CancellationToken::new();
    let (wake, receive) = tokio::sync::watch::channel(());
    let worker = tokio::spawn(script_runs(
        AdmissionGate::default(),
        token.clone(),
        service,
        receive,
    ));
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        calls(&pool, probe).await - before,
        2,
        "execution limits must not multiply idle queries"
    );
    wake.send_modify(|_| {});
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        calls(&pool, probe).await - before,
        4,
        "one notification wakes one dispatcher"
    );
    // No notification: the bounded fallback must still probe the durable queue.
    tokio::time::sleep(Duration::from_secs(30)).await;
    token.cancel();
    worker.await.unwrap().unwrap();
    assert_eq!(calls(&pool, probe).await - before, 6);
    assert_eq!(calls(&pool, recovery).await, recovery_before);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL with pg_stat_statements preloaded"]
async fn running_scripts_use_signals_without_frequent_control_or_payload_reads() {
    let (pool, store, service, executor) = fixture(ScriptOptions::default()).await;
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_stat_statements")
        .execute(&pool)
        .await
        .unwrap();
    async fn reads(pool: &sqlx::PgPool, pattern: &str) -> i64 {
        sqlx::query_scalar("SELECT COALESCE(sum(calls),0)::bigint FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND query LIKE $1")
            .bind(pattern).fetch_one(pool).await.unwrap()
    }
    let initial = enqueue(
        &store,
        ScriptExecutor::Agent {
            agent_id: Uuid::now_v7(),
        },
    )
    .await;
    store.cancel(initial.script_id, initial.id).await.unwrap();
    let actor = ActorId::new(citadel_identity::SYSTEM_ACTOR_ID);
    let mut progress = service
        .run(
            actor,
            initial.script_id,
            "Manual",
            &serde_json::json!({}),
            None,
            None,
        )
        .await
        .unwrap();
    let queued = progress.recv().await.unwrap();
    let id = queued.run_id.unwrap();
    let claim = service.claim_execution(true).await.unwrap().unwrap();
    let running = service.clone();
    let token = CancellationToken::new();
    let worker = tokio::spawn(async move { running.execute_claimed(claim, &token).await });
    executor.wait_started(1).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let control = reads(&pool, "SELECT cancelrequested FROM scriptruns%").await;
    let payload = reads(
        &pool,
        "SELECT *, EXISTS(SELECT % FROM scriptexecutorreservations%",
    )
    .await;
    let status = reads(&pool, "SELECT status FROM scriptruns%").await;
    assert!(
        control > 0 && payload > 0 && status > 0,
        "query patterns must match observed SQL"
    );
    tokio::time::sleep(Duration::from_millis(2200)).await;
    assert_eq!(
        reads(&pool, "SELECT cancelrequested FROM scriptruns%").await,
        control
    );
    assert_eq!(
        reads(
            &pool,
            "SELECT *, EXISTS(SELECT % FROM scriptexecutorreservations%"
        )
        .await,
        payload
    );
    assert_eq!(reads(&pool, "SELECT status FROM scriptruns%").await, status);
    service.cancel(initial.script_id, id).await.unwrap();
    executor.cleanup.add_permits(1);
    tokio::time::timeout(Duration::from_secs(2), worker)
        .await
        .expect("local cancellation must not wait for fallback")
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(event) = progress.recv().await {
            if event.status == Some(ScriptRunStatus::Cancelled) {
                return;
            }
        }
        panic!("missing terminal progress");
    })
    .await
    .unwrap();
    drop(progress);
    // Queued cancellation has no execution task to publish a terminal event.
    let mut progress = service
        .run(
            actor,
            initial.script_id,
            "Manual",
            &serde_json::json!({}),
            None,
            None,
        )
        .await
        .unwrap();
    let id = progress.recv().await.unwrap().run_id.unwrap();
    service.cancel(initial.script_id, id).await.unwrap();
    let event = tokio::time::timeout(Duration::from_secs(2), progress.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.status, Some(ScriptRunStatus::Cancelled));
    drop(progress);
    // A persisted cancellation between claim and local registration must prevent
    // dispatch even when no in-memory token existed at cancellation time.
    let mut progress = service
        .run(
            actor,
            initial.script_id,
            "Manual",
            &serde_json::json!({}),
            None,
            None,
        )
        .await
        .unwrap();
    let id = progress.recv().await.unwrap().run_id.unwrap();
    let claim = service.claim_execution(true).await.unwrap().unwrap();
    store.cancel(initial.script_id, id).await.unwrap();
    service
        .execute_claimed(claim, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(executor.started.load(Ordering::SeqCst), 1);
    assert_eq!(
        store.run_status(initial.script_id, id).await.unwrap(),
        ScriptRunStatus::Cancelled
    );
    drop(progress);
    pool.close().await;
}
