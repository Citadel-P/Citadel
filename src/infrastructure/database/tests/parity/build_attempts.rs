use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn baseline_restart_preserves_build_runs_ownership_and_checksum() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    let installed = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(installed.applied, 1);
    assert_eq!(installed.already_applied, 0);
    let manifest: serde_json::Value =
        serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    let checksum = manifest["migrations"][0]["sha256"].as_str().unwrap();
    let platform = Uuid::now_v7();
    let repository = Uuid::now_v7();
    let backup_owner = Uuid::now_v7();
    sqlx::query("INSERT INTO backupsourceleases(sourcekey,ownerrunid,operationtype,createdat,expiresat) VALUES('held-source',$1,'Backup',now()-INTERVAL '2 hours',now()-INTERVAL '1 hour')")
        .bind(backup_owner).execute(&mut connection).await.unwrap();
    let held_before: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(l) FROM backupsourceleases l WHERE sourcekey='held-source'",
    )
    .fetch_one(&mut connection)
    .await
    .unwrap();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'fixture','Local',1,0,1000,'attempts',0,'{}','Online',0)")
        .bind(platform).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url) VALUES($1,$2,'main','attempts','Healthy','Manual','https://example.invalid/repo')")
        .bind(repository).bind(Uuid::from_u128(1)).execute(&mut connection).await.unwrap();
    let mut runs = Vec::new();
    for status in ["Queued", "Preparing", "Running", "Failed"] {
        let project = Uuid::now_v7();
        let run = Uuid::now_v7();
        sqlx::query("INSERT INTO buildprojects(id,branch,createdbyactorid,gitrepositoryid,imagerepository,name,normalizedname,platformid,currentrunid) VALUES($1,'main',$2,$3,'fixture',$4,upper($4),$5,$6)")
            .bind(project).bind(Uuid::from_u128(1)).bind(repository).bind(status).bind(platform).bind(run).execute(&mut connection).await.unwrap();
        let attempt = matches!(status, "Preparing" | "Running").then(Uuid::now_v7);
        sqlx::query("INSERT INTO buildruns(id,branch,buildprojectid,contextpath,dockerfilepath,gitrepositoryid,gitrepositorynamesnapshot,imagerepository,platformsnapshot,projectnamesnapshot,registrysnapshot,status,timeoutseconds,trigger,triggeredbyactorid,resolvedcommitsha,attemptid) VALUES($1,'main',$2,'.','Dockerfile',$3,'attempts','fixture','{}','fixture','{}',$4,60,'Manual',$5,'pinned-input',$6)")
            .bind(run).bind(project).bind(repository).bind(status).bind(Uuid::from_u128(1)).bind(attempt).execute(&mut connection).await.unwrap();
        let before: serde_json::Value =
            sqlx::query_scalar("SELECT to_jsonb(r) FROM buildruns r WHERE id=$1")
                .bind(run)
                .fetch_one(&mut connection)
                .await
                .unwrap();
        runs.push((run, status, attempt, before));
    }
    let outcome = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(outcome.already_applied, 1);
    assert_eq!(outcome.applied, 0);
    let (held_after, attempt): (serde_json::Value, Option<Uuid>) = sqlx::query_as(
        "SELECT to_jsonb(l),attemptid FROM backupsourceleases l WHERE sourcekey='held-source'",
    )
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(
        held_after, held_before,
        "restart preserves even expired storage holds"
    );
    assert!(
        attempt.is_none(),
        "restart cannot invent cleanup or live ownership proof"
    );
    for (run, status, expected_attempt, before) in runs {
        let (after, attempt): (serde_json::Value, Option<Uuid>) =
            sqlx::query_as("SELECT to_jsonb(r),attemptid FROM buildruns r WHERE id=$1")
                .bind(run)
                .fetch_one(&mut connection)
                .await
                .unwrap();
        assert_eq!(
            after, before,
            "restart must not replay, requeue or alter {status} history"
        );
        assert_eq!(attempt, expected_attempt);
    }
    let after: String =
        sqlx::query_scalar("SELECT checksum FROM citadel_schema_migrations WHERE id='0001'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(after, checksum);
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
