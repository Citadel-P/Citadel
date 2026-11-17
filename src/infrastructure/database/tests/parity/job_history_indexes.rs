use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn job_history_index_upgrade_preserves_checkpoints_and_scoped_reads() {
    let database = Database::create().await;
    let mut connection = PgConnection::connect(&database.url).await.unwrap();
    for sql in [
        include_str!("../../generated/0001_initial.sql"),
        include_str!("../../generated/0002_instance_updates.sql"),
        include_str!("../../generated/0003_platform_io.sql"),
        include_str!("../../generated/0004_query_indexes.sql"),
        include_str!("../../generated/0005_health_history_retention.sql"),
        include_str!("../../generated/0006_memory_breakdown.sql"),
        include_str!("../../generated/0007_platform_coarse_history.sql"),
        include_str!("../../generated/0008_job_diagnostics.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&mut connection).await.unwrap();
    }
    sqlx::raw_sql("CREATE TABLE citadel_schema_migrations (id text PRIMARY KEY,name text NOT NULL,checksum text NOT NULL,started_at timestamptz NOT NULL,completed_at timestamptz,failure text)")
        .execute(&mut connection).await.unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(citadel_database::MIGRATION_MANIFEST_JSON).unwrap();
    for migration in manifest["migrations"].as_array().unwrap().iter().take(8) {
        sqlx::query("INSERT INTO citadel_schema_migrations VALUES($1,$2,$3,now(),now(),NULL)")
            .bind(migration["id"].as_str().unwrap())
            .bind(migration["name"].as_str().unwrap())
            .bind(migration["sha256"].as_str().unwrap())
            .execute(&mut connection)
            .await
            .unwrap();
    }
    let checksums: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    let boots: Vec<Uuid> = (0..4).map(|_| Uuid::now_v7()).collect();
    for boot in &boots {
        sqlx::query("INSERT INTO jobdiagnosticsessions(bootid,coreid,version,heartbeat,snapshot,flushsequence) VALUES($1,$1,'fixture',604800000,'{}',1)")
            .bind(boot).execute(&mut connection).await.unwrap();
        // Six days of hourly summaries and one day of five-minute windows.
        sqlx::query("INSERT INTO jobdiagnosticbuckets(bootid,jobid,bucket,resolutionms,data) SELECT $1,'job-'||j,t,resolution,jsonb_build_object('iterations',30,'elapsedMs',42) FROM generate_series(1,35) j CROSS JOIN (SELECT generate_series(0::bigint,514800000,3600000) t,3600000 resolution UNION ALL SELECT generate_series(518400000::bigint,604500000,300000),300000) times ORDER BY t,j")
            .bind(boot).execute(&mut connection).await.unwrap();
    }
    sqlx::raw_sql("ANALYZE jobdiagnosticbuckets; PREPARE history_read(text,bigint,uuid[],bigint) AS SELECT data || jsonb_build_object('resolutionMs',resolutionms) FROM jobdiagnosticbuckets WHERE jobid=$1 AND bucket >= $2 AND bucket+resolutionms > $4 AND bootid=ANY($3) ORDER BY bucket DESC LIMIT 2017;")
        .execute(&mut connection).await.unwrap();
    let read = "SELECT bootid,jobid,bucket,resolutionms,data FROM jobdiagnosticbuckets WHERE jobid='job-10' AND bootid=ANY($1) ORDER BY bucket,bootid";
    let before: Vec<(Uuid, String, i64, i64, serde_json::Value)> = sqlx::query_as(read)
        .bind(&boots)
        .fetch_all(&mut connection)
        .await
        .unwrap();
    let baseline = read_plans(&mut connection, &boots).await;
    let outcome = MigrationRunner::migrate(&database.url).await.unwrap();
    assert_eq!(outcome.already_applied, 8);
    assert_eq!(outcome.applied, super::super::migration_count() - 8);
    let removed: bool =
        sqlx::query_scalar("SELECT to_regclass('public.jobdiagnosticbuckets_job') IS NULL")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert!(removed);
    let after: Vec<(Uuid, String, i64, i64, serde_json::Value)> = sqlx::query_as(read)
        .bind(&boots)
        .fetch_all(&mut connection)
        .await
        .unwrap();
    assert_eq!(before, after);
    for (before, after) in baseline
        .iter()
        .zip(read_plans(&mut connection, &boots).await)
    {
        assert_eq!(before["Actual Rows"], after["Actual Rows"]);
        let buffers = |v: &serde_json::Value| {
            v["Shared Hit Blocks"].as_u64().unwrap() + v["Shared Read Blocks"].as_u64().unwrap()
        };
        assert!(buffers(&after) <= buffers(before) * 2 + 16, "{after}");
        assert_eq!(after["Temp Written Blocks"], 0);
    }
    let retained: Vec<(String, String)> =
        sqlx::query_as("SELECT id,checksum FROM citadel_schema_migrations ORDER BY id LIMIT 8")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    assert_eq!(checksums, retained);
    // The conflict key and mutable-window checkpoint semantics are unchanged.
    let changed: i64 = sqlx::query_scalar("INSERT INTO jobdiagnosticbuckets(bootid,jobid,bucket,data) VALUES($1,'job-10',604500000,'{\"iterations\":60}') ON CONFLICT(bootid,jobid,bucket) DO UPDATE SET data=EXCLUDED.data RETURNING (data->>'iterations')::bigint")
        .bind(boots[0]).fetch_one(&mut connection).await.unwrap();
    assert_eq!(changed, 60);
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

async fn read_plans(connection: &mut PgConnection, boots: &[Uuid]) -> Vec<serde_json::Value> {
    let mut plans = Vec::new();
    for mode in [
        "SET plan_cache_mode=force_custom_plan",
        "SET plan_cache_mode=force_generic_plan",
    ] {
        sqlx::raw_sql(mode).execute(&mut *connection).await.unwrap();
        for count in [1, boots.len()] {
            let ids = boots[..count]
                .iter()
                .map(|id| format!("'{id}'::uuid"))
                .collect::<Vec<_>>()
                .join(",");
            for hours in [0.25, 6., 24., 168.] {
                let cutoff = 604_800_000 - (hours * 3_600_000.) as i64;
                let sql = format!(
                    "EXPLAIN(ANALYZE,BUFFERS,FORMAT JSON,TIMING OFF) EXECUTE history_read('job-10',{},ARRAY[{ids}],{cutoff})",
                    cutoff / 3_600_000 * 3_600_000
                );
                let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
                    .fetch_one(&mut *connection)
                    .await
                    .unwrap();
                plans.push(plan[0]["Plan"].clone());
            }
        }
    }
    plans
}
