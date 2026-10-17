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
        6
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
    assert_eq!(first.applied, 6);
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
    assert_eq!(result.already_applied, 6);
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

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn initial_baseline_includes_appearance_defaults_and_restart_preserves_preferences() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    let installed = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(installed.applied, 6);
    assert_eq!(installed.already_applied, 0);
    let user = Uuid::now_v7();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,name,email) VALUES ($1,$2,$2,'existing-user','existing@example.test')").bind(user).bind(Uuid::from_u128(1)).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO userpreferences(userid,timezone,datetimeformat,theme,updatedat) VALUES ($1,'Europe/Paris','TwentyFourHour','Dark','2026-01-01T00:00:00Z')").bind(user).execute(&mut connection).await.unwrap();
    let outcome = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(outcome.applied, 0);
    assert_eq!(outcome.already_applied, 6);
    let row: (String,String,String,String,String,String,String,String) = sqlx::query_as("SELECT timezone,datetimeformat,theme,themecolor,font,radius,contentlayout,density FROM userpreferences WHERE userid=$1").bind(user).fetch_one(&mut connection).await.unwrap();
    assert_eq!(
        row,
        (
            "Europe/Paris".into(),
            "TwentyFourHour".into(),
            "Dark".into(),
            "Neutral".into(),
            "Geist".into(),
            "None".into(),
            "Full".into(),
            "Compact".into()
        )
    );
    let preserved: bool = sqlx::query_scalar(
        "SELECT updatedat='2026-01-01T00:00:00Z'::timestamptz FROM userpreferences WHERE userid=$1",
    )
    .bind(user)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert!(preserved);
    assert_eq!(
        MigrationRunner::migrate(&database.url)
            .await
            .unwrap()
            .applied,
        0
    );
    drop(connection);
    database.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn initial_baseline_includes_image_updates_and_restart_preserves_observations() {
    let database = Database::create().await;
    MigrationRunner::migrate(&database.url).await.unwrap();
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture.sock','Local',0,1,0,'baseline-fixture',0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).execute(&mut connection).await.unwrap();
    let image = Uuid::now_v7();
    sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags) VALUES($1,'sha256:existing','app',$2,now(),'[\"app:latest\"]')").bind(image).bind(platform).execute(&mut connection).await.unwrap();
    let checksum: String =
        sqlx::query_scalar("SELECT checksum FROM citadel_schema_migrations WHERE id='0001'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    sqlx::query("INSERT INTO imageupdateobservations(imageid,reference,registrykey,registryrevision,attemptedat,result) VALUES($1,'app:latest',$2,'anonymous',now(),'{\"status\":\"UpToDate\"}')")
        .bind(image).bind(Uuid::nil()).execute(&mut connection).await.unwrap();
    let restarted = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!((restarted.applied, restarted.already_applied), (0, 6));
    let status: String = sqlx::query_scalar(
        "SELECT result->>'status' FROM imageupdateobservations WHERE imageid=$1",
    )
    .bind(image)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(status, "UpToDate");
    let preserved: String = sqlx::query_scalar("SELECT tags->>0 FROM images WHERE id=$1")
        .bind(image)
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(preserved, "app:latest");
    let current: String =
        sqlx::query_scalar("SELECT checksum FROM citadel_schema_migrations WHERE id='0001'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(current, checksum);
    assert_eq!(
        MigrationRunner::migrate(&database.url)
            .await
            .unwrap()
            .already_applied,
        6
    );
    connection.close().await.unwrap();
    database.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn health_upgrade_preserves_existing_containers_and_baseline_checksum() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    // Stop immediately after the real published baseline has committed, without editing its SQL.
    sqlx::raw_sql("CREATE TABLE containerhealthtransitions(blocker bool)")
        .execute(&mut connection)
        .await
        .unwrap();
    assert!(
        matches!(MigrationRunner::migrate(&database.url).await, Err(MigrationError::Apply {id, ..}) if id == "0002")
    );
    let checksum: String = sqlx::query_scalar("SELECT checksum FROM citadel_schema_migrations WHERE id='0001' AND completed_at IS NOT NULL").fetch_one(&mut connection).await.unwrap();
    let platform = Uuid::now_v7();
    let container = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture.sock','Local',0,0,0,'health-upgrade',0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports) VALUES($1,$2,'upgrade-id','image','preserved',1,1,'Running','[]')").bind(container).bind(platform).execute(&mut connection).await.unwrap();
    sqlx::raw_sql("DROP TABLE containerhealthtransitions")
        .execute(&mut connection)
        .await
        .unwrap();
    let result = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(result.applied, 5);
    assert_eq!(result.already_applied, 1);
    let row: (String, String, Option<serde_json::Value>) =
        sqlx::query_as("SELECT name,state,health FROM containers WHERE id=$1")
            .bind(container)
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(row, ("preserved".into(), "Running".into(), None));
    let after: String =
        sqlx::query_scalar("SELECT checksum FROM citadel_schema_migrations WHERE id='0001'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(checksum, after);
    assert_eq!(
        MigrationRunner::migrate(&database.url)
            .await
            .unwrap()
            .applied,
        0
    );
    // Exercise the installation cap in this isolated database, including concurrent-safe admission.
    sqlx::query("INSERT INTO containerhealthtransitions(containerid,platformid,dockercontainerid,observedat,state,origin) SELECT gen_random_uuid(),$1,'retained',1,'Running','Lifecycle' FROM generate_series(1,100005)")
        .bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("UPDATE containers SET state='Exited' WHERE id=$1")
        .bind(container)
        .execute(&mut connection)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM containerhealthtransitions")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(count, 100000);
    drop(connection);
    database.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn periodic_health_upgrade_preserves_transition_history() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    // Stop at the real 0002 schema by blocking only the next migration's index.
    sqlx::raw_sql("CREATE TABLE ix_containerhealthtransitions_samples(blocker bool)")
        .execute(&mut connection)
        .await
        .unwrap();
    assert!(
        matches!(MigrationRunner::migrate(&database.url).await, Err(MigrationError::Apply {id, ..}) if id == "0003")
    );
    let checksum: String =
        sqlx::query_scalar("SELECT checksum FROM citadel_schema_migrations WHERE id='0002'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    let platform = Uuid::now_v7();
    let container = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture.sock','Local',0,0,0,'sample-upgrade',0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports) VALUES($1,$2,'upgrade-id','image','preserved',1,1,'Running','[]')").bind(container).bind(platform).execute(&mut connection).await.unwrap();
    let before: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(h) FROM containerhealthtransitions h WHERE containerid=$1",
    )
    .bind(container)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    sqlx::raw_sql("DROP TABLE ix_containerhealthtransitions_samples")
        .execute(&mut connection)
        .await
        .unwrap();
    let result = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!((result.applied, result.already_applied), (4, 2));
    let after: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(h) FROM containerhealthtransitions h WHERE containerid=$1",
    )
    .bind(container)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(before, after);
    let after_checksum: String =
        sqlx::query_scalar("SELECT checksum FROM citadel_schema_migrations WHERE id='0002'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(checksum, after_checksum);
    connection.close().await.unwrap();
    database.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn workload_health_identity_upgrade_keeps_existing_health_and_old_checksums() {
    let database = Database::create().await;
    MigrationRunner::migrate(&database.url).await.unwrap();
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    // Reconstruct the immediately preceding schema in this disposable fixture only.
    sqlx::raw_sql("ALTER TABLE containers DROP COLUMN composeservicename, DROP COLUMN healthstoprequestedat; ALTER TABLE stacks DROP COLUMN healthappliedspec, DROP COLUMN healthappliedplatformid; DELETE FROM citadel_schema_migrations WHERE id>='0004';").execute(&mut connection).await.unwrap();
    restore_previous_health_writer(&mut connection).await;
    let before: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let platform = Uuid::now_v7();
    let container = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture.sock','Local',0,0,0,'identity-upgrade',0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,health) VALUES($1,$2,'existing','image','kept',1,1,'Running','[]','{\"configuration\":\"Enabled\",\"result\":\"Healthy\",\"observedAt\":1000}')").bind(container).bind(platform).execute(&mut connection).await.unwrap();
    let upgraded = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!((upgraded.applied, upgraded.already_applied), (3, 3));
    let after: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,checksum FROM citadel_schema_migrations WHERE id<'0004' ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(before, after);
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT health->>'result',composeservicename FROM containers WHERE id=$1")
            .bind(container)
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(row, ("Healthy".into(), None));
    drop(connection);
    database.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn workload_health_evidence_upgrade_does_not_guess_applied_topology_or_stop_intent() {
    let database = Database::create().await;
    MigrationRunner::migrate(&database.url).await.unwrap();
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    // Reconstruct 0004 only inside this disposable database, preserving its journal.
    sqlx::raw_sql("ALTER TABLE stacks DROP COLUMN healthappliedspec, DROP COLUMN healthappliedplatformid; ALTER TABLE containers DROP COLUMN healthstoprequestedat; DELETE FROM citadel_schema_migrations WHERE id>='0005';")
        .execute(&mut connection).await.unwrap();
    restore_previous_health_writer(&mut connection).await;
    let checksums: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let platform = Uuid::now_v7();
    let container = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture.sock','Local',0,0,0,'evidence-upgrade',0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,composeservicename) VALUES($1,$2,'existing','image','kept',1,1,'Exited','[]','web')")
        .bind(container).bind(platform).execute(&mut connection).await.unwrap();
    let result = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!((result.applied, result.already_applied), (2, 4));
    let row: (String, String, Option<i64>) = sqlx::query_as(
        "SELECT state,composeservicename,healthstoprequestedat FROM containers WHERE id=$1",
    )
    .bind(container)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(row, ("Exited".into(), "web".into(), None));
    let after: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,checksum FROM citadel_schema_migrations WHERE id<'0005' ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(checksums, after);
    assert_eq!(
        MigrationRunner::migrate(&database.url)
            .await
            .unwrap()
            .applied,
        0
    );
    drop(connection);
    database.close().await;
}

// Reconstruct only disposable fixtures; published migration SQL stays immutable.
async fn restore_previous_health_writer(connection: &mut PgConnection) {
    let sql = include_str!("../../generated/0003_container_health_samples.sql");
    let start = sql.find("CREATE OR REPLACE FUNCTION").unwrap();
    sqlx::raw_sql(&sql[start..])
        .execute(connection)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn health_sampling_upgrade_preserves_history_and_only_samples_enabled_checks() {
    let database = Database::create().await;
    MigrationRunner::migrate(&database.url).await.unwrap();
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    restore_previous_health_writer(&mut connection).await;
    sqlx::query("DELETE FROM citadel_schema_migrations WHERE id='0006'")
        .execute(&mut connection)
        .await
        .unwrap();
    let checksums: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let platform = Uuid::now_v7();
    let existing = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture.sock','Local',0,0,0,'sampling-upgrade',0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,health) VALUES($1,$2,'existing','image','kept',1,1,'Running','[]','{\"configuration\":\"NoCheck\",\"observedAt\":1000}')")
        .bind(existing).bind(platform).execute(&mut connection).await.unwrap();
    let before: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(h) FROM containerhealthtransitions h WHERE containerid=$1",
    )
    .bind(existing)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    let upgraded = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!((upgraded.applied, upgraded.already_applied), (1, 5));
    let after: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(h) FROM containerhealthtransitions h WHERE containerid=$1",
    )
    .bind(existing)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(before, after);
    let after_checksums: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,checksum FROM citadel_schema_migrations WHERE id<'0006' ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(checksums, after_checksums);

    let minute: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM clock_timestamp()) / 60)::bigint * 60000",
    )
    .fetch_one(&mut connection)
    .await
    .unwrap();
    for configuration in ["NoCheck", "Disabled", "Unknown", "Enabled"] {
        let container = Uuid::now_v7();
        let mut evidence =
            serde_json::json!({"configuration": configuration, "observedAt": minute - 120000});
        sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,health) VALUES($1,$2,$1::text,'image','sample',1,1,'Running','[]',$3)")
            .bind(container).bind(platform).bind(&evidence).execute(&mut connection).await.unwrap();
        for stamp in [minute - 60000, minute] {
            evidence["observedAt"] = serde_json::json!(stamp);
            sqlx::query("UPDATE containers SET health=$2 WHERE id=$1")
                .bind(container)
                .bind(&evidence)
                .execute(&mut connection)
                .await
                .unwrap();
        }
        let samples: i64 = sqlx::query_scalar("SELECT count(*) FROM containerhealthtransitions WHERE containerid=$1 AND origin='Sample'")
            .bind(container).fetch_one(&mut connection).await.unwrap();
        assert_eq!(
            samples,
            if configuration == "Enabled" { 2 } else { 0 },
            "{configuration}"
        );
        let current: serde_json::Value =
            sqlx::query_scalar("SELECT health FROM containers WHERE id=$1")
                .bind(container)
                .fetch_one(&mut connection)
                .await
                .unwrap();
        assert_eq!(current, evidence, "current evidence must still refresh");
        evidence["configuration"] = serde_json::json!(if configuration == "Enabled" {
            "Disabled"
        } else {
            "Enabled"
        });
        sqlx::query("UPDATE containers SET health=$2 WHERE id=$1")
            .bind(container)
            .bind(&evidence)
            .execute(&mut connection)
            .await
            .unwrap();
        sqlx::query("UPDATE containers SET state='Exited' WHERE id=$1")
            .bind(container)
            .execute(&mut connection)
            .await
            .unwrap();
        let changes: i64 = sqlx::query_scalar("SELECT count(*) FROM containerhealthtransitions WHERE containerid=$1 AND origin<>'Sample'")
            .bind(container).fetch_one(&mut connection).await.unwrap();
        assert_eq!(
            changes, 3,
            "initial observation, configuration change and exit remain recorded"
        );
    }
    drop(connection);
    database.close().await;
}
