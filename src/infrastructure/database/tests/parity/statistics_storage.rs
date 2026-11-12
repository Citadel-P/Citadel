use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn compact_statistics_preserve_samples_uniqueness_and_migration_history() {
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
    sqlx::raw_sql("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status)
        VALUES('00000000-0000-0000-0000-000000000010','storage','storage','Local',2,0,1000,0,0,'{}','Online');
        INSERT INTO deployments(id,createdbyactorid,name,platformid,spec,status)
        VALUES('00000000-0000-0000-0000-000000000020','00000000-0000-0000-0000-000000000001','storage','00000000-0000-0000-0000-000000000010','{}','Stopped');
        INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,state,created,updated,ports,issystem)
        VALUES('00000000-0000-0000-0000-000000000030','00000000-0000-0000-0000-000000000010','storage','image','storage','Running',0,0,'{}',false);
        INSERT INTO containerstats(id,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
        SELECT gen_random_uuid(),'00000000-0000-0000-0000-000000000030',n,1,2,3,4,n*10,n*20,NULL,n*30 FROM generate_series(1,1000) n;
        INSERT INTO workloadstats(id,platformid,deploymentid,servicename,replicakey,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
        SELECT gen_random_uuid(),'00000000-0000-0000-0000-000000000010','00000000-0000-0000-0000-000000000020','','deployment',containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes FROM containerstats;
        INSERT INTO swarmservicestats(id,platformid,dockerserviceid,dockertaskid,servicename,taskkey,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
        SELECT gen_random_uuid(),'00000000-0000-0000-0000-000000000010','service','task','service','slot:1',created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes FROM containerstats;
        INSERT INTO platformnodeiostats(id,platformid,nodeid,created,containerio)
        SELECT gen_random_uuid(),'00000000-0000-0000-0000-000000000010','worker',n,jsonb_build_array(jsonb_build_object('containerId','removed','rxBytes',n*10,'txBytes',n*20)) FROM generate_series(1,1000) n;")
        .execute(&mut connection).await.unwrap();
    sqlx::query("UPDATE swarmservicestats SET stackid='00000000-0000-0000-0000-000000000404'")
        .execute(&mut connection)
        .await
        .unwrap();
    let tables = [
        "containerstats",
        "workloadstats",
        "swarmservicestats",
        "platformnodeiostats",
    ];
    let mut samples = Vec::new();
    for table in tables {
        let rows: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT jsonb_agg(to_jsonb(s)-'id' ORDER BY created) FROM {table} s"
        )))
        .fetch_one(&mut connection)
        .await
        .unwrap();
        samples.push(rows);
    }
    let before_journal: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    // Refuse divergent duplicates rather than silently choosing one history.
    sqlx::query("UPDATE containerstats SET rxbytes=rxbytes+1 WHERE created=1")
        .execute(&mut connection)
        .await
        .unwrap();
    assert!(MigrationRunner::migrate(&database.url).await.is_err());
    let original: bool = sqlx::query_scalar(
        "SELECT to_regclass('public.statisticssamples') IS NULL
        AND to_regclass('public.ix_activityevents_createdat_id') IS NULL
        AND EXISTS(SELECT 1 FROM containerstats WHERE id IS NOT NULL AND created=1 AND rxbytes=11)",
    )
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert!(
        original,
        "failed normalization must roll back schema and preserve original captures"
    );
    sqlx::query("UPDATE containerstats SET rxbytes=rxbytes-1 WHERE created=1")
        .execute(&mut connection)
        .await
        .unwrap();
    let result = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(
        (result.applied, result.already_applied),
        (super::super::migration_count() - 3, 3)
    );
    let after_journal: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,checksum FROM citadel_schema_migrations WHERE id<='0003' ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(before_journal, after_journal);
    // The consolidated migration removes redundant UUID indexes while moving
    // all metrics into the shared sample store.
    let old_indexes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_indexes WHERE schemaname='public'
        AND indexname IN ('pk_containerstats','pk_swarmservicestats','workloadstats_pkey')",
    )
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(old_indexes, 0);
    let tuned: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_class WHERE relnamespace='public'::regnamespace
        AND relname IN ('statisticssamples_default','statisticsrollups_default','platformnodeiostats','platformstats','containerhealthtransitions')
        AND reloptions @> ARRAY['autovacuum_vacuum_scale_factor=0.05','autovacuum_vacuum_threshold=1000']")
        .fetch_one(&mut connection).await.unwrap();
    assert_eq!(tuned, 5);
    let duplicate_node =
        sqlx::query("INSERT INTO platformnodeiostats SELECT * FROM platformnodeiostats LIMIT 1")
            .execute(&mut connection)
            .await
            .unwrap_err();
    assert_eq!(
        duplicate_node
            .as_database_error()
            .unwrap()
            .code()
            .as_deref(),
        Some("23505")
    );
    for (table, before) in tables.into_iter().zip(samples) {
        let after:serde_json::Value=sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT jsonb_agg(to_jsonb(s)-ARRAY['samplecount','readcount','writecount','lastcreated','firstcounters','lastcounters','counterdeltas','counterdurations','peaks'] ORDER BY created) FROM {table} s"
        ))).fetch_one(&mut connection).await.unwrap();
        assert_eq!(
            before, after,
            "{table}: shared measurements preserve every value and owner"
        );
    }
    let physical: i64 = sqlx::query_scalar("SELECT count(*) FROM statisticssamples")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(
        physical, 2000,
        "container/workload samples are stored once; unrelated task stays independent"
    );
    let duplicate =
        sqlx::query("INSERT INTO statisticssamples SELECT * FROM statisticssamples LIMIT 1")
            .execute(&mut connection)
            .await
            .unwrap_err();
    assert_eq!(
        duplicate.as_database_error().unwrap().code().as_deref(),
        Some("23505")
    );
    sqlx::query("DELETE FROM containers WHERE id='00000000-0000-0000-0000-000000000030'")
        .execute(&mut connection)
        .await
        .unwrap();
    let workload_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM workloadstats")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(
        workload_rows, 1000,
        "container deletion must retain workload history"
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

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn platform_history_upgrade_preserves_raw_samples_and_pending_alert_revisions() {
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
        INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status)
        VALUES('00000000-0000-0000-0000-000000000010','platform-history','platform-history','Local',2,0,1000,0,0,'{}','Online');
        INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,alertpending,containerio)
        VALUES(gen_random_uuid(),'00000000-0000-0000-0000-000000000010',10,1,2,3,4,true,'[{\"containerId\":\"removed\",\"rxBytes\":3,\"txBytes\":4}]');
        INSERT INTO platformnodeiostats(id,platformid,nodeid,created,containerio)
        VALUES(gen_random_uuid(),'00000000-0000-0000-0000-000000000010','gone-node',10,'[]');")
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
    let before: (serde_json::Value, String) =
        sqlx::query_as("SELECT to_jsonb(s),s.xmin::text FROM platformstats s")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    MigrationRunner::migrate(&database.url).await.unwrap();
    let after: (serde_json::Value, String) =
        sqlx::query_as("SELECT to_jsonb(s),s.xmin::text FROM platformstats s")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(
        before, after,
        "migration cannot rewrite a pending alert claim"
    );
    let sources: Vec<(String, i64)> =
        sqlx::query_as("SELECT nodeid,compacteduntil FROM platformhistorysources ORDER BY nodeid")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    assert_eq!(sources, vec![(String::new(), 0), ("gone-node".into(), 0)]);
    let node: serde_json::Value =
        sqlx::query_scalar("SELECT containerio FROM platformnodeiostats WHERE nodeid='gone-node'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(node, serde_json::json!([]));
    let journal: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,checksum FROM citadel_schema_migrations WHERE id<='0003' ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    for ((id, checksum), entry) in journal
        .iter()
        .zip(manifest["migrations"].as_array().unwrap())
    {
        assert_eq!(id, entry["id"].as_str().unwrap());
        assert_eq!(checksum, entry["sha256"].as_str().unwrap());
    }
    connection.close().await.unwrap();
    database.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn physical_statistics_identity_upgrade_preserves_applied_history_and_alert_claims() {
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
        INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status)
        VALUES('00000000-0000-0000-0000-000000000010','platform-history','platform-history','Local',2,0,1000,0,0,'{}','Online');
        INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,alertpending,containerio)
        VALUES(gen_random_uuid(),'00000000-0000-0000-0000-000000000010',10,1,2,3,4,true,'[{\"containerId\":\"removed\",\"rxBytes\":3,\"txBytes\":4}]');
        INSERT INTO platformnodeiostats(id,platformid,nodeid,created,containerio)
        VALUES(gen_random_uuid(),'00000000-0000-0000-0000-000000000010','gone-node',10,'[]');")
        .execute(&mut connection).await.unwrap();
    sqlx::raw_sql("UPDATE platforms SET platformdescriptor='{\"$type\":\"DockerSwarm\",\"nodeID\":\"n\"}' WHERE id='00000000-0000-0000-0000-000000000010';
        INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,state,created,updated,ports,issystem,dockernodeid)
        VALUES('00000000-0000-0000-0000-000000000011','00000000-0000-0000-0000-000000000010','same-physical','image','node','Running',0,0,'{}',false,'n');
        INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockercontainerid,dockernodeid,dockerserviceid,image,name,nodehostname,observedat,ports,servicename,slot,state,versionindex)
        VALUES('00000000-0000-0000-0000-000000000010','task','running','same-physical','n','service','image','task','n',now(),'[]','service',1,'running',1);")
        .execute(&mut connection).await.unwrap();
    sqlx::raw_sql("INSERT INTO swarmservicestats(id,platformid,dockerserviceid,dockertaskid,servicename,taskkey,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
        VALUES(gen_random_uuid(),'00000000-0000-0000-0000-000000000010','service','task','service','slot:1',10,1,2,3,4,5,6);")
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
    let before: (serde_json::Value, String) =
        sqlx::query_as("SELECT to_jsonb(s),s.xmin::text FROM platformstats s")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    MigrationRunner::migrate(&database.url).await.unwrap();
    let after: (serde_json::Value, String) =
        sqlx::query_as("SELECT to_jsonb(s),s.xmin::text FROM platformstats s")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(
        before, after,
        "migration cannot rewrite a pending alert claim"
    );
    let sources: Vec<(String, i64)> =
        sqlx::query_as("SELECT nodeid,compacteduntil FROM platformhistorysources ORDER BY nodeid")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    assert_eq!(sources, vec![(String::new(), 0), ("gone-node".into(), 0)]);
    let node: serde_json::Value =
        sqlx::query_scalar("SELECT containerio FROM platformnodeiostats WHERE nodeid='gone-node'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(node, serde_json::json!([]));
    let journal: Vec<(String, String)> = sqlx::query_as(
        "SELECT id,checksum FROM citadel_schema_migrations WHERE id<='0003' ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    for ((id, checksum), entry) in journal
        .iter()
        .zip(manifest["migrations"].as_array().unwrap())
    {
        assert_eq!(id, entry["id"].as_str().unwrap());
        assert_eq!(checksum, entry["sha256"].as_str().unwrap());
    }
    // A later host observation must reuse the source backfilled from the node.
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,state,created,updated,ports,issystem,dockernodeid)
        VALUES('00000000-0000-0000-0000-000000000012','00000000-0000-0000-0000-000000000010','same-physical','image','host','Running',0,0,'{}',false,NULL)")
        .execute(&mut connection).await.unwrap();
    let source: i64 = sqlx::query_scalar("SELECT id FROM statisticssources WHERE dockertaskid='task' AND containerid='00000000-0000-0000-0000-000000000011'")
        .fetch_one(&mut connection).await.unwrap();
    let attached:i64=sqlx::query_scalar("SELECT statistics_source('00000000-0000-0000-0000-000000000010','00000000-0000-0000-0000-000000000012','task')").fetch_one(&mut connection).await.unwrap();
    assert_eq!(
        source, attached,
        "the upgrade reconciles verified live host/node projections without replacing the source"
    );
    connection.close().await.unwrap();
    database.close().await;
}
