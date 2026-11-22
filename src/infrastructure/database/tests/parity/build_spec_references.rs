use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn build_reference_backfill_and_concurrent_edits_match_recursive_retention() {
    let db = Database::create().await;
    let mut writer = PgConnection::connect(&db.url).await.unwrap();
    let mut observer = PgConnection::connect(&db.url).await.unwrap();
    // Seed historical specs before upgrading so the backfill and later trigger
    // updates are both checked against the original recursive lookup.
    let manifest: serde_json::Value =
        serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY,name text NOT NULL,checksum text NOT NULL,started_at timestamptz NOT NULL,completed_at timestamptz,failure text)")
        .execute(&mut writer).await.unwrap();
    let generated = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("generated");
    for migration in manifest["migrations"].as_array().unwrap().iter().take(9) {
        let sql =
            std::fs::read_to_string(generated.join(migration["file"].as_str().unwrap())).unwrap();
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
            .execute(&mut writer)
            .await
            .unwrap();
        sqlx::query("INSERT INTO citadel_schema_migrations VALUES ($1,$2,$3,now(),now(),NULL)")
            .bind(migration["id"].as_str().unwrap())
            .bind(migration["name"].as_str().unwrap())
            .bind(migration["sha256"].as_str().unwrap())
            .execute(&mut writer)
            .await
            .unwrap();
    }
    sqlx::raw_sql("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
            VALUES(md5('platform')::uuid,'fixture','Local',1,0,1000,'fixture',0,'{}','Online',0);
        INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate)
            VALUES(md5('stack')::uuid,'fixture','00000000-0000-0000-0000-000000000001','Inline','{}','{}');
        INSERT INTO deployments(id,name,createdbyactorid,platformid,status,spec)
            VALUES(md5('owner')::uuid,'fixture','00000000-0000-0000-0000-000000000001',md5('platform')::uuid,'Running',
                jsonb_build_array(md5('1')::uuid::text,upper(md5('2')::uuid::text)));
        INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,status,version,spec)
            VALUES(md5('historical')::uuid,md5('stack')::uuid,md5('platform')::uuid,'00000000-0000-0000-0000-000000000001','Active','1',
                jsonb_build_object('nested',jsonb_build_array(md5('3')::uuid::text)));
        CREATE VIEW candidates AS SELECT md5(i::text)::uuid AS id FROM generate_series(1,20)i;
        CREATE VIEW original AS SELECT id FROM candidates r WHERE EXISTS (
            SELECT 1 FROM (SELECT spec FROM deployments UNION ALL SELECT spec FROM stackreleases) owners
            WHERE jsonb_path_exists(spec,'$.** ? (@ == $id)',jsonb_build_object('id',r.id::text)))")
        .execute(&mut writer).await.unwrap();
    let outcome = MigrationRunner::migrate(&db.url).await.unwrap();
    assert_eq!(outcome.already_applied, 9);
    assert_eq!(outcome.applied, super::super::migration_count() - 9);
    async fn check(conn: &mut PgConnection, keys: &[i32]) {
        let (scanned,indexed,expected):(Vec<Uuid>,Vec<Uuid>,Vec<Uuid>)=sqlx::query_as(
            "SELECT ARRAY(SELECT id FROM original ORDER BY id),
                ARRAY(SELECT id FROM candidates r WHERE EXISTS(SELECT 1 FROM buildspecassignments ref WHERE ref.referenceid=r.id) ORDER BY id),
                ARRAY(SELECT md5(n::text)::uuid FROM unnest($1::int[]) n ORDER BY 1)")
            .bind(keys).fetch_one(conn).await.unwrap();
        assert_eq!(scanned, expected);
        assert_eq!(indexed, scanned);
    }
    check(&mut observer, &[1, 3]).await;
    for commit in [false, true] {
        let mut tx = writer.begin().await.unwrap();
        sqlx::raw_sql("UPDATE deployments SET spec=jsonb_build_object('nested',jsonb_build_array(md5('7')::uuid::text,md5('7')::uuid::text));
            DELETE FROM stackreleases").execute(&mut *tx).await.unwrap();
        check(&mut tx, &[7]).await;
        check(&mut observer, &[1, 3]).await;
        if commit {
            tx.commit().await.unwrap();
        } else {
            tx.rollback().await.unwrap();
        }
    }
    check(&mut observer, &[7]).await;
    drop(observer);
    drop(writer);
    db.close().await;
}
