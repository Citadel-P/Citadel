use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, containers::*};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Default)]
struct Store {
    claimed: AtomicUsize,
    finished: AtomicUsize,
}
impl ContainerRepository for Store {
    fn resolve_ids<'a>(
        &'a self,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Uuid>, RuntimeCapabilityError>> {
        Box::pin(async move { Ok(ids.iter().map(|id| Uuid::parse_str(id).unwrap()).collect()) })
    }
    fn claim_selection<'a>(
        &'a self,
        _: ActorId,
        _: bool,
        ids: &'a [Uuid],
        _: ContainerAction,
        _: ContainerSelectionKind,
    ) -> BoxFuture<'a, Result<ContainerClaim, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.claimed.fetch_add(1, Ordering::SeqCst);
            Ok(ContainerClaim {
                operation_id: Uuid::now_v7(),
                started_at: chrono::Utc::now().timestamp(),
                targets: ids
                    .iter()
                    .map(|id| ContainerTarget {
                        id: *id,
                        platform_id: Uuid::nil(),
                        docker_id: "docker-id".into(),
                        node_id: None,
                    })
                    .collect(),
                deployment_ids: vec![],
                stack_ids: vec![],
            })
        })
    }
    fn observed<'a>(
        &'a self,
        _: Uuid,
        _: &'a ContainerTarget,
        _: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async { Ok(()) })
    }
    fn finish(&self, _: Uuid) -> BoxFuture<'_, Result<(), RuntimeCapabilityError>> {
        Box::pin(async {
            self.finished.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
    fn abandon(&self, _: Uuid) -> BoxFuture<'_, Result<(), RuntimeCapabilityError>> {
        Box::pin(async { panic!("unexpected abandon") })
    }
    fn stale(&self) -> BoxFuture<'_, Result<Vec<ContainerClaim>, RuntimeCapabilityError>> {
        Box::pin(async { Ok(vec![]) })
    }
}
struct Runtime {
    started: Semaphore,
    release: Semaphore,
}
impl Runtime {
    fn new() -> Self {
        Self {
            started: Semaphore::new(0),
            release: Semaphore::new(0),
        }
    }
}
impl ContainerMutationRuntime for Runtime {
    fn mutate<'a>(
        &'a self,
        _: &'a ContainerTarget,
        _: ContainerAction,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.started.add_permits(1);
            self.release.acquire().await.unwrap().forget();
            Ok(())
        })
    }
    fn observe<'a>(
        &'a self,
        _: &'a ContainerTarget,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, RuntimeCapabilityError>> {
        Box::pin(async { Ok(Some("running".into())) })
    }
}

#[tokio::test]
async fn disconnect_does_not_abandon_claim_and_overload_does_not_spawn_waiters() {
    let store = Arc::new(Store::default());
    let runtime = Arc::new(Runtime::new());
    let service = ContainerMutationService::new(store.clone(), runtime.clone(), Arc::new(Tasks));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let service = service.clone();
            tokio::spawn(async move {
                service
                    .execute(
                        ActorId::new(Uuid::now_v7()),
                        true,
                        vec![Uuid::now_v7().to_string()],
                        ContainerAction::Start,
                    )
                    .await
            })
        })
        .collect();
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        runtime.started.acquire_many(4),
    )
    .await
    .unwrap()
    .unwrap()
    .forget();
    assert!(
        matches!(service.execute(ActorId::new(Uuid::now_v7()),true,vec![Uuid::now_v7().to_string()],ContainerAction::Stop).await,Err(e) if e.kind==RuntimeErrorKind::ResourceExhausted)
    );
    for handle in handles {
        handle.abort();
    }
    runtime.release.add_permits(4);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while store.finished.load(Ordering::SeqCst) != 4 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(store.claimed.load(Ordering::SeqCst), 4);
}

#[tokio::test(start_paused = true)]
async fn timeout_keeps_unknown_outcome_for_read_only_recovery() {
    let store = Arc::new(Store::default());
    let runtime = Arc::new(Runtime::new());
    let service = ContainerMutationService::new(store.clone(), runtime.clone(), Arc::new(Tasks));
    let task = tokio::spawn(async move {
        service
            .execute(
                ActorId::new(Uuid::now_v7()),
                true,
                vec![Uuid::now_v7().to_string()],
                ContainerAction::Restart,
            )
            .await
    });
    runtime.started.acquire().await.unwrap().forget();
    tokio::time::advance(CONTAINER_OPERATION_TIMEOUT + std::time::Duration::from_secs(1)).await;
    assert!(matches!(task.await.unwrap(),Err(e) if e.kind==RuntimeErrorKind::Timeout));
    assert_eq!(store.claimed.load(Ordering::SeqCst), 1);
    assert_eq!(store.finished.load(Ordering::SeqCst), 0);
}

struct Tasks;
impl ContainerTaskSpawner for Tasks {
    fn spawn(&self, operation: BoxFuture<'static, Result<(), RuntimeCapabilityError>>) -> bool {
        tokio::spawn(operation);
        true
    }
    fn shutdown_token(&self) -> CancellationToken {
        CancellationToken::new()
    }
}

struct ClosedTasks;
impl ContainerTaskSpawner for ClosedTasks {
    fn spawn(&self, _: BoxFuture<'static, Result<(), RuntimeCapabilityError>>) -> bool {
        false
    }
    fn shutdown_token(&self) -> CancellationToken {
        CancellationToken::new()
    }
}

#[tokio::test]
async fn closed_process_admission_creates_no_claim() {
    let store = Arc::new(Store::default());
    let service = ContainerMutationService::new(
        store.clone(),
        Arc::new(Runtime::new()),
        Arc::new(ClosedTasks),
    );
    let result = service
        .execute(
            ActorId::new(Uuid::now_v7()),
            true,
            vec![Uuid::now_v7().to_string()],
            ContainerAction::Start,
        )
        .await;
    assert!(matches!(result, Err(error) if error.kind == RuntimeErrorKind::ResourceExhausted));
    assert_eq!(store.claimed.load(Ordering::SeqCst), 0);
}
