use citadel_stacks::StackRepository;
#[path = "../support/workload_queries.rs"]
mod query_counts;
use super::*;
use citadel_runtime::DynamicTasks;
use citadel_server::tasks::stacks::TrackedStackTasks;

pub(super) async fn verify(pool: &sqlx::PgPool, actor: ActorId, platform: Uuid) {
    let repository = Arc::new(PostgresStackRepository::new(pool.clone()));
    let before = query_counts::count(pool).await;
    let listed = repository
        .list_authorized(actor, true, &Default::default())
        .await
        .unwrap();
    assert!(!listed.is_empty());
    query_counts::assert_single_query(pool, before).await;
    let shutdown = CancellationToken::new();
    let owner = DynamicTasks::new(shutdown.clone());
    let runtime = Arc::new(CompletingStackRuntime::default());
    let service = StackService::new(
        Arc::new(TrackedStackTasks::new(owner.clone())),
        repository.clone(),
        runtime.clone(),
        Arc::new(FixtureBindings),
        Arc::new(NoopStackChangeNotifier),
        shutdown,
    );
    let stack = service.create(actor, true, citadel_stacks::CreateStack {
        name: format!("tracked-{}", Uuid::now_v7()), platform_id: platform,
        description: None, stack_source: citadel_stacks::StackSource::WebEditor,
        spec: serde_json::from_value(json!({"$type":"WebEditor","composeFile":"services:\n  web:\n    image: nginx:alpine\n"})).unwrap(),
        drift_policy: None, tag_ids: vec![], duplicate_source: None,
    }).await.unwrap();
    let progress = service
        .apply(
            actor,
            true,
            citadel_stacks::ApplyStack {
                id: stack.id,
                recreate: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(owner.active(), 1);
    drop(progress); // The HTTP stream can disappear without dropping the claim owner.
    owner
        .drain(std::time::Duration::from_secs(5))
        .await
        .unwrap();
    let applied = repository
        .get_authorized(actor, true, stack.id)
        .await
        .unwrap();
    assert_eq!(applied.status, StackReleaseStatus::Healthy);
    assert_eq!(applied.control_state, "Idle");
    assert_eq!(runtime.apply_calls.lock().unwrap().len(), 1);
    // Rejection happens after Stack claim acquisition. Repeated attempts must
    // release both that claim and the four-operation semaphore.
    for _ in 0..5 {
        assert!(matches!(
            service
                .apply(
                    actor,
                    true,
                    citadel_stacks::ApplyStack {
                        id: stack.id,
                        recreate: None
                    }
                )
                .await,
            Err(StackError::Cancelled)
        ));
        let rejected = repository
            .get_authorized(actor, true, stack.id)
            .await
            .unwrap();
        assert_eq!(rejected.control_state, "Idle");
    }
    assert_eq!(runtime.apply_calls.lock().unwrap().len(), 1);
    let shutdown = CancellationToken::new();
    let cancelling_owner = DynamicTasks::new(shutdown.clone());
    let cancelling = StackService::new(
        Arc::new(TrackedStackTasks::new(cancelling_owner.clone())),
        repository.clone(),
        runtime.clone(),
        Arc::new(FixtureBindings),
        Arc::new(NoopStackChangeNotifier),
        shutdown.clone(),
    );
    runtime.hold_apply.store(1, Ordering::Relaxed);
    let progress = cancelling
        .apply(
            actor,
            true,
            citadel_stacks::ApplyStack {
                id: stack.id,
                recreate: None,
            },
        )
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while runtime.hold_apply.load(Ordering::Relaxed) != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    drop(progress);
    shutdown.cancel();
    cancelling_owner
        .drain(std::time::Duration::from_secs(5))
        .await
        .unwrap();
    let unknown = repository
        .get_authorized(actor, true, stack.id)
        .await
        .unwrap();
    assert_eq!(unknown.control_state, "Processing");
    assert_eq!(unknown.status, StackReleaseStatus::Applying);
    assert!(
        service
            .reconcile_stale_operations(std::time::Duration::ZERO, 100)
            .await
            .unwrap()
            >= 1
    );
    assert_eq!(
        repository
            .get_authorized(actor, true, stack.id)
            .await
            .unwrap()
            .control_state,
        "Idle"
    );
    service.delete(actor, true, &[stack.id]).await.unwrap();
}
