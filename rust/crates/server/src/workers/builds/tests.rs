use super::*;
use citadel_adapters::persistence::postgres::builds::PostgresBuildRepository;
use citadel_builds::{
    BuildClaim, BuildExecutionResult, BuildExecutor, BuildLogSink, BuildProjectConfiguration,
    BuildRepository,
};
use citadel_primitives::ActorId;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

struct BlockingExecutor(AtomicUsize);
impl BuildExecutor for BlockingExecutor {
    fn execute<'a>(
        &'a self,
        _: &'a BuildClaim,
        _: &'a dyn BuildLogSink,
        token: &'a CancellationToken,
    ) -> futures_util::future::BoxFuture<'a, BuildExecutionResult> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            token.cancelled().await;
            BuildExecutionResult {
                status: citadel_builds::BuildRunStatus::Cancelled,
                exit_code: None,
                image_digest: None,
                resolved_commit_sha: None,
                image_references: vec![],
                error_code: None,
                error_message: None,
                logs: vec![],
            }
        })
    }
}

// Port of BuildRunWorkerJobTests.Worker_ShouldExecuteQueuedRunsConcurrently.
#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn queued_builds_execute_concurrently_and_shutdown_drains_both() {
    let (pool, actor, store, mut input) = fixture().await;
    let mut runs = Vec::new();
    for _ in 0..2 {
        input.name = format!("parallel-{}", Uuid::now_v7());
        let project = store.create(actor, &input).await.unwrap();
        runs.push(store.enqueue(actor, project.id, "Manual").await.unwrap().id);
    }
    let executor = Arc::new(BlockingExecutor(AtomicUsize::new(0)));
    let token = CancellationToken::new();
    let service = Arc::new(BuildService::new(
        Arc::new(crate::tasks::builds::TrackedBuildTasks::new(
            citadel_runtime::DynamicTasks::new(token.clone()),
        )),
        token.clone(),
        store.clone(),
        executor.clone(),
        chrono::Duration::minutes(5),
    ));
    let worker = tokio::spawn(build_runs(
        token.clone(),
        service,
        2,
        tokio::sync::watch::channel(()).1,
    ));
    tokio::time::timeout(Duration::from_secs(5), async {
        while executor.0.load(Ordering::SeqCst) < 2 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the second build must start before the first completes");
    token.cancel();
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    for id in runs {
        assert_eq!(
            store.get_run(id).await.unwrap().status,
            citadel_builds::BuildRunStatus::Cancelled
        );
    }
    pool.close().await;
}

async fn fixture() -> (
    sqlx::PgPool,
    ActorId,
    Arc<PostgresBuildRepository>,
    BuildProjectConfiguration,
) {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&url)
        .await
        .unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    let platform = Uuid::now_v7();
    let registry = Uuid::now_v7();
    let repository = Uuid::now_v7();
    let tag = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tags(id,name,normalizedname,color,createdbyactorid) VALUES($1,$2,lower($2),'#112233',$3)")
        .bind(tag)
        .bind(format!("build-tag-{}", tag.simple()))
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)").bind(platform).bind(format!("build-platform-{}",platform.simple())).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{}'::json,$2,$3,'docker.io','Enabled')").bind(registry).bind(actor.value()).bind(format!("build-registry-{}",registry.simple())).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/repo.git','Idle')").bind(repository).bind(actor.value()).bind(format!("build-repo-{}",repository.simple())).execute(&pool).await.unwrap();
    let store = Arc::new(PostgresBuildRepository::new(pool.clone()));
    let input = BuildProjectConfiguration {
        name: format!("build-{}", Uuid::now_v7().simple()),
        description: None,
        enabled: true,
        git_repository_id: repository,
        branch: Some("main".into()),
        context_path: None,
        dockerfile_path: None,
        target: None,
        build_args: None,
        build_secrets: None,
        platform_id: Some(platform),
        push_to_registry: true,
        registry_id: Some(registry),
        image_repository: "citadel/test".into(),
        tag_templates: None,
        webhook: None,
        timeout_seconds: Some(60),
        retention_run_count: Some(2),
        tag_ids: vec![tag],
        builder_kind: "Platform".into(),
        build_agent_pool_id: None,
    };

    (pool, actor, store, input)
}

