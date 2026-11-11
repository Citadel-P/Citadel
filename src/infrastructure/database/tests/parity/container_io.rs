use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn block_io_upgrade_preserves_recorded_statistics_and_applied_migrations() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    sqlx::raw_sql(include_str!("../../generated/0001_initial.sql"))
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY, name text NOT NULL, checksum text NOT NULL, started_at timestamptz NOT NULL, completed_at timestamptz, failure text)")
        .execute(&mut connection).await.unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    for migration in manifest["migrations"].as_array().unwrap().iter().take(1) {
        sqlx::query("INSERT INTO citadel_schema_migrations VALUES($1,$2,$3,now(),now(),NULL)")
            .bind(migration["id"].as_str().unwrap())
            .bind(migration["name"].as_str().unwrap())
            .bind(migration["sha256"].as_str().unwrap())
            .execute(&mut connection)
            .await
            .unwrap();
    }
    let platform = Uuid::now_v7();
    let container = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,'io','io','Local',2,0,1000,0,0,'{}','Online')")
        .bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,state,created,updated,ports,issystem) VALUES($1,$2,'io','image','io','Running',0,0,'{}',false)")
        .bind(container).bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO containerstats(id,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) VALUES($1,$2,100,10,100,20,200,1000,2000)")
        .bind(Uuid::now_v7()).bind(container).execute(&mut connection).await.unwrap();
    let before: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let outcome = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(
        (outcome.applied, outcome.already_applied),
        (super::super::migration_count() - 1, 1)
    );
    let recorded: (f64,f64,Option<f64>,Option<f64>) = sqlx::query_as("SELECT rxbytes,txbytes,blockreadbytes,blockwritebytes FROM containerstats WHERE containerid=$1")
        .bind(container).fetch_one(&mut connection).await.unwrap();
    assert_eq!(recorded, (1000., 2000., None, None));
    let after: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,checksum FROM citadel_schema_migrations WHERE id='0001' ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(before, after);
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

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn platform_io_upgrade_preserves_existing_platform_history_and_applied_checksums() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    sqlx::raw_sql(concat!(
        include_str!("../../generated/0001_initial.sql"),
        "\n",
        include_str!("../../generated/0002_instance_updates.sql")
    ))
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY, name text NOT NULL, checksum text NOT NULL, started_at timestamptz NOT NULL, completed_at timestamptz, failure text)")
        .execute(&mut connection).await.unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    for migration in manifest["migrations"].as_array().unwrap().iter().take(2) {
        sqlx::query("INSERT INTO citadel_schema_migrations VALUES($1,$2,$3,now(),now(),NULL)")
            .bind(migration["id"].as_str().unwrap())
            .bind(migration["name"].as_str().unwrap())
            .bind(migration["sha256"].as_str().unwrap())
            .execute(&mut connection)
            .await
            .unwrap();
    }
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,'io','io','Local',2,0,1000,0,0,'{}','Online')")
        .bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes) VALUES($1,$2,100,10,20,1000,2000)")
        .bind(Uuid::now_v7()).bind(platform).execute(&mut connection).await.unwrap();
    let before: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let outcome = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(
        (outcome.applied, outcome.already_applied),
        (super::super::migration_count() - 2, 2)
    );
    let retained: (f64, f64, Option<serde_json::Value>) =
        sqlx::query_as("SELECT rxbytes,txbytes,containerio FROM platformstats WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(retained, (1000., 2000., None));
    let after: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,checksum FROM citadel_schema_migrations WHERE id IN ('0001','0002') ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(before, after);
    let counters = serde_json::json!([{"containerId":"deleted","rxBytes":1000,"txBytes":2000}]);
    sqlx::query("UPDATE platformstats SET containerio=$2 WHERE platformid=$1")
        .bind(platform)
        .bind(&counters)
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("INSERT INTO platformnodeiostats(platformid,nodeid,created,containerio) VALUES($1,'worker',100,$2)")
        .bind(platform).bind(&counters).execute(&mut connection).await.unwrap();
    assert_eq!(
        MigrationRunner::migrate(&database.url)
            .await
            .unwrap()
            .applied,
        0
    );
    let host: serde_json::Value =
        sqlx::query_scalar("SELECT containerio FROM platformstats WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&mut connection)
            .await
            .unwrap();
    let node: serde_json::Value = sqlx::query_scalar(
        "SELECT containerio FROM platformnodeiostats WHERE platformid=$1 AND nodeid='worker'",
    )
    .bind(platform)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(host, counters);
    assert_eq!(node, counters);
    connection.close().await.unwrap();
    database.close().await;
}
