use super::*;
use citadel_adapters::persistence::postgres::backups::coverage::changes::PostgresCoverageChanges;
use citadel_backups::coverage::CoverageChangeSource;

#[tokio::test]
#[ignore = "requires an isolated CITADEL_PLATFORM_DATABASE_URL"]
async fn committed_run_refreshes_shared_storage_owners_and_skips_unrelated_workloads() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    let [
        platform,
        actor,
        secret,
        repo,
        policy,
        first,
        second,
        unrelated,
    ] = std::array::from_fn(|_| Uuid::now_v7());
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$1::text,'InternalEncrypted')")
        .bind(secret).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO backuprepositories(id,name,normalizedname,passwordsecretid,spec,status,type,createdbyactorid) VALUES($1,$1::text,$1::text,$2,'{}','Ready','FileSystem',$3)")
        .bind(repo).bind(secret).bind(actor).execute(&pool).await.unwrap();
    sqlx::query(r#"UPDATE platforms SET platformdescriptor='{"$type":"Docker"}' WHERE id=$1"#)
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let mut inventory = snapshot(platform, false);
    inventory.swarm = None;
    inventory.info.swarm = None;
    inventory.containers[0].is_swarm_task = false;
    inventory.containers[0].mounts = Some(json!([{"Type":"volume","Name":"shared"}]));
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&inventory)
        .await
        .unwrap();
    for (id, volume) in [
        (first, "shared:/a"),
        (second, "shared:/b"),
        (unrelated, "other:/a"),
    ] {
        sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,status,spec) VALUES($1,$1::text,$2,$3,'Created',$4)")
            .bind(id).bind(platform).bind(actor).bind(json!({"Volumes":[volume]})).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO backuppolicies(id,name,normalizedname,backuprepositoryid,createdbyactorid,runasactorid,source) VALUES($1,$1::text,$1::text,$2,$3,$3,$4)")
        .bind(policy).bind(repo).bind(actor).bind(json!({"$type":"Deployment","deploymentId":first})).execute(&pool).await.unwrap();
    let mut listener = PostgresCoverageChanges::connect(&pool).await.unwrap();
    sqlx::query("SELECT pg_notify('citadel_backup_coverage',$1)")
        .bind(format!("{platform}:{policy}"))
        .execute(&pool)
        .await
        .unwrap();
    let change = tokio::time::timeout(std::time::Duration::from_secs(5), listener.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(change.platform_id, platform);
    assert_eq!(
        change
            .deployment_ids
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        [first, second].into_iter().collect()
    );
    assert_eq!(change.container_ids.len(), 1);
    // Other inventory cases reuse this database and can read the full catalog.
    for (statement, id) in [
        ("DELETE FROM backuppolicies WHERE id=$1", policy),
        ("DELETE FROM backuprepositories WHERE id=$1", repo),
        ("DELETE FROM secretdefinitions WHERE id=$1", secret),
        ("DELETE FROM deployments WHERE platformid=$1", platform),
        ("DELETE FROM platforms WHERE id=$1", platform),
        ("DELETE FROM tags WHERE createdbyactorid=$1", actor),
        ("DELETE FROM actors WHERE id=$1", actor),
    ] {
        sqlx::query(statement)
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
