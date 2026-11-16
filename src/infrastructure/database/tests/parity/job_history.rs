use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn job_history_upgrade_from_seven_preserves_settings_and_applied_checksums() {
    let version = 7;
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
        include_str!("../../generated/0005_health_history_retention.sql"),
        "\n",
        include_str!("../../generated/0006_memory_breakdown.sql"),
        "\n",
        include_str!("../../generated/0007_platform_coarse_history.sql")
    ))
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY,name text NOT NULL,checksum text NOT NULL,started_at timestamptz NOT NULL,completed_at timestamptz,failure text)").execute(&mut connection).await.unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    for m in manifest["migrations"]
        .as_array()
        .unwrap()
        .iter()
        .take(version)
    {
        sqlx::query("INSERT INTO citadel_schema_migrations VALUES($1,$2,$3,now(),now(),NULL)")
            .bind(m["id"].as_str().unwrap())
            .bind(m["name"].as_str().unwrap())
            .bind(m["sha256"].as_str().unwrap())
            .execute(&mut connection)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE instancesetupstates SET updatechecksenabled=false, mfapolicy='RequiredForAdministrators' WHERE id=1")
        .execute(&mut connection).await.unwrap();
    let checksums: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let outcome = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(outcome.already_applied, version);
    assert_eq!(outcome.applied, super::super::migration_count() - version);
    let settings: (bool, String) =
        sqlx::query_as("SELECT updatechecksenabled, mfapolicy FROM instancesetupstates WHERE id=1")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(settings, (false, "RequiredForAdministrators".into()));
    let boot = Uuid::now_v7();
    sqlx::query("INSERT INTO jobdiagnosticsessions(bootid,coreid,version,heartbeat,snapshot,flushsequence) VALUES($1,$1,'test',600000,'{}',1)").bind(boot).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO jobdiagnosticbuckets(bootid,jobid,bucket,data) VALUES($1,'test',300000,'{\"iterations\":12,\"elapsedMs\":42}')").bind(boot).execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO jobdiagnosticevents(bootid,sequence,jobid,observed,kind) VALUES($1,1,'test',300000,'failed')").bind(boot).execute(&mut connection).await.unwrap();
    assert_eq!(
        MigrationRunner::migrate(&database.url)
            .await
            .unwrap()
            .applied,
        0
    );
    let retained: (i64, serde_json::Value) =
        sqlx::query_as("SELECT resolutionms,data FROM jobdiagnosticbuckets")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(
        retained,
        (300000, serde_json::json!({"iterations":12,"elapsedMs":42}))
    );
    let legacy: (String, Option<serde_json::Value>) = sqlx::query_as(
        "SELECT kind,problem FROM jobdiagnosticevents WHERE bootid=$1 AND sequence=1",
    )
    .bind(boot)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(legacy, ("failed".into(), None));
    let context = serde_json::json!({"at":300000,"summary":"A reported problem","target":{"resourceType":"Platform","resourceId":boot,"operationId":null}});
    sqlx::query("UPDATE jobdiagnosticevents SET problem=$2 WHERE bootid=$1 AND sequence=1")
        .bind(boot)
        .bind(&context)
        .execute(&mut connection)
        .await
        .unwrap();
    let saved: serde_json::Value = sqlx::query_scalar(
        "SELECT problem FROM jobdiagnosticevents WHERE bootid=$1 AND sequence=1",
    )
    .bind(boot)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(saved, context);
    let after: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id LIMIT $1")
            .bind(version as i64)
            .fetch_all(&mut connection)
            .await
            .unwrap();
    assert_eq!(checksums, after);
    connection.close().await.unwrap();
    database.close().await;
}
