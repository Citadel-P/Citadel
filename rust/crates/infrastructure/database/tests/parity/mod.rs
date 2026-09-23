use citadel_database::{MigrationError, MigrationRunner};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

// Dedicated child databases keep these destructive migration probes independent
// of each other and of the existing checksum-tampering test.
struct Database {
    admin_url: String,
    url: String,
    name: String,
}

impl Database {
    async fn create() -> Self {
        let admin_url = super::database_url();
        let name = format!("citadel_migration_{}", Uuid::now_v7().simple());
        let mut admin = PgConnection::connect(&admin_url).await.unwrap();
        // Identifier consists solely of our literal prefix and UUID hex digits.
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut admin)
            .await
            .unwrap();
        let mut url = url::Url::parse(&admin_url).unwrap();
        url.set_path(&name);
        Self {
            admin_url,
            url: url.into(),
            name,
        }
    }

    async fn close(self) {
        let mut admin = PgConnection::connect(&self.admin_url).await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP DATABASE {} WITH (FORCE)",
            self.name
        )))
        .execute(&mut admin)
        .await
        .unwrap();
    }
}

// PreReleaseUpgradeTests.FailingMigration_ShouldRollbackSchemaAndRemainUnjournaled.
// The Rust journal deliberately retains bounded failed-attempt diagnostics;
// unlike DbUp, it must have no *completed* entry, not no diagnostic row.
#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn failed_baseline_rolls_back_schema_preserves_data_and_can_retry() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    // Force failure partway through the real embedded baseline, without changing
    // its SQL/checksum or adding a test-only production migration API.
    sqlx::raw_sql("CREATE TABLE roles (marker text); INSERT INTO roles VALUES ('preserved');")
        .execute(&mut connection)
        .await
        .unwrap();
    let error = MigrationRunner::migrate(&database.url).await.unwrap_err();
    assert!(matches!(error, MigrationError::Apply { ref id, .. } if id == "0001"));
    let actors: Option<String> = sqlx::query_scalar("SELECT to_regclass('public.actors')::text")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert!(
        actors.is_none(),
        "DDL before the failing statement must roll back"
    );
    let marker: String = sqlx::query_scalar("SELECT marker FROM roles")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(marker, "preserved");
    let (completed, failed): (bool, bool) = sqlx::query_as(
        "SELECT completed_at IS NOT NULL, failure IS NOT NULL FROM citadel_schema_migrations WHERE id='0001'",
    ).fetch_one(&mut connection).await.unwrap();
    assert!(!completed);
    assert!(failed);
    sqlx::query("DROP TABLE roles")
        .execute(&mut connection)
        .await
        .unwrap();
    assert_eq!(
        MigrationRunner::migrate(&database.url)
            .await
            .unwrap()
            .applied,
        1
    );
    let (completed, failed): (bool, bool) = sqlx::query_as(
        "SELECT completed_at IS NOT NULL, failure IS NOT NULL FROM citadel_schema_migrations WHERE id='0001'",
    ).fetch_one(&mut connection).await.unwrap();
    assert!(completed);
    assert!(
        !failed,
        "successful retry clears failed-attempt diagnostics"
    );
    assert_eq!(
        MigrationRunner::migrate(&database.url)
            .await
            .unwrap()
            .applied,
        0
    );
    connection.close().await.unwrap();
    database.close().await;
}

async fn action(connection: &mut PgConnection) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO actions(id,alertonfailure,code,createdbyactorid,enabled,name,runasactorid,scheduleenabled,scheduletimezone,timeoutseconds) VALUES($1,FALSE,'','00000000-0000-0000-0000-000000000001',TRUE,$2,'00000000-0000-0000-0000-000000000001',FALSE,'UTC',60)")
        .bind(id).bind(id.to_string()).execute(connection).await.unwrap();
    id
}

