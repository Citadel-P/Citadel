use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn dashboard_upgrade_preserves_existing_role_grants_and_seeds_the_example_home() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY,name text NOT NULL,checksum text NOT NULL,started_at timestamptz NOT NULL,completed_at timestamptz,failure text)")
        .execute(&mut connection).await.unwrap();
    let source_catalog = [
        include_str!("../../generated/0001_initial.sql"),
        include_str!("../../generated/0002_instance_updates.sql"),
        include_str!("../../generated/0003_platform_io.sql"),
        include_str!("../../generated/0004_query_indexes.sql"),
        include_str!("../../generated/0005_health_history_retention.sql"),
        include_str!("../../generated/0006_memory_breakdown.sql"),
        include_str!("../../generated/0007_platform_coarse_history.sql"),
        include_str!("../../generated/0008_job_diagnostics.sql"),
        include_str!("../../generated/0009_job_history_write_overhead.sql"),
        include_str!("../../generated/0010_image_update_schedule.sql"),
    ];
    for (sql, migration) in source_catalog
        .into_iter()
        .zip(manifest["migrations"].as_array().unwrap())
    {
        sqlx::raw_sql(sql).execute(&mut connection).await.unwrap();
        sqlx::query("INSERT INTO citadel_schema_migrations VALUES($1,$2,$3,now(),now(),NULL)")
            .bind(migration["id"].as_str().unwrap())
            .bind(migration["name"].as_str().unwrap())
            .bind(migration["sha256"].as_str().unwrap())
            .execute(&mut connection)
            .await
            .unwrap();
    }
    let role = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO roles(id,name,roletype) VALUES ($1,'Dashboard upgrade fixture','Custom')",
    )
    .bind(role)
    .execute(&mut connection)
    .await
    .unwrap();
    let permission = Uuid::now_v7();
    sqlx::query("INSERT INTO permissions(id,permissionlevel,resourcetype,roleid,specificpermissions) VALUES ($1,1,0,$2,0)")
        .bind(permission).bind(role).execute(&mut connection).await.unwrap();
    let migration = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(migration.already_applied, 10);
    assert_eq!(migration.applied, 1);
    let retained: bool = sqlx::query_scalar("SELECT EXISTS (SELECT FROM permissions WHERE id=$1 AND roleid=$2 AND resourcetype=0 AND permissionlevel=1)")
        .bind(permission).bind(role)
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert!(retained);
    let counts: (i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM dashboards), (SELECT count(*) FROM instancedashboarddefault), (SELECT count(*) FROM personaldashboarddefaults), (SELECT count(*) FROM permissions WHERE resourcetype=22)")
        .fetch_one(&mut connection).await.unwrap();
    assert_eq!(counts, (1, 1, 0, 0));
    let home: (String, String) = sqlx::query_as(
        "SELECT name, visibility FROM dashboards JOIN instancedashboarddefault ON id = dashboardid",
    )
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(home, ("Overview".into(), "InstanceShared".into()));
    let duplicate = sqlx::query(
        "INSERT INTO dashboards(id, name, visibility, configuration)
         VALUES ($1, 'overview', 'InstanceShared', '{}')",
    )
    .bind(Uuid::now_v7())
    .execute(&mut connection)
    .await
    .unwrap_err();
    assert_eq!(
        duplicate.as_database_error().unwrap().constraint(),
        Some("ix_dashboards_unique_name")
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
