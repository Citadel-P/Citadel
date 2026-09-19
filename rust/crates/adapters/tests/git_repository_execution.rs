use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use chrono::Utc;
use citadel_adapters::crypto::AesGcmSecretProtector;
use citadel_adapters::postgres::git::accounts::PostgresGitAccountRepository;
use citadel_adapters::postgres::git::repositories::PostgresGitRepositoryExecutionPersistence;
use citadel_adapters::stack_source_materializer::GitStackSourceMaterializer;
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_git::{
    GitAccountService, GitCli, GitRepositoryExecutionPersistence, GitRepositoryExecutionService,
    SyncResult,
};
use citadel_stacks::{
    StackOperationClaim, StackSourceMaterializerPort, StackSpec, StackSpecCommon,
    StackUpdateBehavior,
};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
#[path = "git_repository_execution/stack_updates.rs"]
mod git_stack_updates;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL and Git"]
async fn synchronization_claims_recover_and_real_git_results_are_persisted() {
    let database_url = std::env::var("CITADEL_PHASE7_DATABASE_URL")
        .expect("CITADEL_PHASE7_DATABASE_URL is required");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'System')")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    let store = Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone()));

    let race_id = Uuid::now_v7();
    insert_repository(&pool, actor, race_id, "file:///unused", "race").await;
    let claim = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10))
        .await
        .unwrap()
        .unwrap();
    store
        .enqueue_sync(ActorId::new(actor), race_id, Some("main"))
        .await
        .unwrap();
    let state: (String, String) = sqlx::query_as(
        "SELECT reference.status,repository.controlstate FROM gitrepositoryrefs reference JOIN gitrepositories repository ON repository.id=reference.gitrepositoryid WHERE reference.gitrepositoryid=$1",
    )
    .bind(race_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, ("Syncing".to_owned(), "Queued".to_owned()));
    store
        .complete(
            &claim,
            &SyncResult {
                commit: "a".repeat(40),
                cloned: true,
            },
        )
        .await
        .unwrap();
    let state: (String, String) = sqlx::query_as(
        "SELECT reference.status,repository.controlstate FROM gitrepositoryrefs reference JOIN gitrepositories repository ON repository.id=reference.gitrepositoryid WHERE reference.gitrepositoryid=$1",
    )
    .bind(race_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, ("Pending".to_owned(), "Queued".to_owned()));
    let repeated = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10))
        .await
        .unwrap()
        .unwrap();
    store.fail(&repeated, "fixture completed").await.unwrap();
    let webhook = serde_json::json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"old-key"});
    sqlx::query("UPDATE gitrepositories SET webhook=$2 WHERE id=$1")
        .bind(race_id)
        .bind(webhook)
        .execute(&pool)
        .await
        .unwrap();
    let authenticated = store.get_webhook(race_id).await.unwrap().unwrap();
    sqlx::query("UPDATE gitrepositories SET webhook=NULL WHERE id=$1")
        .bind(race_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .enqueue_webhook(ActorId::new(actor), race_id, "main", &authenticated)
            .await,
        Err(citadel_git::GitRepositoryExecutionError::Conflict)
    ));
    let control: String =
        sqlx::query_scalar("SELECT controlstate FROM gitrepositories WHERE id=$1")
            .bind(race_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        control, "Idle",
        "a revoked webhook cannot queue after authenticating against an old snapshot"
    );

    let root = std::env::temp_dir().join(format!("citadel-phase7-git-{}", Uuid::now_v7()));
    let remote = root.join("remote");
    let cache = root.join("cache");
    std::fs::create_dir_all(&remote).unwrap();
    git(&remote, &["init", "--initial-branch=main"]);
    git(&remote, &["config", "user.email", "citadel@example.test"]);
    git(&remote, &["config", "user.name", "Citadel Test"]);
    std::fs::write(remote.join("compose.yaml"), b"services: {}\n").unwrap();
    std::fs::create_dir_all(remote.join("config")).unwrap();
    std::fs::write(remote.join("config/app.conf"), b"mode=production\n").unwrap();
    git(&remote, &["add", "compose.yaml", "config/app.conf"]);
    git(&remote, &["commit", "-m", "initial"]);
    let expected = command_output(&remote, &["rev-parse", "HEAD"]);
    let repository_id = Uuid::now_v7();
    insert_repository(
        &pool,
        actor,
        repository_id,
        &format!("file:///{}", remote.to_string_lossy().replace('\\', "/")),
        "real",
    )
    .await;
    let accounts = Arc::new(GitAccountService::new(
        Arc::new(PostgresGitAccountRepository::new(pool.clone())),
        Arc::new(AesGcmSecretProtector::new(&[91_u8; 32]).unwrap()),
    ));
    let notifications = Arc::new(AtomicUsize::new(0));
    let changes = notifications.clone();
    let service = Arc::new(
        GitRepositoryExecutionService::new(
            store,
            accounts,
            Arc::new(GitCli::new(Duration::from_secs(10))),
            cache,
            Duration::from_secs(60),
        )
        .with_change_notifier(Arc::new(move || {
            changes.fetch_add(1, Ordering::SeqCst);
        })),
    );
    let request_actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(request_actor)
        .execute(&pool)
        .await
        .unwrap();
    service
        .request_sync(ActorId::new(request_actor), repository_id, Some("main"))
        .await
        .unwrap();
    assert!(
        service
            .process_one(&CancellationToken::new())
            .await
            .unwrap()
    );
    let persisted: (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT status,resolvedcommitsha,lasterror FROM gitrepositoryrefs WHERE gitrepositoryid=$1",
    )
    .bind(repository_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(persisted.0, "Healthy");
    assert_eq!(persisted.1.as_deref(), Some(expected.trim()));
    assert!(persisted.2.is_none());
    assert_eq!(
        notifications.load(Ordering::SeqCst),
        2,
        "claim and committed completion both notify"
    );
    assert!(
        !service
            .process_one(&CancellationToken::new())
            .await
            .unwrap()
    );
    assert_eq!(
        notifications.load(Ordering::SeqCst),
        2,
        "idle workers do not notify"
    );
    let snapshot = service
        .stack_snapshot(
            repository_id,
            Some(expected.trim()),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(snapshot.resolved_commit_sha, expected.trim());
    assert_eq!(
        snapshot
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>(),
        ["compose.yaml", "config/app.conf"]
    );
    assert_eq!(snapshot.files[1].content, b"mode=production\n");
    let source = GitStackSourceMaterializer::new(Arc::clone(&service))
        .materialize(
            &StackOperationClaim {
                stack_id: Uuid::now_v7(),
                release_id: Uuid::now_v7(),
                platform_id: Uuid::now_v7(),
                name: "git-stack".to_owned(),
                project_name: "git-stack".to_owned(),
                platform_type: "Docker".to_owned(),
                spec: StackSpec::Git {
                    git_repo_id: repository_id,
                    branch: "main".to_owned(),
                    commit_sha: Some(expected.trim().to_owned()),
                    update_behavior: StackUpdateBehavior::Disabled,
                    webhook: None,
                    compose_paths: vec!["compose.yaml".to_owned()],
                    working_directory: None,
                    compose_env_files_from_repo: Vec::new(),
                    watch_paths: Vec::new(),
                    additional_env_file_from_repo: Vec::new(),
                    common: StackSpecCommon::default(),
                },
                row_version: 1,
                actor_id: actor,
                operation: "Apply".to_owned(),
                service_names: Vec::new(),
            },
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(source.compose_paths, ["compose.yaml"]);
    assert_eq!(source.working_directory, ".");
    assert_eq!(source.resolved_commit_sha.as_deref(), Some(expected.trim()));
    assert_eq!(source.files.len(), 2);
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activityevents WHERE resourceid=$1 AND eventtype='GitRepoCloned' AND status='Success'",
    )
    .bind(repository_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity_count, 1);
    let activity_actor: Uuid = sqlx::query_scalar("SELECT createdbyactorid FROM activityevents WHERE resourceid=$1 AND eventtype='GitRepoCloned'")
        .bind(repository_id).fetch_one(&pool).await.unwrap();
    assert_eq!(
        activity_actor, request_actor,
        "manual synchronization uses the requester rather than the repository creator"
    );

    // Build execution must refresh the remote branch, not silently reuse the
    // previous successful cache. The same worker owns all cache mutations.
    std::fs::write(remote.join("config/app.conf"), b"mode=updated\n").unwrap();
    git(&remote, &["add", "config/app.conf"]);
    git(&remote, &["commit", "-m", "update"]);
    let updated = command_output(&remote, &["rev-parse", "HEAD"]);
    let token = CancellationToken::new();
    let wait = service.synchronize_commit(ActorId::new(actor), repository_id, "main", &token);
    let worker = async {
        while !service.process_one(&token).await.unwrap() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    let (resolved, ()) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(wait, worker)
    })
    .await
    .unwrap();
    assert_eq!(resolved.unwrap(), updated.trim());
    assert_ne!(updated.trim(), expected.trim());
    let pinned = service
        .stack_snapshot(repository_id, Some(expected.trim()), &token)
        .await
        .unwrap();
    assert_eq!(pinned.resolved_commit_sha, expected.trim());
    git_stack_updates::verify(
        &pool,
        service.clone(),
        ActorId::new(actor),
        repository_id,
        expected.trim(),
        updated.trim(),
    )
    .await;

    // Cancellation before a request must not enqueue new shared work.
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(
        service
            .synchronize_commit(ActorId::new(actor), repository_id, "main", &cancelled)
            .await
            .is_err()
    );
    assert!(!service.process_one(&token).await.unwrap());

    // A failed synchronization is terminal for this waiter, never a fallback to
    // the last good commit and never a five-minute wait after the failure.
    let wait =
        service.synchronize_commit(ActorId::new(actor), repository_id, "missing-branch", &token);
    let worker = async {
        loop {
            match service.process_one(&token).await {
                Ok(false) => tokio::time::sleep(Duration::from_millis(10)).await,
                result => {
                    assert!(
                        result.unwrap(),
                        "the worker persists the Git failure as a completed iteration"
                    );
                    break;
                }
            }
        }
    };
    let (resolved, ()) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(wait, worker)
    })
    .await
    .unwrap();
    assert!(resolved.is_err());

    sqlx::query("DELETE FROM gitrepositories WHERE id=ANY($1::uuid[])")
        .bind(vec![race_id, repository_id])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE resourceid=ANY($1::uuid[])")
        .bind(vec![race_id, repository_id])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=ANY($1)")
        .bind(vec![actor, request_actor])
        .execute(&pool)
        .await
        .unwrap();
    let _ = std::fs::remove_dir_all(root);
}

