//! Empty workers must not enter recovery transactions or load operation payloads.
use citadel_adapters::persistence::postgres::{
    backups::PostgresBackupPersistence, builds::PostgresBuildRepository,
    resource_copies::PostgresCopyRepository, updates::PostgresUpdateRepository,
};
use citadel_backups::{BackupPersistence, RestoreClaimScope};
use citadel_builds::BuildRepository;
use citadel_resource_copies::CopyRepository;
use citadel_updates::AgentUpdateRepository;

#[tokio::test]
#[ignore = "requires fresh disposable CITADEL_EXECUTION_DATABASE_URL with pg_stat_statements preloaded"]
async fn empty_workers_use_read_only_hints_without_recovery_transactions() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_stat_statements")
        .execute(&pool)
        .await
        .unwrap();
    async fn calls(pool: &sqlx::PgPool, query: &str) -> i64 {
        sqlx::query_scalar("SELECT COALESCE(sum(calls),0)::bigint FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND query LIKE $1")
            .bind(query).fetch_one(pool).await.unwrap()
    }
    let begins = calls(&pool, "BEGIN%").await;
    let updates = calls(&pool, "UPDATE %").await;
    let full_state = calls(&pool, "SELECT operation FROM agentupdateoperations%").await;
    let builds = PostgresBuildRepository::new(pool.clone());
    let backups = PostgresBackupPersistence::new(pool.clone());
    let agents = PostgresUpdateRepository::new(pool.clone());
    let copies = PostgresCopyRepository::new(pool.clone());
    let before = chrono::Utc::now() - chrono::Duration::minutes(10);
    for _ in 0..3 {
        copies.recover_current().await.unwrap();
        assert!(copies.pending(None).await.unwrap().is_empty());
        assert!(!agents.pending_work().await.unwrap().active);
        assert!(builds.claim_next(before).await.unwrap().is_none());
        assert!(builds.next_retry_at().await.unwrap().is_none());
        assert!(backups.claim_backup(before).await.unwrap().is_none());
        assert!(
            backups
                .claim_restore(before, RestoreClaimScope::All)
                .await
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(calls(&pool, "BEGIN%").await, begins);
    assert_eq!(calls(&pool, "UPDATE %").await, updates);
    assert_eq!(
        calls(&pool, "SELECT operation FROM agentupdateoperations%").await,
        full_state
    );
    pool.close().await;
}
