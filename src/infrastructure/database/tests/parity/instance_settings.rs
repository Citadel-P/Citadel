use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn instance_settings_upgrade_preserves_setup_choices_and_applied_history() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    sqlx::raw_sql(concat!(
        include_str!("../../generated/0001_initial.sql"),
        "\n",
        include_str!("../../generated/0002_instance_updates.sql"),
        "\n",
        include_str!("../../generated/0003_platform_io.sql")
    ))
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY,name text NOT NULL,checksum text NOT NULL,started_at timestamptz NOT NULL,completed_at timestamptz,failure text);
        UPDATE instancesetupstates SET updatechecksenabled=false,updatecheckintervalhours=24,mfapolicy='RequiredForAdministrators';").execute(&mut connection).await.unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    for migration in manifest["migrations"].as_array().unwrap().iter().take(3) {
        sqlx::query("INSERT INTO citadel_schema_migrations VALUES($1,$2,$3,now(),now(),NULL)")
            .bind(migration["id"].as_str().unwrap())
            .bind(migration["name"].as_str().unwrap())
            .bind(migration["sha256"].as_str().unwrap())
            .execute(&mut connection)
            .await
            .unwrap();
    }
    let before: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let result = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(result.applied, 1);
    assert_eq!(result.already_applied, 3);
    let settings: (bool,i32,String,serde_json::Value) = sqlx::query_as("SELECT updatechecksenabled,updatecheckintervalhours,mfapolicy,instanceconfiguration FROM instancesetupstates WHERE id=1").fetch_one(&mut connection).await.unwrap();
    assert_eq!(
        settings,
        (
            false,
            24,
            "RequiredForAdministrators".into(),
            serde_json::json!({})
        )
    );
    let retained: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,checksum FROM citadel_schema_migrations WHERE id<='0003' ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(before, retained);
    assert!(
        sqlx::query("UPDATE instancesetupstates SET instanceconfiguration='[]'")
            .execute(&mut connection)
            .await
            .is_err()
    );
    sqlx::query("UPDATE instancesetupstates SET instanceconfiguration='{\"scriptsEnabled\":false,\"scriptsDefaultTimeoutSeconds\":42}'").execute(&mut connection).await.unwrap();
    assert_eq!(
        MigrationRunner::migrate(&database.url)
            .await
            .unwrap()
            .applied,
        0
    );
    let saved: serde_json::Value =
        sqlx::query_scalar("SELECT instanceconfiguration FROM instancesetupstates WHERE id=1")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(saved["scriptsEnabled"], false);
    assert_eq!(saved["scriptsDefaultTimeoutSeconds"], 42);
    connection.close().await.unwrap();
    database.close().await;
}