async fn insert_repository(pool: &sqlx::PgPool, actor: Uuid, id: Uuid, url: &str, name: &str) {
    sqlx::query(
        "INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate,controltriggeredby) VALUES($1,$2,'main',$3,'Pending','Manual',$4,'Queued',$2)",
    )
    .bind(id)
    .bind(actor)
    .bind(format!("{name}-{}", id.simple()))
    .bind(url)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO gitrepositoryrefs(id,gitrepositoryid,branch,status,lastsyncedat) VALUES($1,$2,'main','Pending',CURRENT_TIMESTAMP)",
    )
    .bind(Uuid::now_v7())
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

fn git(directory: &std::path::Path, arguments: &[&str]) {
    let status = Command::new("git")
        .current_dir(directory)
        .args(arguments)
        .status()
        .expect("Git is installed");
    assert!(status.success(), "Git command failed: {arguments:?}");
}

fn command_output(directory: &std::path::Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(directory)
        .args(arguments)
        .output()
        .expect("Git is installed");
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn poll_activity_scope_and_webhook_failure_snapshot_match_job_policy() {
    let url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let actor = Uuid::now_v7();
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'System')")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    insert_repository(&pool, actor, id, "file:///unused", "poll-policy").await;
    let store = PostgresGitRepositoryExecutionPersistence::new(pool.clone());
    let claim = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10))
        .await
        .unwrap()
        .unwrap();
    store
        .complete(
            &claim,
            &SyncResult {
                commit: "a".repeat(40),
                cloned: true,
            },
        )
        .await
        .unwrap();
    async fn count(pool: &sqlx::PgPool, id: Uuid) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }
    async fn poll(
        store: &PostgresGitRepositoryExecutionPersistence,
        pool: &sqlx::PgPool,
        actor: Uuid,
        id: Uuid,
        branch: &str,
    ) -> citadel_git::GitSyncClaim {
        store
            .enqueue_sync(ActorId::new(actor), id, Some(branch))
            .await
            .unwrap();
        sqlx::query("UPDATE gitrepositoryrefs SET synctrigger='Poll' WHERE gitrepositoryid=$1 AND branch=$2").bind(id).bind(branch).execute(pool).await.unwrap();
        store
            .claim_next(Utc::now() - chrono::Duration::minutes(10))
            .await
            .unwrap()
            .unwrap()
    }
    assert_eq!(count(&pool, id).await, 1);
    let claim = poll(&store, &pool, actor, id, "main").await;
    store
        .complete(
            &claim,
            &SyncResult {
                commit: "A".repeat(40),
                cloned: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        count(&pool, id).await,
        1,
        "unchanged poll is silent, including SHA casing"
    );
    for expected in [2, 2] {
        let claim = poll(&store, &pool, actor, id, "main").await;
        store.fail(&claim, "same transport error").await.unwrap();
        assert_eq!(
            count(&pool, id).await,
            expected,
            "identical polling failure records once"
        );
    }
    let claim = poll(&store, &pool, actor, id, "main").await;
    store
        .complete(
            &claim,
            &SyncResult {
                commit: "b".repeat(40),
                cloned: false,
            },
        )
        .await
        .unwrap();
    let claim = poll(&store, &pool, actor, id, "feature").await;
    store.fail(&claim, "secondary unavailable").await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM gitrepositories WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        status, "Healthy",
        "secondary polling failure does not degrade default branch"
    );
    let webhook = serde_json::json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"test"});
    sqlx::query("UPDATE gitrepositories SET webhook=$2 WHERE id=$1")
        .bind(id)
        .bind(webhook)
        .execute(&pool)
        .await
        .unwrap();
    let expected = store.get_webhook(id).await.unwrap().unwrap();
    let mut listener = sqlx::postgres::PgListener::connect(&url).await.unwrap();
    listener.listen("citadel_job_alerts").await.unwrap();
    store
        .enqueue_webhook(ActorId::new(actor), id, "main", &expected)
        .await
        .unwrap();
    let claim = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        claim.trigger, "Webhook",
        "trigger survives durable queue round trip"
    );
    store
        .fail(&claim, "webhook execution failed")
        .await
        .unwrap();
    let notification = tokio::time::timeout(Duration::from_secs(2), listener.recv())
        .await
        .unwrap()
        .unwrap();
    let observation: citadel_alerts::AlertObservation =
        serde_json::from_str(notification.payload()).unwrap();
    assert_eq!(observation.alert_type, "WebhookGitRepoSyncFailed");
    assert_eq!(observation.resource_id, id);
    assert_eq!(observation.info["Reason"], "webhook execution failed");
    assert_eq!(observation.resource_type, "Webhook");
    assert_eq!(
        observation.deduplication_component,
        "main:webhook execution failed"
    );
    assert_eq!(observation.info["Branch"], "main");
    assert!(matches!(
        store.fail(&claim, "duplicate completion").await,
        Err(citadel_git::GitRepositoryExecutionError::Conflict)
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.recv())
            .await
            .is_err(),
        "failed transaction publishes no duplicate alert"
    );
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///'||$2,'Local',1,0,1024,$2,0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).bind(format!("poll-{platform}")).execute(&pool).await.unwrap();
    let input = citadel_stacks::CreateStack {
        name: format!("poll-stack-{platform}"), platform_id: platform,
        stack_source: citadel_stacks::StackSource::Git,
        spec: serde_json::from_value(serde_json::json!({"$type":"Git","gitRepoId":id,"branch":"feature","composePaths":["compose.yaml"],"updateBehavior":"Notify"})).unwrap(),
        description: None, drift_policy: None, tag_ids: vec![], duplicate_source: None,
    };
    let stack = citadel_stacks::StackRepository::create(
        &citadel_adapters::postgres::stacks::PostgresStackRepository::new(pool.clone()),
        ActorId::new(actor),
        true,
        &input,
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE gitrepositories SET syncmode='PullInterval',syncintervalminutes=1 WHERE id=$1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE gitrepositoryrefs SET lastsyncedat=now()-interval '5 minutes' WHERE gitrepositoryid=$1").bind(id).execute(&pool).await.unwrap();
    assert_eq!(
        store.enqueue_due(100).await.unwrap(),
        2,
        "poll both default branch and branch subscribed by a Git Stack"
    );
    assert_eq!(
        store.enqueue_due(100).await.unwrap(),
        0,
        "overlapping scheduler ticks cannot duplicate queued branches"
    );
    let mut branches = std::collections::BTreeSet::new();
    for _ in 0..2 {
        let claim = store
            .claim_next(Utc::now() - chrono::Duration::minutes(10))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(claim.trigger, "Poll");
        branches.insert(claim.branch.clone());
        store
            .complete(
                &claim,
                &SyncResult {
                    commit: "b".repeat(40),
                    cloned: false,
                },
            )
            .await
            .unwrap();
    }
    assert_eq!(
        branches,
        std::collections::BTreeSet::from(["main".to_string(), "feature".to_string()])
    );
    let requestor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'System')")
        .bind(requestor)
        .execute(&pool)
        .await
        .unwrap();
    store
        .enqueue_sync(ActorId::new(requestor), id, Some("main"))
        .await
        .unwrap();
    let claim = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10))
        .await
        .unwrap()
        .unwrap();
    store
        .complete(
            &claim,
            &SyncResult {
                commit: "b".repeat(40),
                cloned: false,
            },
        )
        .await
        .unwrap();
    let recorded:Uuid=sqlx::query_scalar("SELECT createdbyactorid FROM activityevents WHERE resourceid=$1 ORDER BY createdat DESC,id DESC LIMIT 1").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(
        recorded, requestor,
        "manual activity retains the request actor rather than the repository creator"
    );
    sqlx::query("DELETE FROM stacks WHERE id=$1")
        .bind(stack.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
        .bind(stack.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM gitrepositories WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(requestor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
}

struct NoGitProcess;
impl citadel_git::GitProcessPort for NoGitProcess {
    fn run<'a>(
        &'a self,
        _: citadel_execution::ProcessRequest,
        _: &'a CancellationToken,
    ) -> futures_util::future::BoxFuture<
        'a,
        Result<citadel_execution::ProcessOutput, citadel_execution::ProcessError>,
    > {
        Box::pin(async { panic!("invalid credentials/remote must fail before invoking Git") })
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn invalid_credentials_and_unresolvable_remote_release_claim_without_running_git() {
    let url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let actor = Uuid::now_v7();
    let account = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO gitaccounts(id,authtype,configuration,createdbyactorid,domain,name,transport) VALUES($1,'Token','{}',$2,'github.com',$1::text,'Https')")
        .bind(account).bind(actor).execute(&pool).await.unwrap();
    let accounts = Arc::new(GitAccountService::new(
        Arc::new(PostgresGitAccountRepository::new(pool.clone())),
        Arc::new(AesGcmSecretProtector::new(&[91; 32]).unwrap()),
    ));
    let store = Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone()));
    let service = GitRepositoryExecutionService::new(
        store.clone(),
        accounts,
        Arc::new(GitCli::with_process(
            Arc::new(NoGitProcess),
            "must-not-run",
            Duration::from_secs(1),
        )),
        std::env::temp_dir(),
        Duration::from_secs(60),
    );
    for (linked_account, expected_error) in [
        (Some(account), "credential"),
        (None, "complete repository URL"),
    ] {
        let id = Uuid::now_v7();
        insert_repository(&pool, actor, id, "relative/repository", "invalid-remote").await;
        sqlx::query("UPDATE gitrepositories SET gitaccountid=$2 WHERE id=$1")
            .bind(id)
            .bind(linked_account)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            service
                .process_one(&CancellationToken::new())
                .await
                .unwrap()
        );
        let row: (String, String, String, String) = sqlx::query_as("SELECT r.status,r.controlstate,f.status,f.lasterror FROM gitrepositories r JOIN gitrepositoryrefs f ON f.gitrepositoryid=r.id WHERE r.id=$1")
            .bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "Degraded");
        assert_eq!(row.1, "Idle");
        assert_eq!(row.2, "Degraded");
        assert!(row.3.contains(expected_error), "{}", row.3);
        let activity_actor: Uuid = sqlx::query_scalar("SELECT createdbyactorid FROM activityevents WHERE resourceid=$1 ORDER BY createdat DESC LIMIT 1")
            .bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(activity_actor, actor);
        sqlx::query("DELETE FROM gitrepositories WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM gitaccounts WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
