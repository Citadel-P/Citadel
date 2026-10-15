use super::*;
use citadel_resource_copies::*;
use futures_util::future::BoxFuture;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

// Inject transport failures around real PostgreSQL commits. No storage operation
// is replayable: a failed acknowledgement must only retry its durable outcome.
struct FaultyStore {
    inner: PostgresCopyRepository,
    lose_claim: AtomicBool,
    completion_attempts: AtomicUsize,
}
impl CopyRepository for FaultyStore {
    fn targets(
        &self,
        actor: ActorId,
        kind: CopyKind,
        source: Uuid,
    ) -> BoxFuture<'_, Result<CopyTargets, CopyError>> {
        self.inner.targets(actor, kind, source)
    }
    fn prepare_destination<'a>(
        &'a self,
        actor: ActorId,
        plan: &'a mut CopyPlan,
    ) -> BoxFuture<'a, Result<(), CopyError>> {
        self.inner.prepare_destination(actor, plan)
    }
    fn current_data_allowed(
        &self,
        actor: ActorId,
        platform: Uuid,
    ) -> BoxFuture<'_, Result<bool, CopyError>> {
        self.inner.current_data_allowed(actor, platform)
    }
    fn recover_current(&self) -> BoxFuture<'_, Result<(), CopyError>> {
        self.inner.recover_current()
    }
    fn claim_current(
        &self,
        actor: ActorId,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<(CopyPlan, CopyRestore)>, CopyError>> {
        Box::pin(async move {
            let claim = self.inner.claim_current(actor, id).await?;
            if claim.is_some() && self.lose_claim.swap(false, Ordering::SeqCst) {
                return Err(CopyError::Storage(
                    "Lost claim commit acknowledgement".into(),
                ));
            }
            Ok(claim)
        })
    }
    fn finish_current(
        &self,
        actor: ActorId,
        id: Uuid,
        volume: Uuid,
        result: Result<(), String>,
    ) -> BoxFuture<'_, Result<(), CopyError>> {
        Box::pin(async move {
            let attempt = self.completion_attempts.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                return Err(CopyError::Storage(
                    "Completion database temporarily unavailable".into(),
                ));
            }
            self.inner.finish_current(actor, id, volume, result).await?;
            if attempt == 1 {
                return Err(CopyError::Storage(
                    "Lost completion commit acknowledgement".into(),
                ));
            }
            Ok(())
        })
    }
    fn source(
        &self,
        actor: ActorId,
        kind: CopyKind,
        id: Uuid,
    ) -> BoxFuture<'_, Result<CopySource, CopyError>> {
        self.inner.source(actor, kind, id)
    }
    fn snapshots<'a>(
        &'a self,
        actor: ActorId,
        source: &'a CopySource,
    ) -> BoxFuture<'a, Result<Vec<CopySnapshot>, CopyError>> {
        self.inner.snapshots(actor, source)
    }
    fn save_review<'a>(
        &'a self,
        actor: ActorId,
        plan: &'a CopyPlan,
    ) -> BoxFuture<'a, Result<(), CopyError>> {
        self.inner.save_review(actor, plan)
    }
    fn start(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>> {
        self.inner.start(actor, id)
    }
    fn get(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>> {
        self.inner.get(actor, id)
    }
    fn cleanup(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>> {
        self.inner.cleanup(actor, id)
    }
    fn finish_cleanup(
        &self,
        actor: ActorId,
        id: Uuid,
        result: Result<(), String>,
    ) -> BoxFuture<'_, Result<(), CopyError>> {
        self.inner.finish_cleanup(actor, id, result)
    }
    fn cancel(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>> {
        self.inner.cancel(actor, id)
    }
    fn pending(&self) -> BoxFuture<'_, Result<Vec<(ActorId, Uuid)>, CopyError>> {
        self.inner.pending()
    }
    fn plan(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyPlan, CopyError>> {
        self.inner.plan(actor, id)
    }
    fn advance(
        &self,
        actor: ActorId,
        id: Uuid,
        storage_result: Option<Result<(), String>>,
    ) -> BoxFuture<'_, Result<CopyProgress, CopyError>> {
        self.inner.advance(actor, id, storage_result)
    }
}
#[derive(Default)]
struct PausedStorage {
    started: Notify,
    release: Notify,
    executions: AtomicUsize,
    cancelled: AtomicUsize,
}
impl CopyStorage for PausedStorage {
    fn copy_current<'a>(
        &'a self,
        _: &'a CopyPlan,
        _: &'a CopyRestore,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), CopyError>> {
        Box::pin(async move {
            self.executions.fetch_add(1, Ordering::SeqCst);
            self.started.notify_one();
            tokio::select! {
                () = self.release.notified() => Ok(()),
                () = cancel.cancelled() => {
                    self.cancelled.fetch_add(1, Ordering::SeqCst);
                    Err(CopyError::Conflict("Copy cancelled".into()))
                }
            }
        })
    }
    fn check<'a>(
        &'a self,
        _: &'a CopyPlan,
        _: Option<&'a [CopyRestore]>,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), CopyError>> {
        Box::pin(async { Ok(()) })
    }
    fn cleanup<'a>(
        &'a self,
        _: &'a CopyPlan,
        _: &'a [CopyRestore],
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), CopyError>> {
        Box::pin(async { Ok(()) })
    }
}
async fn queue(store: &PostgresCopyRepository, actor: ActorId, source: &CopySource) -> Uuid {
    let request = CopyRequest {
        destination_platform_id: None,
        destination_node_id: None,
        data_mode: CopyDataMode::CurrentData,
        consistency_acknowledged: true,
        kind: source.kind,
        source_id: source.id,
        name: format!("recovery-{}", Uuid::now_v7().simple()),
        selections: vec![],
    };
    let plan = planning::plan(Uuid::now_v7(), source.clone(), request, vec![]).unwrap();
    store.save_review(actor, &plan).await.unwrap();
    store.start(actor, plan.preview.id).await.unwrap();
    plan.preview.id
}
#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn copy_execution_recovers_database_failures_without_blocking_reconciliation() {
    let _fixture = FIXTURE_LOCK.lock().await;
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    let admin = seed_administrator(&pool).await;
    let actor = admin.actor_id;
    let ids = seed_dependencies(&pool, actor).await;
    let deployment = PostgresDeploymentRepository::new(pool.clone()).create(actor, true, &citadel_deployments::CreateDeployment {
        name: format!("recovery-source-{}", Uuid::now_v7().simple()), platform_id: ids.platform,
        description: None, tag_ids: vec![], duplicate_source: None,
        spec: serde_json::from_value(json!({"image":{"$type":"Local","imageId":"fixture"},"volumes":["fixture:/data"]})).unwrap(),
    }).await.unwrap();
    let store = Arc::new(FaultyStore {
        inner: PostgresCopyRepository::new(pool.clone()),
        lose_claim: AtomicBool::new(true),
        completion_attempts: AtomicUsize::new(0),
    });
    let storage = Arc::new(PausedStorage::default());
    let backups = Arc::new(BackupService::new(
        Arc::new(PostgresBackupPersistence::new(pool.clone())),
        Arc::new(FakeBackupExecutor),
        Arc::new(FakeBackupPlanner::default()),
        Duration::minutes(5),
        Arc::new(AllowBackupExecution),
    ));
    let copies = CopyService::new(store.clone(), storage.clone(), backups, Arc::new(|_, _| {}));
    let source = store
        .source(actor, CopyKind::Deployment, deployment.id)
        .await
        .unwrap();
    let cancel = CancellationToken::new();
    let abandoned = queue(&store.inner, actor, &source).await;
    assert!(copies.process_current_queue(&cancel).await.is_err());
    assert_eq!(
        store.get(actor, abandoned).await.unwrap().restores[0].status,
        "Running"
    );
    // No Core restart: the next execution scan fences the ambiguous claim.
    copies.process_current_queue(&cancel).await.unwrap();
    copies.process(&cancel).await.unwrap();
    assert_eq!(
        store.get(actor, abandoned).await.unwrap().status,
        CopyStatus::Failed
    );
    assert_eq!(storage.executions.load(Ordering::SeqCst), 0);

    let active = queue(&store.inner, actor, &source).await;
    let execution = copies.process_current_queue(&cancel);
    let reconcile = async {
        storage.started.notified().await;
        // A different completed parent can finalize while storage is blocked.
        let other = queue(&store.inner, actor, &source).await;
        let (_, job) = store
            .inner
            .claim_current(actor, other)
            .await
            .unwrap()
            .unwrap();
        store
            .inner
            .finish_current(actor, other, job.id, Ok(()))
            .await
            .unwrap();
        copies.cleanup(actor, abandoned).await.unwrap();
        copies.process(&cancel).await.unwrap();
        assert_eq!(
            store.get(actor, other).await.unwrap().status,
            CopyStatus::Succeeded
        );
        assert_eq!(
            store.get(actor, abandoned).await.unwrap().status,
            CopyStatus::Cleaned
        );
        assert_eq!(
            store.get(actor, active).await.unwrap().restores[0].status,
            "Running"
        );
        storage.release.notify_one();
    };
    let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        tokio::join!(execution, reconcile)
    })
    .await
    .expect("Reconciliation or completion retry stalled");
    result.unwrap();
    assert_eq!(storage.executions.load(Ordering::SeqCst), 1);
    assert_eq!(store.completion_attempts.load(Ordering::SeqCst), 3);
    copies.process(&cancel).await.unwrap();
    assert_eq!(
        store.get(actor, active).await.unwrap().status,
        CopyStatus::Succeeded
    );

    // Cancellation reaches the execution lane; shutdown does not detach work.
    let stopping = queue(&store.inner, actor, &source).await;
    let execution = copies.process_current_queue(&cancel);
    let stop = async {
        storage.started.notified().await;
        copies.cancel(actor, stopping).await.unwrap();
    };
    let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(execution, stop)
    })
    .await
    .expect("Cancellation did not reach storage");
    result.unwrap();
    copies.process(&cancel).await.unwrap();
    assert_eq!(
        store.get(actor, stopping).await.unwrap().status,
        CopyStatus::Cancelled
    );
    assert_eq!(storage.cancelled.load(Ordering::SeqCst), 1);

    let shutdown = queue(&store.inner, actor, &source).await;
    let execution = copies.process_current_queue(&cancel);
    let stop = async {
        storage.started.notified().await;
        cancel.cancel();
    };
    let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(execution, stop)
    })
    .await
    .expect("Shutdown left an unowned execution");
    result.unwrap();
    assert_eq!(
        store.get(actor, shutdown).await.unwrap().restores[0].status,
        "Failed"
    );
    assert_eq!(storage.cancelled.load(Ordering::SeqCst), 2);
}