async fn insert_run(
    connection: &mut PgConnection,
    action: Uuid,
    status: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO actionruns(id,actionid,actionname,codehash,codesnapshot,runasactorid,status,timeoutseconds,trigger) VALUES($1,$2,'parity','','','00000000-0000-0000-0000-000000000001',$3,60,'Manual')")
        .bind(id).bind(action).bind(status).execute(connection).await?;
    Ok(id)
}

fn assert_active_conflict(error: sqlx::Error) {
    let error = error
        .as_database_error()
        .expect("PostgreSQL constraint failure");
    assert_eq!(error.code().as_deref(), Some("23505"));
    assert_eq!(error.constraint(), Some("ix_actionruns_active_action"));
}

// PreReleaseUpgradeTests.CandidateMigration_ShouldEnforceSingleActiveActionRunPerAction,
// expanded to both active states and terminal-history/requeue behavior.
#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn database_enforces_active_run_state_matrix_and_allows_terminal_history() {
    let database = Database::create().await;
    MigrationRunner::migrate(&database.url).await.unwrap();
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    for active in ["Queued", "Running"] {
        for competing in ["Queued", "Running"] {
            let action = action(&mut connection).await;
            let first = insert_run(&mut connection, action, active).await.unwrap();
            assert_active_conflict(
                insert_run(&mut connection, action, competing)
                    .await
                    .unwrap_err(),
            );
            for terminal in ["Succeeded", "Failed", "Cancelled", "Rejected"] {
                insert_run(&mut connection, action, terminal).await.unwrap();
            }
            sqlx::query("UPDATE actionruns SET status='Succeeded' WHERE id=$1")
                .bind(first)
                .execute(&mut connection)
                .await
                .unwrap();
            insert_run(&mut connection, action, competing)
                .await
                .unwrap();
        }
    }
    connection.close().await.unwrap();
    database.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn concurrent_direct_inserts_allow_only_one_active_run_per_action() {
    let database = Database::create().await;
    MigrationRunner::migrate(&database.url).await.unwrap();
    let mut left = PgConnection::connect(&database.url).await.unwrap();
    let mut right = PgConnection::connect(&database.url).await.unwrap();
    let id = action(&mut left).await;
    let (left_result, right_result) =
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            tokio::join!(
                insert_run(&mut left, id, "Queued"),
                insert_run(&mut right, id, "Running")
            )
        })
        .await
        .expect("conflicting inserts must terminate without a deadlock");
    let results = [left_result, right_result];
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_active_conflict(results.into_iter().find_map(Result::err).unwrap());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM actionruns WHERE actionid=$1")
        .bind(id)
        .fetch_one(&mut left)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let other = action(&mut left).await;
    insert_run(&mut left, other, "Running").await.unwrap();
    left.close().await.unwrap();
    right.close().await.unwrap();
    database.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn initial_baseline_includes_job_state_and_restart_preserves_git_refs() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    let first = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(first.applied, 1);
    assert_eq!(first.already_applied, 0);
    let pending_column: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_name='platformstats' AND column_name='alertpending' AND column_default='false')")
        .fetch_one(&mut connection).await.unwrap();
    assert!(pending_column);
    let repository = Uuid::now_v7();
    let reference = Uuid::now_v7();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url) VALUES($1,$2,'main','migration-fixture','Healthy','Manual','https://example.invalid/repo')").bind(repository).bind(Uuid::from_u128(1)).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO gitrepositoryrefs(id,gitrepositoryid,branch,resolvedcommitsha,status,lastsyncedat) VALUES($1,$2,'main','retained-commit','Healthy',now())").bind(reference).bind(repository).execute(&mut connection).await.unwrap();
    let result = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(result.applied, 0);
    assert_eq!(result.already_applied, 1);
    let result: (String, String) =
        sqlx::query_as("SELECT synctrigger,resolvedcommitsha FROM gitrepositoryrefs WHERE id=$1")
            .bind(reference)
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(result, ("Manual".into(), "retained-commit".into()));
    drop(connection);
    database.close().await;
}
