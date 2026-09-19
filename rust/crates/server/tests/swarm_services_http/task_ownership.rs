use citadel_swarm_services::SwarmServiceRepository;
#[path = "../support/workload_queries.rs"]
mod query_counts;
use super::*;
use citadel_application::DynamicTasks;
use citadel_server::api::swarm_services::TrackedSwarmServiceTasks;

pub(super) async fn verify(pool: &sqlx::PgPool, actor: ActorId, id: Uuid) {
    let shutdown = CancellationToken::new();
    let owner = DynamicTasks::new(shutdown.clone());
    let repository = Arc::new(PostgresSwarmServiceRepository::new(pool.clone()));
    let before = query_counts::count(pool).await;
    let listed = repository
        .list_authorized(actor, true, &Default::default())
        .await
        .unwrap();
    assert!(!listed.is_empty());
    query_counts::assert_single_query(pool, before).await;
    let service = SwarmServiceService::new(
        Arc::new(TrackedSwarmServiceTasks::new(owner.clone())),
        repository.clone(),
        Arc::new(CompletingRuntime),
        shutdown,
    );
    let progress = service.apply(actor, true, id);
    assert_eq!(owner.active(), 1);
    drop(progress);
    owner
        .drain(std::time::Duration::from_secs(5))
        .await
        .unwrap();
    let applied = repository.get_authorized(actor, true, id).await.unwrap();
    assert_eq!(
        applied.current_operation.as_ref().unwrap().state,
        "Completed"
    );
    assert_eq!(applied.control_state, "Idle");
    let version = applied.row_version;
    // Service claims live inside the accepted task, so rejection never creates
    // an operation or consumes a semaphore permit permanently.
    for _ in 0..5 {
        let mut progress = service.apply(actor, true, id);
        let failure = progress.recv().await.unwrap();
        assert!(failure.message.contains("shutting down"));
        assert!(progress.recv().await.is_none());
    }
    assert_eq!(
        repository
            .get_authorized(actor, true, id)
            .await
            .unwrap()
            .row_version,
        version
    );

    let shutdown = CancellationToken::new();
    let owner = DynamicTasks::new(shutdown.clone());
    let runtime = Arc::new(CancellingRuntime(tokio::sync::Notify::new()));
    let cancelling = SwarmServiceService::new(
        Arc::new(TrackedSwarmServiceTasks::new(owner.clone())),
        repository.clone(),
        runtime.clone(),
        shutdown.clone(),
    );
    let progress = cancelling.apply(actor, true, id);
    tokio::time::timeout(std::time::Duration::from_secs(5), runtime.0.notified())
        .await
        .unwrap();
    drop(progress);
    shutdown.cancel();
    owner
        .drain(std::time::Duration::from_secs(5))
        .await
        .unwrap();
    let unknown = repository.get_authorized(actor, true, id).await.unwrap();
    assert_eq!(unknown.control_state, "Idle");
    assert_eq!(unknown.current_operation.unwrap().state, "OutcomeUnknown");
}

struct CancellingRuntime(tokio::sync::Notify);
impl SwarmServiceRuntime for CancellingRuntime {
    fn apply<'a>(
        &'a self,
        _: &'a ServiceOperationClaim,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>> {
        Box::pin(async move {
            self.0.notify_one();
            cancel.cancelled().await;
            Err(SwarmServiceError::Cancelled)
        })
    }
    fn scale<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        _: i32,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>> {
        self.apply(claim, cancel)
    }
    fn force_update<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>> {
        self.apply(claim, cancel)
    }
    fn delete<'a>(
        &'a self,
        platform: Uuid,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        CompletingRuntime.delete(platform, id, cancel)
    }
    fn observe<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<RuntimeServiceResult>, SwarmServiceError>> {
        CompletingRuntime.observe(claim, cancel)
    }
}
