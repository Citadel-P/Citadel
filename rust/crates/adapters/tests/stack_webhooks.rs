use citadel_adapters::postgres::stacks::PostgresStackRepository;
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_stacks::*;
use serde_json::json;
use uuid::Uuid;

// Ports StackWebhookDeployQueueRepositoryTests and StackWebhookDeployJobTests:
// coalescing, atomic claim, configuration fencing, bounded retries and recovery.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn stack_webhook_queue_is_fenced_atomic_bounded_and_settled_with_apply() {
    let url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let store = PostgresStackRepository::new(pool.clone());
    let platform = Uuid::now_v7();
    let repository = Uuid::now_v7();
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).bind(platform.to_string()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/repository.git','Idle')")
        .bind(repository).bind(actor.value()).bind(repository.to_string()).execute(&pool).await.unwrap();
    let input = citadel_stacks::CreateStack {
        name: format!("webhook-{platform}"), platform_id: platform,
        stack_source: citadel_stacks::StackSource::Git,
        spec: serde_json::from_value(json!({
        "$type":"Git","gitRepoId":repository,"branch":"main","composePaths":["compose.yml"],"updateBehavior":"StackAutoDeploy",
        "webhook":{"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"disposable-stack-webhook-secret"}
    })).unwrap(),
        description: None, drift_policy: None, tag_ids: vec![], duplicate_source: None,
    };
    let stack = store.create(actor, true, &input).await.unwrap();
    let commit = "a".repeat(40);
    store.enqueue_webhook(&stack, Some(&commit)).await.unwrap();
    store.enqueue_webhook(&stack, Some(&commit)).await.unwrap();
    let jobs = store.ready_webhooks(10).await.unwrap();
    assert_eq!(jobs.len(), 1);
    let id = jobs[0].id;
    // A configuration edit after reception must prevent execution at the claim boundary.
    let mut changed = input.spec.clone();
    if let StackSpec::Git { branch, .. } = &mut changed {
        *branch = "other".into();
    }
    sqlx::query("UPDATE stackreleases SET spec=$2 WHERE id=$1")
        .bind(stack.current_stack_release_id)
        .bind(changed.to_storage_value().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .claim_apply(actor, true, stack.id, None, Some(id))
            .await,
        Err(StackError::NotFound)
    ));
    assert!(store.enqueue_webhook(&stack, Some(&commit)).await.is_err());
    let state: String = sqlx::query_scalar("SELECT controlstate FROM stacks WHERE id=$1")
        .bind(stack.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "Idle");
    sqlx::query("UPDATE stackreleases SET spec=$2 WHERE id=$1")
        .bind(stack.current_stack_release_id)
        .bind(input.spec.to_storage_value().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    // Claim and mark Processing are one transaction; competing consumers cannot dispatch twice.
    let (a, b) = tokio::join!(
        store.claim_apply(actor, true, stack.id, None, Some(id)),
        store.claim_apply(actor, true, stack.id, None, Some(id))
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let claim = a.or(b).unwrap();
    store
        .fail_apply(actor, &claim, "connection lost", true)
        .await
        .unwrap();
    assert!(
        store.ready_webhooks(10).await.unwrap().is_empty(),
        "unknown outcomes are reconciled, never retried blindly"
    );
    let recovery = store
        .stale_apply_claims(i64::MAX, 10)
        .await
        .unwrap()
        .into_iter()
        .find(|(_, c)| c.stack_id == stack.id)
        .unwrap()
        .1;
    store
        .fail_apply(actor, &recovery, "rollout failed", false)
        .await
        .unwrap();
    let row: (String, i32) =
        sqlx::query_as("SELECT status,attempts FROM stackwebhookdeployqueue WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row, ("Queued".into(), 1));
    assert!(
        store.ready_webhooks(10).await.unwrap().is_empty(),
        "retry has a backoff"
    );
    for attempt in 2..=3 {
        sqlx::query("UPDATE stackwebhookdeployqueue SET availableat=now() WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        let claim = store
            .claim_apply(actor, true, stack.id, None, Some(id))
            .await
            .unwrap();
        store
            .fail_apply(actor, &claim, "rollout failed", false)
            .await
            .unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM stackwebhookdeployqueue WHERE id=$1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, if attempt == 3 { 0 } else { 1 });
    }
    // The existing Stack failure activity remains the durable failure explanation.
    let activities: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM activityevents WHERE resourceid=$1 AND status='Failure'",
    )
    .bind(stack.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(activities >= 3);
    let stack = store.get_authorized(actor, true, stack.id).await.unwrap();
    store.enqueue_webhook(&stack, Some(&commit)).await.unwrap();
    let job = store.ready_webhooks(10).await.unwrap().remove(0);
    let claim = store
        .claim_apply(actor, true, stack.id, None, Some(job.id))
        .await
        .unwrap();
    store
        .complete_apply(
            actor,
            &claim,
            &StackRuntimeResult {
                status: StackReleaseStatus::Healthy,
                messages: vec![],
            },
            &[],
            None,
        )
        .await
        .unwrap();
    assert!(store.ready_webhooks(10).await.unwrap().is_empty());
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM stackwebhookdeployqueue WHERE stackid=$1")
            .bind(stack.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    pool.close().await;
}
