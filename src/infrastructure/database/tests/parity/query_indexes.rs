use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn query_index_upgrade_preserves_history_and_bounds_activity_page() {
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
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY, name text NOT NULL, checksum text NOT NULL, started_at timestamptz NOT NULL, completed_at timestamptz, failure text)")
        .execute(&mut connection).await.unwrap();
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
    sqlx::raw_sql("INSERT INTO activityevents(id,createdat,createdbyactorid,eventtype,info,resourcename,resourcetype,status)
        SELECT gen_random_uuid(),now()-(n/10)*interval '1 minute','00000000-0000-0000-0000-000000000001','ContainerStarted','{}','fixture','Platform','Success' FROM generate_series(1,100000) n;
        ANALYZE activityevents;")
        .execute(&mut connection).await.unwrap();
    let before: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let page: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM activityevents ORDER BY createdat DESC,id DESC LIMIT 50",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    let outcome = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(
        (outcome.applied, outcome.already_applied),
        (super::super::migration_count() - 3, 3)
    );
    let after: Vec<(String,String)> = sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations WHERE id IN ('0001','0002','0003') ORDER BY id")
        .fetch_all(&mut connection).await.unwrap();
    assert_eq!(before, after);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(count, 100000);
    let after_page: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM activityevents ORDER BY createdat DESC,id DESC LIMIT 50",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(
        page, after_page,
        "ordering ties must preserve the same page"
    );
    let plan: serde_json::Value = sqlx::query_scalar("EXPLAIN (ANALYZE, FORMAT JSON) SELECT id FROM activityevents ORDER BY createdat DESC,id DESC LIMIT 50")
        .fetch_one(&mut connection).await.unwrap();
    let scan = &plan[0]["Plan"]["Plans"][0];
    assert_eq!(scan["Index Name"], "ix_activityevents_createdat_id");
    assert_eq!(scan["Actual Rows"].as_f64(), Some(50.));
    let indexes: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_index WHERE indisvalid AND indisready AND indexrelid IN ('ix_activityevents_createdat_id'::regclass,'ix_scriptruns_retention'::regclass,'ix_buildruns_retention'::regclass,'ix_resourcecopies_review_expiry'::regclass)")
        .fetch_one(&mut connection).await.unwrap();
    assert_eq!(indexes, 4);
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
