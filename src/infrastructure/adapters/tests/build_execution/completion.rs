//! Fault injection delegates completion to the real PostgreSQL transaction.
use super::*;
use chrono::DateTime;
use citadel_builds::*;
use futures_util::future::BoxFuture;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct Store {
    inner: PostgresBuildRepository,
    pool: sqlx::PgPool,
    mode: usize,
    claims: AtomicUsize,
    finishes: AtomicUsize,
    cleaned: Arc<AtomicBool>,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
}
impl BuildRepository for Store {
    fn health_pools(
        &self,
        after: Uuid,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BuildAgentPool>, BuildError>> {
        self.inner.health_pools(after, limit)
    }
    fn record_pool_health<'a>(
        &'a self,
        pool: &'a BuildAgentPool,
        result: &'a BuildPoolCheck,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        self.inner.record_pool_health(pool, result)
    }
    fn project_permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>, BuildError>,
    > {
        self.inner.project_permissions(actor, ids)
    }
    fn create_pool<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildAgentPoolConfiguration,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        self.inner.create_pool(actor, input)
    }
    fn list_pools(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildAgentPool>, BuildError>> {
        self.inner.list_pools(actor, administrator)
    }
    fn get_pool<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        self.inner.get_pool(id)
    }
    fn pool_permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>, BuildError>,
    > {
        self.inner.pool_permissions(actor, ids)
    }
    fn archive_pool<'a>(
        &'a self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<(), BuildError>> {
        self.inner.archive_pool(id, actor)
    }
    fn update_pool<'a>(
        &'a self,
        current: &'a BuildAgentPool,
        input: &'a BuildAgentPoolConfiguration,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        self.inner.update_pool(current, input, actor)
    }
    fn claim_pool_test(
        &self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'_, Result<BuildAgentPool, BuildError>> {
        self.inner.claim_pool_test(id, actor)
    }
    fn finish_pool_test<'a>(
        &'a self,
        claim: &'a BuildAgentPool,
        result: &'a BuildPoolCheck,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        self.inner.finish_pool_test(claim, result)
    }
    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildProjectConfiguration,
    ) -> BoxFuture<'a, Result<BuildProject, BuildError>> {
        self.inner.create(actor, input)
    }
    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildProject>, BuildError>> {
        self.inner.list(actor, administrator)
    }
    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildProject, BuildError>> {
        self.inner.get(id)
    }
    fn update<'a>(
        &'a self,
        current: &'a BuildProject,
        input: &'a BuildProjectConfiguration,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<BuildProject, BuildError>> {
        self.inner.update(current, input, actor, metadata_only)
    }
    fn archive<'a>(&'a self, id: Uuid, actor: ActorId) -> BoxFuture<'a, Result<(), BuildError>> {
        self.inner.archive(id, actor)
    }
    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
    ) -> BoxFuture<'a, Result<BuildRun, BuildError>> {
        self.inner.enqueue(actor, id, trigger)
    }
    fn enqueue_webhook<'a>(
        &'a self,
        current: &'a BuildProject,
        branch: &'a str,
        commit: Option<&'a str>,
    ) -> BoxFuture<'a, Result<BuildRun, BuildError>> {
        self.inner.enqueue_webhook(current, branch, commit)
    }
    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<BuildClaim>, BuildError>> {
        self.claims.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            let mut claim = self.inner.claim_next(stale_before).await?;
            if self.mode == 6
                && let Some(claim) = &mut claim
            {
                claim.run.timeout_seconds = 1;
            }
            Ok(claim)
        })
    }
    fn finish<'a>(
        &'a self,
        claim: &'a BuildClaim,
        result: &'a BuildExecutionResult,
        observation: Option<&'a citadel_alerts::AlertObservation>,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        Box::pin(async move {
            let attempt = self.finishes.fetch_add(1, Ordering::SeqCst);
            if self.mode == 10 {
                return Err(BuildError::Conflict("definitive fixture rejection".into()));
            }
            self.entered.notify_one();
            assert!(
                self.cleaned.load(Ordering::SeqCst),
                "executor cleanup must precede finish"
            );
            if attempt == 0 && self.mode == 0 {
                self.resume.notified().await;
            }
            if self.mode == 3 || (self.mode == 0 && attempt == 0) {
                return Err(BuildError::Storage("injected before commit".into()));
            }
            if self.mode == 2 && attempt == 0 {
                citadel_adapters::persistence::postgres::maintenance::recover_on_startup(
                    &self.pool,
                )
                .await
                .unwrap();
            }
            if self.mode == 4 && attempt == 0 {
                sqlx::query(
                    "UPDATE buildprojects SET currentrunid=NULL,controlstate='Idle' WHERE id=$1",
                )
                .bind(claim.project.id)
                .execute(&self.pool)
                .await
                .unwrap();
            }
            let completion = self.inner.finish(claim, result, observation).await;
            if self.mode == 9 && attempt == 0 {
                assert!(
                    completion.is_err(),
                    "audit failure must roll back the whole completion"
                );
                assert_eq!(
                    self.inner.get_run(claim.run.id).await.unwrap().status,
                    BuildRunStatus::Preparing
                );
                let queued: i64 = sqlx::query_scalar("SELECT count(*) FROM alertobservations WHERE payload->>'deduplication_component'=$1")
                    .bind(claim.run.id.to_string()).fetch_one(&self.pool).await.unwrap();
                assert_eq!(queued, 0, "observation must roll back with terminal state");
                let ddl = format!(
                    "ALTER TABLE activityevents DROP CONSTRAINT completion_audit_{}",
                    claim.project.id.simple()
                );
                sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
                    .execute(&self.pool)
                    .await
                    .unwrap();
            }
            let committed = completion?;
            if matches!(self.mode, 1 | 7) && attempt == 0 {
                return Err(BuildError::Storage(
                    "injected lost commit acknowledgment".into(),
                ));
            }
            Ok(committed)
        })
    }
    fn list_runs<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        project_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<BuildRun>, BuildError>> {
        self.inner
            .list_runs(actor, administrator, project_id, limit)
    }
    fn get_run<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildRun, BuildError>> {
        self.inner.get_run(id)
    }
    fn logs<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<Vec<BuildLogEntry>, BuildError>> {
        self.inner.logs(id)
    }
    fn append_log<'a>(
        &'a self,
        run_id: Uuid,
        log: &'a BuildLog,
    ) -> BoxFuture<'a, Result<BuildLogEntry, BuildError>> {
        self.inner.append_log(run_id, log)
    }
    fn cancel_queued<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<bool, BuildError>> {
        self.inner.cancel_queued(id)
    }
}

