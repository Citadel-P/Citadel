use citadel_database::{MigrationError, MigrationRunner};
use sqlx::{Connection, PgConnection, Row};

mod parity;

fn migration_count() -> usize {
    serde_json::from_str::<serde_json::Value>(citadel_database::MIGRATION_MANIFEST_JSON).unwrap()["migrations"].as_array().unwrap().len()
}

fn database_url() -> String {
    std::env::var("CITADEL_TEST_DATABASE_URL")
        .expect("CITADEL_TEST_DATABASE_URL is required for this integration test")
}

#[tokio::test]
#[ignore = "requires a fresh disposable CITADEL_TEST_DATABASE_URL"]
async fn instance_updates_upgrade_preserves_existing_settings_and_migration_history() {
    let url = database_url();
    let mut connection = PgConnection::connect(&url).await.unwrap();
    // Build the actual preceding catalog, never alter a current migration journal.
    sqlx::raw_sql(include_str!("../generated/0001_initial.sql"))
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY, name text NOT NULL, checksum text NOT NULL, started_at timestamptz NOT NULL, completed_at timestamptz, failure text);
        INSERT INTO citadel_schema_migrations VALUES
        ('0001','initial','42d6600f4892a48a87b6014097c1c461f5dca298974a64fdcd30388262301128',now(),now(),NULL);
        UPDATE instancesetupstates SET updatechecksenabled=false, mfapolicy='RequiredForAdministrators';")
        .execute(&mut connection).await.unwrap();
    let before: Vec<(String, String)> =
        sqlx::query_as("SELECT id, checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let result = MigrationRunner::migrate(&url).await.unwrap();
    assert_eq!(result.applied, migration_count() - 1);
    assert_eq!(result.already_applied, 1);
    let settings: (bool, String, i32) = sqlx::query_as("SELECT updatechecksenabled, mfapolicy, updatecheckintervalhours FROM instancesetupstates WHERE id=1")
        .fetch_one(&mut connection).await.unwrap();
    assert_eq!(settings, (false, "RequiredForAdministrators".into(), 6));
    let retained: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, checksum FROM citadel_schema_migrations WHERE id = '0001' ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(retained, before);
    assert!(
        sqlx::query("UPDATE instancesetupstates SET updatecheckintervalhours=7")
            .execute(&mut connection)
            .await
            .is_err()
    );
    assert_eq!(MigrationRunner::migrate(&url).await.unwrap().applied, 0);
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL"]
async fn clean_install_restart_and_checksum_enforcement() {
    let database_url = database_url();

    let (left, right) = tokio::join!(
        MigrationRunner::migrate(&database_url),
        MigrationRunner::migrate(&database_url)
    );
    let outcomes = [left.unwrap(), right.unwrap()];
    assert_eq!(
        outcomes
            .iter()
            .map(|outcome| outcome.applied)
            .sum::<usize>(),
        migration_count(),
        "the advisory lock must serialize concurrent startup"
    );

    let second = MigrationRunner::migrate(&database_url)
        .await
        .expect("restart migration should be idempotent");
    assert_eq!(second.applied, 0);
    assert_eq!(second.already_applied, migration_count());

    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    let tables: std::collections::BTreeSet<String> = sqlx::query_scalar::<_, String>(
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public'",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap()
    .into_iter()
    .collect();
    let mut expected: std::collections::BTreeSet<String> = citadel_database::SCHEMA_SQL
        .lines()
        .filter_map(|line| line.strip_prefix("CREATE TABLE "))
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect();
    expected.insert("citadel_schema_migrations".into());
    assert_eq!(
        tables, expected,
        "the migrated tables must match the declarative schema plus the journal"
    );
    let description_tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name::text FROM information_schema.columns WHERE table_schema='public' AND table_name IN ('backuprepositories','backuppolicies') AND column_name='description' ORDER BY table_name",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(description_tables, vec!["backuppolicies"]);

    let outbox_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public.alertdeliveryoutbox') IS NOT NULL")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert!(outbox_exists);

    let role_count: i64 = sqlx::query("SELECT count(*) AS count FROM roles")
        .fetch_one(&mut connection)
        .await
        .unwrap()
        .try_get("count")
        .unwrap();
    assert_eq!(role_count, 3);

    sqlx::query(
        "INSERT INTO citadel_schema_migrations (id, name, checksum, started_at) VALUES ('9999', 'unknown', 'unknown', CURRENT_TIMESTAMP)",
    )
    .execute(&mut connection)
    .await
    .unwrap();
    drop(connection);
    assert!(matches!(
        MigrationRunner::migrate(&database_url).await,
        Err(MigrationError::UnknownMigration(id)) if id == "9999"
    ));

    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query("DELETE FROM citadel_schema_migrations WHERE id = '9999'")
        .execute(&mut connection)
        .await
        .unwrap();

    sqlx::query("UPDATE citadel_schema_migrations SET checksum = 'tampered' WHERE id = '0001'")
        .execute(&mut connection)
        .await
        .unwrap();
    drop(connection);

    assert!(matches!(
        MigrationRunner::migrate(&database_url).await,
        Err(MigrationError::AppliedChecksum { .. })
    ));
}
