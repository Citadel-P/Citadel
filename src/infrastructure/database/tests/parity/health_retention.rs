use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn health_retention_upgrade_preserves_history_and_indexes_local_caps() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    sqlx::raw_sql(concat!(
        include_str!("../../generated/0001_initial.sql"),
        "\n",
        include_str!("../../generated/0002_instance_updates.sql"),
        "\n",
        include_str!("../../generated/0003_platform_io.sql"),
        "\n",
        include_str!("../../generated/0004_query_indexes.sql")
    ))
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY,name text NOT NULL,checksum text NOT NULL,started_at timestamptz NOT NULL,completed_at timestamptz,failure text)").execute(&mut connection).await.unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    for migration in manifest["migrations"].as_array().unwrap().iter().take(4) {
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
    sqlx::query("INSERT INTO platforms(id,address,connectortype,name,platformdescriptor,status,cpucount,imagecount,memtotal,networkcount,volumecount) VALUES($1,$1::text,'Local',$1::text,'{\"$type\":\"Docker\"}','Online',1,0,1024,0,0)").bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,health,projectionobservedat) VALUES($1,$2,'history','image','history',1,1,'Running','[]',jsonb_build_object('configuration','Enabled','result','Healthy','observedAt',(floor(extract(epoch FROM now())/60)*60000-60000)::bigint),1)").bind(container).bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO containerhealthtransitions(containerid,platformid,dockercontainerid,observedat,state,origin) SELECT $1,$2,'history',n,'Running','Sample' FROM generate_series(1,10080) n").bind(container).bind(platform).execute(&mut connection).await.unwrap();
    // History deliberately survives removal of its original container.
    sqlx::query("INSERT INTO containerhealthtransitions(containerid,platformid,dockercontainerid,observedat,state,origin) VALUES($1,$2,'removed',1,'Stopped','Lifecycle')").bind(Uuid::now_v7()).bind(platform).execute(&mut connection).await.unwrap();
    let before: Vec<(i64, serde_json::Value)> =
        sqlx::query_as("SELECT id,to_jsonb(h) FROM containerhealthtransitions h ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let outcome = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(outcome.already_applied, 4);
    assert_eq!(outcome.applied, super::super::migration_count() - 4);
    let after: Vec<(i64, serde_json::Value)> = sqlx::query_as(
        "SELECT id,to_jsonb(h)-'retentionordinal' FROM containerhealthtransitions h ORDER BY id",
    )
    .fetch_all(&mut connection)
    .await
    .unwrap();
    assert_eq!(before, after);
    sqlx::query("UPDATE containers SET health=jsonb_set(health,'{observedAt}',to_jsonb((floor(extract(epoch FROM now())/60)*60000)::bigint)) WHERE id=$1").bind(container).execute(&mut connection).await.unwrap();
    let retained: (i64,i64,i64) = sqlx::query_as("SELECT count(*),min(retentionordinal),max(retentionordinal) FROM containerhealthtransitions WHERE containerid=$1 AND origin='Sample'").bind(container).fetch_one(&mut connection).await.unwrap();
    assert_eq!(retained, (10080, 2, 10081));
    // Rollback restores both admission counter and inserted evidence.
    sqlx::raw_sql("BEGIN")
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("UPDATE containers SET state='Stopped' WHERE id=$1")
        .bind(container)
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::raw_sql("ROLLBACK")
        .execute(&mut connection)
        .await
        .unwrap();
    let changes: i64 = sqlx::query_scalar(
        "SELECT changes FROM containerhealthhistorycounters WHERE containerid=$1",
    )
    .bind(container)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(changes, 1);
    for i in 0..210 {
        sqlx::query("UPDATE containers SET state=$2 WHERE id=$1")
            .bind(container)
            .bind(if i % 2 == 0 { "Stopped" } else { "Running" })
            .execute(&mut connection)
            .await
            .unwrap();
    }
    let changes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM containerhealthtransitions WHERE containerid=$1 AND origin<>'Sample'",
    )
    .bind(container)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(changes, 200);
    sqlx::raw_sql("ANALYZE containerhealthtransitions")
        .execute(&mut connection)
        .await
        .unwrap();
    let old: serde_json::Value = sqlx::query_scalar("EXPLAIN(ANALYZE,BUFFERS,FORMAT JSON) SELECT id FROM containerhealthtransitions WHERE containerid=$1 AND origin='Sample' ORDER BY id DESC OFFSET 10080").bind(container).fetch_one(&mut connection).await.unwrap();
    let new: serde_json::Value = sqlx::query_scalar("EXPLAIN(ANALYZE,BUFFERS,FORMAT JSON) SELECT id FROM containerhealthtransitions WHERE containerid=$1 AND origin='Sample' AND retentionordinal<=1").bind(container).fetch_one(&mut connection).await.unwrap();
    let old_hits = old[0]["Plan"]["Shared Hit Blocks"].as_u64().unwrap();
    let new_hits = new[0]["Plan"]["Shared Hit Blocks"].as_u64().unwrap();
    assert!(
        new_hits < old_hits,
        "indexed expiry should not walk retained history: old={old}, new={new}"
    );
    eprintln!("health retention shared hits: offset={old_hits}, indexed={new_hits}");
    // The global ID window also evicts orphan counter state after sequence gaps.
    sqlx::query("SELECT setval(pg_get_serial_sequence('containerhealthtransitions','id'),(SELECT max(id)+100000 FROM containerhealthtransitions),true)")
        .execute(&mut connection).await.unwrap();
    sqlx::query("UPDATE containers SET state='Stopped' WHERE id=$1")
        .bind(container)
        .execute(&mut connection)
        .await
        .unwrap();
    let global: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM containerhealthtransitions),(SELECT count(*) FROM containerhealthhistorycounters)")
        .fetch_one(&mut connection).await.unwrap();
    assert_eq!(global, (1, 1));
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