// Port: BuildRunCleanupServiceTests retention/disabled paths. Rust additionally
// protects builds referenced by deployment/stack specifications.
#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn retention_obeys_configuration_and_preserves_referenced_builds() {
    let (pool, actor, store, mut input) = fixture().await;
    input.retention_run_count = Some(10);
    let project = store.create(actor, &input).await.unwrap();
    let mut runs = Vec::new();
    for _ in 0..3 {
        let run = store.enqueue(actor, project.id, "Manual").await.unwrap();
        let claim = store
            .claim_next(chrono::Utc::now() - chrono::Duration::minutes(5))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(claim.run.id, run.id);
        store
            .finish(
                &claim,
                &BuildExecutionResult {
                    status: citadel_builds::BuildRunStatus::Failed,
                    exit_code: Some(1),
                    image_digest: None,
                    resolved_commit_sha: None,
                    image_references: vec![],
                    error_code: None,
                    error_message: None,
                    logs: vec![],
                },
            )
            .await
            .unwrap();
        runs.push(run.id);
    }
    sqlx::query(
        "UPDATE buildruns SET completedat=CURRENT_TIMESTAMP-INTERVAL '91 days' WHERE id=ANY($1)",
    )
    .bind(&runs[..2])
    .execute(&pool)
    .await
    .unwrap();
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$2,$3,$4,'Healthy',$5)")
        .bind(deployment).bind(deployment.to_string()).bind(input.platform_id.unwrap()).bind(serde_json::json!({"image":{"resolvedBuildRunId":runs[1]}})).bind(actor.value()).execute(&pool).await.unwrap();
    for retention in [None, Some(0)] {
        citadel_adapters::persistence::postgres::maintenance::cleanup(&pool, retention)
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM buildruns WHERE id=ANY($1)")
                .bind(&runs)
                .fetch_one(&pool)
                .await
                .unwrap(),
            3
        );
    }
    citadel_adapters::persistence::postgres::maintenance::cleanup(&pool, Some(90))
        .await
        .unwrap();
    let retained: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM buildruns WHERE id=ANY($1)")
        .bind(&runs)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(!retained.contains(&runs[0]));
    assert!(retained.contains(&runs[1]));
    assert!(retained.contains(&runs[2]));
    pool.close().await;
}

struct FailedExecutor;
impl BuildExecutor for FailedExecutor {
    fn execute<'a>(
        &'a self,
        _: &'a BuildClaim,
        _: &'a dyn BuildLogSink,
        _: &'a CancellationToken,
    ) -> futures_util::future::BoxFuture<'a, BuildExecutionResult> {
        Box::pin(async {
            BuildExecutionResult {
                status: citadel_builds::BuildRunStatus::Failed,
                exit_code: Some(1),
                image_digest: None,
                resolved_commit_sha: None,
                image_references: vec![],
                error_code: None,
                error_message: None,
                logs: vec![],
            }
        })
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn project_retention_preserves_both_json_contracts_and_notifies_after_commit() {
    let (pool, actor, store, mut input) = fixture().await;
    input.retention_run_count = Some(2);
    let project = store.create(actor, &input).await.unwrap();
    let notifications = Arc::new(AtomicUsize::new(0));
    let observed = notifications.clone();
    let token = CancellationToken::new();
    let service = BuildService::new(
        Arc::new(crate::tasks::builds::TrackedBuildTasks::new(
            citadel_runtime::DynamicTasks::new(token.clone()),
        )),
        token.clone(),
        store.clone(),
        Arc::new(FailedExecutor),
        chrono::Duration::minutes(5),
    )
    .with_change_notifier(Arc::new(move || {
        observed.fetch_add(1, Ordering::SeqCst);
    }));
    let mut runs = Vec::new();
    for index in 0..6 {
        let run = store.enqueue(actor, project.id, "Manual").await.unwrap();
        assert!(service.process_one(&token).await.unwrap());
        assert_eq!(
            store.get_run(run.id).await.unwrap().status,
            citadel_builds::BuildRunStatus::Failed
        );
        runs.push(run.id);
        if index < 2 {
            let spec = if index == 0 {
                serde_json::json!({"image":{"resolvedBuildRunId":run.id}})
            } else {
                serde_json::json!({"Image":{"AppliedBuildRunId":run.id}})
            };
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$2,$3,$4,'Healthy',$5)")
                .bind(id).bind(id.to_string()).bind(input.platform_id.unwrap()).bind(spec).bind(actor.value()).execute(&pool).await.unwrap();
        } else if index == 2 {
            let stack = Uuid::now_v7();
            let release = Uuid::now_v7();
            sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate,currentstackreleaseid) VALUES($1,$1::text,$2,'WebEditor','{}','{}',$3)")
                .bind(stack).bind(actor.value()).bind(release).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,$4,$5,'Healthy','1')")
                .bind(release).bind(actor.value()).bind(input.platform_id.unwrap())
                .bind(serde_json::json!({"buildImageBindings":[{"resolvedBuildRunId":run.id}]}))
                .bind(stack).execute(&pool).await.unwrap();
        }
    }
    let retained: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM buildruns WHERE buildprojectid=$1")
            .bind(project.id)
            .fetch_all(&pool)
            .await
            .unwrap();
    for id in [&runs[0], &runs[1], &runs[2], &runs[4], &runs[5]] {
        assert!(
            retained.contains(id),
            "referenced and newest runs must survive"
        );
    }
    assert!(
        !retained.contains(&runs[3]),
        "unreferenced run beyond retention must be removed"
    );
    assert_eq!(
        notifications.load(Ordering::SeqCst),
        12,
        "one claim and one committed completion invalidation per run"
    );
    assert!(!service.process_one(&token).await.unwrap());
    assert_eq!(
        notifications.load(Ordering::SeqCst),
        12,
        "idle polling must not notify"
    );
    pool.close().await;
}
