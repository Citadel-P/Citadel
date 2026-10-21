use super::*;
use citadel_git::{
    GitRepositoryError, GitRepositoryMutationKind, GitRepositoryPatch, GitRepositoryPersistence,
};

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn sync_claims_fence_completion_preserve_other_branches_and_freeze_execution_configuration() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'System')")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    let id = Uuid::now_v7();
    insert_repository(
        &pool,
        actor,
        id,
        "https://example.invalid/repo.git",
        "queue-safety",
    )
    .await;
    sqlx::query("UPDATE gitrepositories SET onpull=$2 WHERE id=$1")
        .bind(id)
        .bind(r#"{"commands":["echo sync"],"path":"./"}"#)
        .execute(&pool)
        .await
        .unwrap();
    let store = PostgresGitRepositoryExecutionPersistence::new(pool.clone());
    let catalog = PostgresGitRepositoryPersistence::new(pool.clone());
    let actor_id = ActorId::new(actor);
    let cutoff = Utc::now() - chrono::Duration::minutes(10);
    let first = store.claim_next(cutoff).await.unwrap().unwrap();
    store
        .enqueue_sync(actor_id, id, Some("other"))
        .await
        .unwrap();
    for patch in [
        serde_json::json!({"onPull":null}),
        serde_json::json!({"url":"https://example.invalid/changed.git"}),
        serde_json::json!({"defaultBranch":"other"}),
    ] {
        let patch: GitRepositoryPatch = serde_json::from_value(patch).unwrap();
        assert!(matches!(
            catalog
                .update_git_repository(actor_id, id, &patch, GitRepositoryMutationKind::Update)
                .await,
            Err(GitRepositoryError::Conflict(_))
        ));
    }
    assert!(matches!(
        catalog.delete_git_repositories(actor_id, &[id]).await,
        Err(GitRepositoryError::Conflict(_))
    ));
    let metadata: GitRepositoryPatch =
        serde_json::from_value(serde_json::json!({"description":"safe while syncing"})).unwrap();
    catalog
        .update_git_repository(actor_id, id, &metadata, GitRepositoryMutationKind::Metadata)
        .await
        .unwrap();
    let result = SyncResult {
        commit: "a".repeat(40),
        cloned: false,
    };
    store.complete(&first, &result).await.unwrap();
    assert_eq!(
        store.get_ref(id, "main").await.unwrap().unwrap().status,
        citadel_git::GitRepositoryRefStatus::Healthy
    );
    let other = store.claim_next(cutoff).await.unwrap().unwrap();
    assert_eq!(
        other.branch, "other",
        "another branch must not rerun the completed hook"
    );
    store.complete(&other, &result).await.unwrap();
    assert!(store.claim_next(cutoff).await.unwrap().is_none());
    // A completion from an older attempt must not settle a newly claimed sync.
    store
        .enqueue_sync(actor_id, id, Some("main"))
        .await
        .unwrap();
    let newer = store.claim_next(cutoff).await.unwrap().unwrap();
    assert!(store.complete(&first, &result).await.is_err());
    assert!(store.fail(&first, "late failure").await.is_err());
    assert_eq!(
        store.get_ref(id, "main").await.unwrap().unwrap().status,
        citadel_git::GitRepositoryRefStatus::Syncing
    );
    store.complete(&newer, &result).await.unwrap();
    // Recovery leaves the interrupted branch for review, but services already queued branches.
    store
        .enqueue_sync(actor_id, id, Some("main"))
        .await
        .unwrap();
    let interrupted = store.claim_next(cutoff).await.unwrap().unwrap();
    store
        .enqueue_sync(actor_id, id, Some("other"))
        .await
        .unwrap();
    sqlx::query("UPDATE gitrepositoryrefs SET lastsyncedat=CURRENT_TIMESTAMP-INTERVAL '20 minutes' WHERE id=$1").bind(interrupted.reference_id).execute(&pool).await.unwrap();
    let recovered = PostgresGitRepositoryExecutionPersistence::new(pool.clone());
    let other = recovered
        .claim_next(cutoff)
        .await
        .unwrap()
        .expect("queued branch must remain claimable");
    assert_eq!(other.branch, "other");
    assert_eq!(
        store.get_ref(id, "main").await.unwrap().unwrap().status,
        citadel_git::GitRepositoryRefStatus::Degraded
    );
    store.complete(&other, &result).await.unwrap();
    assert!(store.claim_next(cutoff).await.unwrap().is_none());
    catalog
        .delete_git_repositories(actor_id, &[id])
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
    pool.close().await;
}
