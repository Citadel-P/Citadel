use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use citadel_adapters::crypto::AesGcmSecretProtector;
use citadel_adapters::git_account_store::PostgresGitAccountStore;
use citadel_adapters::git_repository_execution_store::PostgresGitRepositoryExecutionStore;
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_git::{
    GitAccountService, GitCli, GitRepositoryExecutionService, GitRepositoryExecutionStore,
    SyncResult,
};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

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
    let store = Arc::new(PostgresGitRepositoryExecutionStore::new(pool.clone()));

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

    let root = std::env::temp_dir().join(format!("citadel-phase7-git-{}", Uuid::now_v7()));
    let remote = root.join("remote");
    let cache = root.join("cache");
    std::fs::create_dir_all(&remote).unwrap();
    git(&remote, &["init", "--initial-branch=main"]);
    git(&remote, &["config", "user.email", "citadel@example.test"]);
    git(&remote, &["config", "user.name", "Citadel Test"]);
    std::fs::write(remote.join("compose.yaml"), b"services: {}\n").unwrap();
    git(&remote, &["add", "compose.yaml"]);
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
        Arc::new(PostgresGitAccountStore::new(pool.clone())),
        Arc::new(AesGcmSecretProtector::new(&[91_u8; 32]).unwrap()),
    ));
    let service = GitRepositoryExecutionService::new(
        store,
        accounts,
        Arc::new(GitCli::new(Duration::from_secs(10))),
        cache,
        Duration::from_secs(60),
    );
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
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activityevents WHERE resourceid=$1 AND eventtype='GitRepoCloned' AND status='Success'",
    )
    .bind(repository_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity_count, 1);

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
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor)
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
