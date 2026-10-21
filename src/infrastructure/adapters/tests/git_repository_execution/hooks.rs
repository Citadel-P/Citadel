use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL and Git"]
async fn hooks_execute_in_durable_sync_and_failed_or_interrupted_runs_require_manual_retry() {
    let database_url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
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
    let root = std::env::temp_dir().join(format!("citadel-hooks-{}", Uuid::now_v7()));
    let remote = root.join("remote");
    std::fs::create_dir_all(remote.join("scripts")).unwrap();
    git(&remote, &["init", "-b", "main"]);
    git(&remote, &["config", "user.name", "Citadel Tests"]);
    git(
        &remote,
        &["config", "user.email", "citadel-tests@example.invalid"],
    );
    std::fs::write(remote.join("scripts/tracked"), "original").unwrap();
    git(&remote, &["add", "."]);
    git(&remote, &["commit", "-m", "initial"]);
    let commit = command_output(&remote, &["rev-parse", "HEAD"])
        .trim()
        .to_owned();
    let id = Uuid::now_v7();
    insert_repository(
        &pool,
        actor,
        id,
        url::Url::from_file_path(&remote).unwrap().as_str(),
        "hooks",
    )
    .await;
    let clone = serde_json::json!({"path":"scripts","commands":["printf clone > hook.log", "printf secret >&2; exit 9"]});
    let pull = serde_json::json!({"path":"scripts","commands":["printf pull >> hook.log"]});
    sqlx::query("UPDATE gitrepositories SET onclone=$2,onpull=$3,syncmode='PullInterval',syncintervalminutes=1 WHERE id=$1")
        .bind(id).bind(clone.to_string()).bind(pull.to_string()).execute(&pool).await.unwrap();
    let store = Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone()));
    let service = GitRepositoryExecutionService::new(
        store.clone(),
        Arc::new(GitAccountService::new(
            Arc::new(PostgresGitAccountRepository::new(pool.clone())),
            Arc::new(AesGcmSecretProtector::new(&[91_u8; 32]).unwrap()),
        )),
        Arc::new(GitCli::new(
            Arc::new(citadel_processes::SystemProcess),
            Arc::new(
                citadel_adapters::filesystem::git_workspace::LocalGitWorkspace::new(
                    citadel_runtime::DynamicTasks::new(CancellationToken::new()),
                ),
            ),
            Duration::from_secs(10),
        )),
        root.join("cache"),
        Duration::from_secs(60),
    );
    let cancel = CancellationToken::new();
    let mut completion = service.subscribe_completion();
    assert!(service.process_one(&cancel).await.unwrap());
    assert!(completion.has_changed().unwrap());
    completion.borrow_and_update();
    let reference = store.get_ref(id, "main").await.unwrap().unwrap();
    assert_eq!(
        reference.status,
        citadel_git::GitRepositoryRefStatus::Degraded
    );
    let error = reference.last_error.unwrap();
    assert!(error.contains("On Clone command 2"), "{error}");
    assert!(!error.contains("secret"), "{error}");
    assert!(
        !service.cache_path(id).exists(),
        "failed clone hook must not publish the cache"
    );
    assert_eq!(
        std::fs::read_dir(root.join("cache")).unwrap().count(),
        0,
        "staging directories must be cleaned"
    );
    sqlx::query("UPDATE gitrepositoryrefs SET lastsyncedat=CURRENT_TIMESTAMP-INTERVAL '10 minutes' WHERE gitrepositoryid=$1").bind(id).execute(&pool).await.unwrap();
    assert_eq!(service.enqueue_due(100).await.unwrap(), 0);
    assert!(
        store
            .enqueue_apply(ActorId::new(actor), id, "main")
            .await
            .is_err()
    );
    assert!(
        store
            .enqueue_sync(
                ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
                id,
                Some("main")
            )
            .await
            .is_err()
    );
    let webhook = citadel_primitives::WebhookConfig::default();
    sqlx::query("UPDATE gitrepositories SET webhook=$2 WHERE id=$1")
        .bind(id)
        .bind(sqlx::types::Json(&webhook))
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        store
            .enqueue_webhook(ActorId::new(actor), id, "main", &webhook)
            .await
            .is_err()
    );
    let clone = serde_json::json!({"path":"scripts","commands":["test -z \"${CITADEL_EXECUTION_DATABASE_URL+x}\"; printf clone > hook.log"]});
    sqlx::query("UPDATE gitrepositories SET onclone=$2 WHERE id=$1")
        .bind(id)
        .bind(clone.to_string())
        .execute(&pool)
        .await
        .unwrap();
    service
        .request_sync(ActorId::new(actor), id, None)
        .await
        .unwrap();
    assert!(service.process_one(&cancel).await.unwrap());
    let cached = service.cache_path(id).join("scripts/hook.log");
    assert_eq!(std::fs::read_to_string(&cached).unwrap(), "clone");
    assert_eq!(
        service
            .synchronized_ref(id, "main")
            .await
            .unwrap()
            .resolved_commit_sha
            .as_deref(),
        Some(commit.as_str())
    );
    let listing = service
        .list_directory(id, Some("main"), "scripts", &cancel)
        .await
        .unwrap();
    assert!(
        listing.entries.iter().all(|entry| entry.name != "hook.log"),
        "generated files must not enter committed source reads"
    );
    // An unchanged fetch still executes On Pull; commands run in array order.
    service
        .request_sync(ActorId::new(actor), id, None)
        .await
        .unwrap();
    assert!(service.process_one(&cancel).await.unwrap());
    assert_eq!(std::fs::read_to_string(&cached).unwrap(), "clonepull");
    // Deployment and webhook requests use the same hook path.
    store
        .enqueue_apply(ActorId::new(actor), id, "main")
        .await
        .unwrap();
    assert!(service.process_one(&cancel).await.unwrap());
    store
        .enqueue_webhook(ActorId::new(actor), id, "main", &webhook)
        .await
        .unwrap();
    assert!(service.process_one(&cancel).await.unwrap());
    sqlx::query("UPDATE gitrepositoryrefs SET lastsyncedat=CURRENT_TIMESTAMP-INTERVAL '10 minutes' WHERE gitrepositoryid=$1").bind(id).execute(&pool).await.unwrap();
    assert_eq!(service.enqueue_due(100).await.unwrap(), 1);
    assert!(service.process_one(&cancel).await.unwrap());
    assert_eq!(
        std::fs::read_to_string(&cached).unwrap(),
        "clonepullpullpullpull"
    );
    // Simulate a crash after claiming: recovery must not replay an ambiguous hook.
    service
        .request_sync(ActorId::new(actor), id, None)
        .await
        .unwrap();
    let claim = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10))
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE gitrepositoryrefs SET lastsyncedat=CURRENT_TIMESTAMP-INTERVAL '10 minutes' WHERE gitrepositoryid=$1").bind(id).execute(&pool).await.unwrap();
    let recovered = PostgresGitRepositoryExecutionPersistence::new(pool.clone());
    assert!(
        recovered
            .claim_next(Utc::now() - chrono::Duration::minutes(1))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store.get_ref(id, "main").await.unwrap().unwrap().status,
        citadel_git::GitRepositoryRefStatus::Degraded
    );
    assert_eq!(service.enqueue_due(100).await.unwrap(), 0);
    assert!(
        store
            .complete(
                &claim,
                &SyncResult {
                    commit: commit.clone(),
                    cloned: false
                }
            )
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(&cached).unwrap(),
        "clonepullpullpullpull"
    );
    // Explicit review/retry restores normal synchronization.
    service
        .request_sync(ActorId::new(actor), id, None)
        .await
        .unwrap();
    assert!(service.process_one(&cancel).await.unwrap());
    assert_eq!(
        std::fs::read_to_string(&cached).unwrap(),
        "clonepullpullpullpullpull"
    );
    // A request arriving during a failing sync must not silently replay its hooks.
    service
        .request_sync(ActorId::new(actor), id, None)
        .await
        .unwrap();
    let claim = store
        .claim_next(Utc::now() - chrono::Duration::minutes(10))
        .await
        .unwrap()
        .unwrap();
    service
        .request_sync(ActorId::new(actor), id, None)
        .await
        .unwrap();
    store
        .fail(&claim, "hook failure after a side effect")
        .await
        .unwrap();
    assert!(!service.process_one(&cancel).await.unwrap());
    assert_eq!(
        store.get_ref(id, "main").await.unwrap().unwrap().status,
        citadel_git::GitRepositoryRefStatus::Degraded
    );
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
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    pool.close().await;
}