struct Tasks;
impl BuildTaskSpawner for Tasks {
    fn spawn(&self, _: &'static str, _: BoxFuture<'static, Result<(), BuildError>>) -> bool {
        panic!("process_one must remain in its caller's worker slot")
    }
}
struct Alerts;
impl citadel_alerts::AlertEventSink for Alerts {
    fn observe<'a>(
        &'a self,
        _: &'a citadel_alerts::AlertObservation,
    ) -> BoxFuture<'a, Result<Option<citadel_alerts::AlertEvent>, citadel_alerts::AlertError>> {
        panic!("Build completion must enqueue transactionally, never evaluate directly")
    }
}
struct Executor {
    calls: AtomicUsize,
    cleaned: Arc<AtomicBool>,
    started: tokio::sync::Notify,
    mode: usize,
}
impl BuildExecutor for Executor {
    fn execute<'a>(
        &'a self,
        _: &'a BuildClaim,
        _: &'a dyn BuildLogSink,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, BuildExecutionResult> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.started.notify_one();
            if matches!(self.mode, 5 | 6) {
                cancel.cancelled().await;
            }
            // Represents the executor's awaited subprocess/staging cleanup.
            tokio::task::yield_now().await;
            self.cleaned.store(true, Ordering::SeqCst);
            let mut result = super::success_result('a');
            if self.mode != 7 {
                result.status = BuildRunStatus::Failed;
                result.error_message = Some("fixture failure".into());
            }
            result
        })
    }
}

