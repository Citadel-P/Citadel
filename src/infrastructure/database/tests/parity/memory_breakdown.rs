use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn memory_breakdown_upgrade_preserves_samples_and_applied_checksums() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    sqlx::raw_sql(concat!(
        include_str!("../../generated/0001_initial.sql"),
        "\n",
        include_str!("../../generated/0002_instance_updates.sql"),
        "\n",
        include_str!("../../generated/0003_platform_io.sql"),
        "\n",
        include_str!("../../generated/0004_query_indexes.sql"),
        "\n",
        include_str!("../../generated/0005_health_history_retention.sql")
    ))
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY,name text NOT NULL,checksum text NOT NULL,started_at timestamptz NOT NULL,completed_at timestamptz,failure text)").execute(&mut connection).await.unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    for m in manifest["migrations"].as_array().unwrap().iter().take(5) {
        sqlx::query("INSERT INTO citadel_schema_migrations VALUES($1,$2,$3,now(),now(),NULL)")
            .bind(m["id"].as_str().unwrap())
            .bind(m["name"].as_str().unwrap())
            .bind(m["sha256"].as_str().unwrap())
            .execute(&mut connection)
            .await
            .unwrap();
    }
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,name,platformdescriptor,status,cpucount,imagecount,memtotal,networkcount,volumecount) VALUES($1,$1::text,'Local','memory','{\"$type\":\"Docker\"}','Online',1,0,1000,0,0)").bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("WITH s AS (INSERT INTO statisticssources(platformid,containerid) VALUES($1,$2) RETURNING id)
        INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
        SELECT id,100,1,800,200,1000,0,0 FROM s").bind(platform).bind(Uuid::now_v7()).execute(&mut connection).await.unwrap();
    let before: serde_json::Value =
        sqlx::query_scalar("SELECT to_jsonb(s) FROM statisticssamples s")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    let checksums: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let outcome = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(outcome.already_applied, 5);
    assert_eq!(outcome.applied, super::super::migration_count() - 5);
    let after: serde_json::Value =
        sqlx::query_scalar("SELECT to_jsonb(s)-'memoryactivecache' FROM statisticssamples s")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(before, after);
    let unknown: bool =
        sqlx::query_scalar("SELECT memoryactivecache IS NULL FROM statisticsobservations")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert!(unknown);
    let after_checksums: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,checksum FROM citadel_schema_migrations WHERE id<'0006' ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(checksums, after_checksums);
    connection.close().await.unwrap();
    database.close().await;
}
