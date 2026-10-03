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
    persisted: AtomicUsize,
    batches: AtomicUsize,
    superseded: AtomicUsize,
    coordinator: Arc<ContainerOperationCoordinator>,
    active_claim: std::sync::Mutex<Option<ContainerClaim>>,
}
impl ContainerRepository for Store {
    fn coordinator(&self) -> Option<Arc<ContainerOperationCoordinator>> {
        Some(self.coordinator.clone())
    }
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
            let claim = ContainerClaim {
                operation_id: Uuid::now_v7(),
                started_at: chrono::Utc::now().timestamp(),
                targets: ids
                    .iter()
                    .map(|id| ContainerTarget {
                        id: *id,
                        platform_id: Uuid::nil(),
                        docker_id: format!("docker-{id}"),
                        node_id: None,
                    })
                    .collect(),
                deployment_ids: vec![],
                stack_ids: vec![],
            };
            *self.active_claim.lock().unwrap() = Some(claim.clone());
            Ok(claim)
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
    fn observed_batch<'a>(
        &'a self,
        _: Uuid,
        observations: &'a [ContainerObservation],
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.batches.fetch_add(1, Ordering::SeqCst);
            if self
                .superseded
                .compare_exchange(1, 0, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Conflict,
                    "superseded inspection",
                    true,
                ));
            }
            assert!(
                observations.iter().all(|o| o.generation.is_some()),
                "inspection fences must be captured before runtime reads"
            );
            self.persisted
                .fetch_add(observations.len(), Ordering::SeqCst);
            Ok(())
        })
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

struct BatchRuntime {
    fail: bool,
    batches: AtomicUsize,
    observations: AtomicUsize,
    unavailable: Option<Uuid>,
}
impl ContainerMutationRuntime for BatchRuntime {
    fn mutate<'a>(
        &'a self,
        _: &'a ContainerTarget,
        _: ContainerAction,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async { panic!("selection must use the batch operation") })
    }
    fn mutate_batch<'a>(
        &'a self,
        targets: &'a [ContainerTarget],
        _: ContainerAction,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            assert_eq!(targets.len(), 6);
            self.batches.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Remote,
                    "partial remote failure",
                    false,
                ))
            } else {
                Ok(())
            }
        })
    }
    fn observe<'a>(
        &'a self,
        target: &'a ContainerTarget,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.observations.fetch_add(1, Ordering::SeqCst);
            if self.unavailable == Some(target.id) {
                return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Unavailable,
                    "inspection unavailable",
                    false,
                ));
            }
            Ok(Some("exited".into()))
        })
    }
}

#[tokio::test]
async fn bulk_mutation_verifies_all_targets_and_retains_claims_after_partial_failure() {
    for fail in [false, true] {
        let store = Arc::new(Store::default());
        let runtime = Arc::new(BatchRuntime {
            fail,
            batches: AtomicUsize::new(0),
            observations: AtomicUsize::new(0),
            unavailable: None,
        });
        let service =
            ContainerMutationService::new(store.clone(), runtime.clone(), Arc::new(Tasks));
        let result = service
            .execute(
                ActorId::new(Uuid::now_v7()),
                true,
                (0..6).map(|_| Uuid::now_v7().to_string()).collect(),
                ContainerAction::Stop,
            )
            .await;
        assert_eq!(result.is_err(), fail);
        assert_eq!(runtime.batches.load(Ordering::SeqCst), 1);
        assert_eq!(runtime.observations.load(Ordering::SeqCst), 6);
        assert_eq!(store.persisted.load(Ordering::SeqCst), 6);
        assert_eq!(store.batches.load(Ordering::SeqCst), 1);
        assert_eq!(store.finished.load(Ordering::SeqCst), usize::from(!fail));
    }
}

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

#[tokio::test]
async fn partial_verification_persists_successful_siblings_in_one_batch_and_retains_claim() {
    let store = Arc::new(Store::default());
    let ids: Vec<_> = (0..6).map(|_| Uuid::now_v7()).collect();
    let runtime = Arc::new(BatchRuntime {
        fail: false,
        batches: AtomicUsize::new(0),
        observations: AtomicUsize::new(0),
        unavailable: Some(ids[0]),
    });
    let service = ContainerMutationService::new(store.clone(), runtime, Arc::new(Tasks));
    let result = service
        .execute(
            ActorId::new(Uuid::now_v7()),
            true,
            ids.iter().map(ToString::to_string).collect(),
            ContainerAction::Stop,
        )
        .await;
    assert!(
        matches!(result,Err(error) if error.kind==RuntimeErrorKind::Unavailable && error.message.contains(&ids[0].to_string()))
    );
    assert_eq!(store.persisted.load(Ordering::SeqCst), 5);
    assert_eq!(store.batches.load(Ordering::SeqCst), 1);
    assert_eq!(store.finished.load(Ordering::SeqCst), 0);
}