pub(super) async fn verify(pool: &sqlx::PgPool, actor: ActorId, input: &BuildProjectConfiguration) {
    // 0: precommit; 1: lost ack; 2: recovery; 3: pending/shutdown;
    // 4: superseded currentrunid; 5: execution shutdown; 6: timeout cleanup;
    // 7: consumer queue lost ack; 8: alerts absent; 9: transactional rollback;
    // 10: definitive domain error (must not retry).
    for mode in 0..11 {
        let cleaned = Arc::new(AtomicBool::new(false));
        let store = Arc::new(Store {
            inner: PostgresBuildRepository::new(pool.clone()),
            pool: pool.clone(),
            mode,
            claims: AtomicUsize::new(0),
            finishes: AtomicUsize::new(0),
            cleaned: cleaned.clone(),
            entered: tokio::sync::Notify::new(),
            resume: tokio::sync::Notify::new(),
        });
        let mut input = input.clone();
        input.name = format!("completion-{mode}-{}", Uuid::now_v7());
        let project = store.create(actor, &input).await.unwrap();
        let consumer = if mode == 7 {
            use citadel_deployments::DeploymentRepository;
            let input = citadel_deployments::CreateDeployment {
                name: format!("completion-consumer-{}", Uuid::now_v7()),
                platform_id: input.platform_id.unwrap(), description: None,
                spec: serde_json::from_value(serde_json::json!({"image":{"$type":"Build","buildProjectId":project.id,"redeployOnBuild":false}})).unwrap(),
                tag_ids: vec![], duplicate_source: None,
            };
            Some(citadel_adapters::persistence::postgres::deployments::PostgresDeploymentRepository::new(pool.clone()).create(actor, true, &input).await.unwrap().id)
        } else {
            None
        };
        let run = store.enqueue(actor, project.id, "Manual").await.unwrap();
        if mode == 9 {
            let ddl = format!(
                "ALTER TABLE activityevents ADD CONSTRAINT completion_audit_{} CHECK (resourceid <> '{}' OR eventtype <> 'BuildRunFailed') NOT VALID",
                project.id.simple(),
                project.id
            );
            sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
                .execute(pool)
                .await
                .unwrap();
        }

        let executor = Arc::new(Executor {
            calls: AtomicUsize::new(0),
            cleaned: cleaned.clone(),
            started: tokio::sync::Notify::new(),
            mode,
        });
        let shutdown = CancellationToken::new();
        let mut service = BuildService::new(
            Arc::new(Tasks),
            shutdown.clone(),
            store.clone(),
            executor.clone(),
            Duration::minutes(5),
        );
        if mode != 8 {
            service = service.with_alerts(Arc::new(Alerts));
        }
        let changes = Arc::new(AtomicUsize::new(0));
        let observed = changes.clone();
        service = service.with_change_notifier(Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        }));
        let observer = async {
            if matches!(mode, 0 | 3) {
                store.entered.notified().await;
                assert_eq!(
                    store.get_run(run.id).await.unwrap().status,
                    BuildRunStatus::Preparing
                );
                assert_eq!(
                    changes.load(Ordering::SeqCst),
                    1,
                    "no terminal hint before commit"
                );
                assert_eq!(
                    store.claims.load(Ordering::SeqCst),
                    1,
                    "pending completion retains the slot"
                );
                if mode == 3 {
                    shutdown.cancel();
                }
                store.resume.notify_one();
            } else if mode == 5 {
                executor.started.notified().await;
                shutdown.cancel();
            }
        };
        let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            tokio::join!(service.process_one(&shutdown), observer)
        })
        .await
        .expect("bounded completion/shutdown");
        if mode == 10 {
            assert!(matches!(result, Err(BuildError::Conflict(_))));
        } else {
            assert!(result.unwrap());
        }
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
        assert!(cleaned.load(Ordering::SeqCst));
        assert_eq!(store.claims.load(Ordering::SeqCst), 1);
        let persisted = store.get_run(run.id).await.unwrap();
        let expected = match mode {
            2 => BuildRunStatus::Interrupted,
            3..=5 | 10 => BuildRunStatus::Preparing,
            6 => BuildRunStatus::TimedOut,
            7 => BuildRunStatus::Succeeded,
            _ => BuildRunStatus::Failed,
        };
        assert_eq!(persisted.status, expected, "mode {mode}");
        if matches!(mode, 0 | 1 | 7 | 8 | 9) {
            assert_eq!(
                persisted.image_digest.as_deref(),
                Some(format!("sha256:{}", "a".repeat(64)).as_str())
            );
            assert_eq!(
                persisted.image_references,
                super::success_result('a').image_references
            );
        }
        if matches!(mode, 0 | 1) {
            assert_eq!(store.finishes.load(Ordering::SeqCst), 2);
        }
        let observations: Vec<serde_json::Value> = sqlx::query_scalar(
            "SELECT payload FROM alertobservations WHERE payload->>'deduplication_component'=$1",
        )
        .bind(run.id.to_string())
        .fetch_all(pool)
        .await
        .unwrap();
        let expected_alert = matches!(mode, 0 | 1 | 6 | 9);
        assert_eq!(
            observations.len(),
            usize::from(expected_alert),
            "mode {mode}"
        );
        if expected_alert {
            assert_eq!(observations[0]["info"]["Status"], expected.as_str());
        }
        let completions: i64 =
            sqlx::query_scalar("SELECT count(*) FROM buildcompletionqueue WHERE buildrunid=$1")
                .bind(run.id)
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(completions, i64::from(mode == 7));
        let activities: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype IN ('BuildRunSucceeded','BuildRunFailed','BuildRunTimedOut','BuildRunInterrupted')")
            .bind(project.id).fetch_one(pool).await.unwrap();
        assert_eq!(activities, i64::from(expected.is_terminal()));
        if let Some(consumer) = consumer {
            sqlx::query("DELETE FROM deployments WHERE id=$1")
                .bind(consumer)
                .execute(pool)
                .await
                .unwrap();
        }
        sqlx::query("DELETE FROM buildcompletionqueue WHERE buildrunid=$1")
            .bind(run.id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM alertobservations WHERE payload->>'deduplication_component'=$1")
            .bind(run.id.to_string())
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
            .bind(project.id)
            .execute(pool)
            .await
            .unwrap();
        // Fixture-only cleanup of unresolved claims; the real recovery owns crashes.
        sqlx::query("DELETE FROM buildruns WHERE buildprojectid=$1")
            .bind(project.id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM buildprojects WHERE id=$1")
            .bind(project.id)
            .execute(pool)
            .await
            .unwrap();
    }
}
