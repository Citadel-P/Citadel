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
    // Sequence increments survive rollback, so fail exactly two completion
    // transactions and verify that Docker is executed only once.
    let suffix = stack.id.simple();
    let sequence = format!("stack_completion_attempts_{suffix}");
    let function = format!("reject_stack_completion_{suffix}");
    let trigger = format!("reject_stack_completion_{suffix}");
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE SEQUENCE {sequence}; CREATE FUNCTION {function}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.stackid='{}'::uuid AND NEW.status='Healthy' AND nextval('{sequence}')<=2 THEN RAISE EXCEPTION 'temporary completion failure' USING ERRCODE='40001'; END IF; RETURN NEW; END $$; CREATE TRIGGER {trigger} BEFORE UPDATE ON stackreleases FOR EACH ROW EXECUTE FUNCTION {function}();", stack.id
    ))).execute(pool).await.unwrap();
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
    assert_eq!(
        applied.control_state,
        citadel_primitives::ResourceControlState::Idle
    );
    assert_eq!(runtime.apply_calls.lock().unwrap().len(), 1);
    let attempts: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT last_value FROM {sequence}"
    )))
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(attempts, 3);
    let activities: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activityevents WHERE resourceid=$1 AND eventtype='StackApplied'",
    )
    .bind(stack.id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(activities, 1);
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("DROP TRIGGER {trigger} ON stackreleases; DROP FUNCTION {function}(); DROP SEQUENCE {sequence};")))
        .execute(pool).await.unwrap();

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
        assert_eq!(
            rejected.control_state,
            citadel_primitives::ResourceControlState::Idle
        );
    }
    assert_eq!(runtime.apply_calls.lock().unwrap().len(), 1);
    verify_failed_completion(pool, actor, stack.id, repository.clone()).await;
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
    assert_eq!(
        unknown.control_state,
        citadel_primitives::ResourceControlState::Processing
    );
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
        citadel_primitives::ResourceControlState::Idle
    );
    service.delete(actor, true, &[stack.id]).await.unwrap();
}

async fn verify_failed_completion(
    pool: &sqlx::PgPool,
    actor: ActorId,
    id: Uuid,
    repository: Arc<PostgresStackRepository>,
) {
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
    let previous_failures: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM activityevents WHERE resourceid=$1 AND status='Failure'",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    let name = format!("fail_stack_completion_{}", id.simple());
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.stackid='{id}'::uuid AND NEW.status='Healthy' THEN RAISE EXCEPTION 'completion storage unavailable' USING ERRCODE='40001'; END IF; RETURN NEW; END $$; CREATE TRIGGER {name} BEFORE UPDATE ON stackreleases FOR EACH ROW EXECUTE FUNCTION {name}();"
    ))).execute(pool).await.unwrap();
    let mut progress = service
        .apply(
            actor,
            true,
            citadel_stacks::ApplyStack { id, recreate: None },
        )
        .await
        .unwrap();
    let last = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let mut last = None;
        while let Some(item) = progress.recv().await {
            last = Some(item);
        }
        last.unwrap()
    })
    .await
    .unwrap();
    owner
        .drain(std::time::Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(last.stack_status, Some(StackReleaseStatus::Unknown));
    assert!(last.message.unwrap().contains("saving its result failed"));
    assert_eq!(runtime.apply_calls.lock().unwrap().len(), 1);
    let pending = repository.get_authorized(actor, true, id).await.unwrap();
    assert_eq!(
        pending.control_state,
        citadel_primitives::ResourceControlState::Processing
    );
    assert_eq!(pending.status, StackReleaseStatus::Applying);
    let failures: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM activityevents WHERE resourceid=$1 AND status='Failure'",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(failures, previous_failures);
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TRIGGER {name} ON stackreleases; DROP FUNCTION {name}();"
    )))
    .execute(pool)
    .await
    .unwrap();
    service
        .reconcile_stale_operations(std::time::Duration::ZERO, 100)
        .await
        .unwrap();
    assert_eq!(
        repository
            .get_authorized(actor, true, id)
            .await
            .unwrap()
            .control_state,
        citadel_primitives::ResourceControlState::Idle
    );
    assert_eq!(runtime.apply_calls.lock().unwrap().len(), 1);
}