struct ConfirmingRuntime {
    store: Arc<Store>,
    events: usize,
    inspections: AtomicUsize,
}
impl ContainerMutationRuntime for ConfirmingRuntime {
    fn mutate<'a>(
        &'a self,
        _: &'a ContainerTarget,
        _: ContainerAction,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async { unreachable!("uses batch mutation") })
    }
    fn mutate_batch<'a>(
        &'a self,
        targets: &'a [ContainerTarget],
        action: ContainerAction,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            let claim = self.store.active_claim.lock().unwrap().clone().unwrap();
            let state = match action {
                ContainerAction::Stop | ContainerAction::Delete(_) => "exited",
                ContainerAction::Pause => "paused",
                _ => "running",
            };
            for target in targets.iter().take(self.events) {
                self.store.coordinator.committed(
                    claim.operation_id,
                    target,
                    Some(state),
                    chrono::Utc::now().timestamp_millis(),
                );
            }
            Ok(())
        })
    }
    fn observe<'a>(
        &'a self,
        _: &'a ContainerTarget,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.inspections.fetch_add(1, Ordering::SeqCst);
            Ok(Some("exited".into()))
        })
    }
}

#[tokio::test]
async fn lifecycle_events_confirm_targets_and_only_missing_targets_are_inspected() {
    for (events, expected_inspections) in [(6, 0), (4, 2), (0, 6)] {
        let store = Arc::new(Store::default());
        let runtime = Arc::new(ConfirmingRuntime {
            store: store.clone(),
            events,
            inspections: AtomicUsize::new(0),
        });
        let service =
            ContainerMutationService::new(store.clone(), runtime.clone(), Arc::new(Tasks));
        service
            .execute(
                ActorId::new(Uuid::now_v7()),
                true,
                (0..6).map(|_| Uuid::now_v7().to_string()).collect(),
                ContainerAction::Stop,
            )
            .await
            .unwrap();
        assert_eq!(
            runtime.inspections.load(Ordering::SeqCst),
            expected_inspections,
            "events={events}"
        );
        assert_eq!(store.finished.load(Ordering::SeqCst), 1, "events={events}");
        assert_eq!(
            store.persisted.load(Ordering::SeqCst),
            expected_inspections,
            "events={events}"
        );
    }
}

#[tokio::test]
async fn task_cancellation_keeps_claim_for_later_read_only_recovery() {
    let store = Arc::new(Store::default());
    let runtime = Arc::new(Runtime::new());
    let service = ContainerMutationService::new(store.clone(), runtime.clone(), Arc::new(Tasks));
    let cancellation = CancellationToken::new();
    let operation = tokio::spawn({
        let service = service.clone();
        let cancellation = cancellation.clone();
        async move {
            service
                .execute_background(
                    ActorId::new(Uuid::now_v7()),
                    vec![Uuid::now_v7().to_string()],
                    ContainerAction::Stop,
                    cancellation,
                )
                .await
        }
    });
    runtime.started.acquire().await.unwrap().forget();
    cancellation.cancel();
    assert!(
        matches!(operation.await.unwrap(), Err(error) if error.kind == RuntimeErrorKind::Unavailable)
    );
    assert_eq!(store.claimed.load(Ordering::SeqCst), 1);
    assert_eq!(store.finished.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn superseded_inspection_is_retried_before_claim_completion() {
    let store = Arc::new(Store::default());
    store.superseded.store(1, Ordering::SeqCst);
    let runtime = Arc::new(Runtime::new());
    runtime.release.add_permits(1);
    let service = ContainerMutationService::new(store.clone(), runtime, Arc::new(Tasks));
    service
        .execute(
            ActorId::new(Uuid::now_v7()),
            true,
            vec![Uuid::now_v7().to_string()],
            ContainerAction::Start,
        )
        .await
        .unwrap();
    assert_eq!(store.batches.load(Ordering::SeqCst), 2);
    assert_eq!(store.persisted.load(Ordering::SeqCst), 1);
    assert_eq!(store.finished.load(Ordering::SeqCst), 1);
}
