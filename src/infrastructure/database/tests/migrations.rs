use citadel_database::{MigrationError, MigrationRunner};
use sqlx::{Connection, PgConnection, Row};

mod parity;

fn database_url() -> String {
    std::env::var("CITADEL_TEST_DATABASE_URL")
        .expect("CITADEL_TEST_DATABASE_URL is required for this integration test")
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
        1,
        "the advisory lock must serialize concurrent startup"
    );

    let second = MigrationRunner::migrate(&database_url)
        .await
        .expect("restart migration should be idempotent");
    assert_eq!(second.applied, 0);
    assert_eq!(second.already_applied, 1);

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
