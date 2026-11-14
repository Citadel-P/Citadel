use super::*;
use citadel_adapters::persistence::postgres::{
    builds::PostgresBuildRepository, maintenance::PostgresMaintenanceStore,
};
use citadel_builds::BuildRepository;
use citadel_runtime::maintenance::MaintenanceStore;

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn idle_build_claim_uses_active_index_and_skips_recovery_transaction() {
    let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
    // A single-session shadow table deliberately lacks recovery's columns. A
    // negative claim must not issue any of those statements, even on large history.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TEMP TABLE buildruns(projectid uuid,status text); INSERT INTO buildruns SELECT gen_random_uuid(),'Succeeded' FROM generate_series(1,100000); CREATE INDEX ON buildruns(projectid) WHERE status IN ('Queued','Preparing','Running'); ANALYZE buildruns;")
        .execute(&pool).await.unwrap();
    let store = PostgresBuildRepository::new(pool.clone());
    for _ in 0..10 {
        assert!(
            store
                .claim_next(chrono::Utc::now())
                .await
                .unwrap()
                .is_none()
        );
    }
    let plan: serde_json::Value = sqlx::query_scalar("EXPLAIN(ANALYZE,BUFFERS,TIMING OFF,FORMAT JSON) SELECT EXISTS(SELECT 1 FROM buildruns WHERE status IN ('Queued','Preparing','Running'))")
        .fetch_one(&pool).await.unwrap();
    assert!(
        plan[0]["Plan"]["Local Hit Blocks"]
            .as_i64()
            .unwrap_or_default()
            < 10,
        "{plan}"
    );
    // Each active status must enter the unchanged authoritative path. The shadow
    // table then produces a storage error instead of falsely reporting no work.
    for status in ["Queued", "Preparing", "Running"] {
        sqlx::query("INSERT INTO buildruns VALUES(gen_random_uuid(),$1)")
            .bind(status)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            store.claim_next(chrono::Utc::now()).await.is_err(),
            "{status} must attempt recovery"
        );
        sqlx::query("DELETE FROM buildruns WHERE status=$1")
            .bind(status)
            .execute(&pool)
            .await
            .unwrap();
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn retention_prepares_once_skips_exhausted_families_and_restarts_after_cancellation() {
    let (pool, database) = storage_fixture().await;
    sqlx::raw_sql("CREATE TABLE maintenance_calls(family text);
        CREATE FUNCTION record_maintenance() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN INSERT INTO maintenance_calls VALUES(TG_TABLE_NAME); RETURN NULL; END $$;
        ALTER FUNCTION statistics_partition(text,bigint) RENAME TO original_statistics_partition;
        CREATE FUNCTION statistics_partition(parent text,day bigint) RETURNS boolean LANGUAGE plpgsql AS $$ BEGIN INSERT INTO maintenance_calls VALUES('partition'); RETURN original_statistics_partition(parent,day); END $$;")
        .execute(&pool).await.unwrap();
    for table in [
        "activityevents",
        "containerhealthtransitions",
        "platformnodeiostats",
        "platformstats",
        "scriptruns",
        "containerhealthhistorycounters",
        "statisticsowners",
        "statisticssources",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE TRIGGER counted_delete AFTER DELETE ON {table} FOR EACH STATEMENT EXECUTE FUNCTION record_maintenance()"))).execute(&pool).await.unwrap();
    }
    async fn seed(pool: &PgPool, count: i32) {
        sqlx::query("INSERT INTO activityevents(id,createdat,createdbyactorid,eventtype,info,resourcename,resourcetype,status)
            SELECT gen_random_uuid(),now()-interval '100 days','00000000-0000-0000-0000-000000000001','Test','{}','test','Test','Succeeded' FROM generate_series(1,$1)")
            .bind(count).execute(pool).await.unwrap();
    }
    let store = PostgresMaintenanceStore::new(pool.clone());
    seed(&pool, 10001).await;
    assert!(store.cleanup(None).await.unwrap() >= 10001);
    let counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT family,count(*) FROM maintenance_calls GROUP BY family ORDER BY family",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    for (family, count) in counts {
        let expected = match family.as_str() {
            "partition" => 12,
            "activityevents" => 4,
            _ => 1,
        };
        assert_eq!(
            count, expected,
            "{family} must not scale with Activity backlog"
        );
    }
    seed(&pool, 10001).await;
    let mut held = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(680224102)")
        .execute(&mut *held)
        .await
        .unwrap();
    // Cancellation after the first Activity batch, while counter admission waits.
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(500), store.cleanup(None))
            .await
            .is_err()
    );
    held.rollback().await.unwrap();
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 5001);
    assert!(store.cleanup(None).await.unwrap() >= 5001);
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
    // Partition preparation retains its position after independent row families.
    // A DDL lock timeout must not starve Activity/health/run retention.
    let day = chrono::Utc::now().timestamp() / 86400 + 2;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP TABLE statisticssamples_d{day}"
    )))
    .execute(&pool)
    .await
    .unwrap();
    let mut held = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE ONLY statisticssamples IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *held)
        .await
        .unwrap();
    seed(&pool, 1).await;
    assert!(store.cleanup(None).await.is_err());
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
    held.rollback().await.unwrap();
    store.cleanup(None).await.unwrap();
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn physical_discovery_bounds_empty_sources_and_reaches_work_beyond_first_page() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    sqlx::query("INSERT INTO statisticssources(platformid,containerid) SELECT $1,gen_random_uuid() FROM generate_series(1,2000)")
        .bind(scope.platform_id).execute(&pool).await.unwrap();
    // Work is followed by many empty pages as well as preceded by them.
    let source: i64 = sqlx::query_scalar("SELECT min(id)+1000 FROM statisticssources")
        .fetch_one(&pool)
        .await
        .unwrap();
    let start = (chrono::Utc::now().timestamp() / 86400 - 3) * 86400;
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
        SELECT $1,$2+n,1,100,5,200,n,n FROM generate_series(1,80000)n")
        .bind(source).bind(start).execute(&pool).await.unwrap();
    sqlx::raw_sql("ANALYZE statisticssources; ANALYZE statisticssamples")
        .execute(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(concat!(
        "PREPARE discovery(bigint,bigint,bigint) AS ",
        include_str!(
            "../../src/persistence/postgres/platforms/statistics/compaction_candidates.sql"
        )
    ))
    .execute(&mut *tx)
    .await
    .unwrap();
    for mode in ["force_custom_plan", "force_generic_plan"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SET LOCAL plan_cache_mode={mode}"
        )))
        .execute(&mut *tx)
        .await
        .unwrap();
        for cursor in [0, source - 64] {
            let execute = format!(
                "EXECUTE discovery({},{},{cursor})",
                start + 2 * 86400,
                start - 4 * 86400
            );
            let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "EXPLAIN(ANALYZE,BUFFERS,WAL,TIMING OFF,FORMAT JSON) {execute}"
            )))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
            let blocks = plan[0]["Plan"]["Shared Hit Blocks"].as_i64().unwrap()
                + plan[0]["Plan"]["Shared Read Blocks"].as_i64().unwrap();
            assert!(blocks < 2000, "{mode} cursor={cursor}: {plan}");
            assert!(plan[0]["Plan"]["Actual Rows"].as_f64().unwrap() <= 64.);
            eprintln!("physical discovery {mode} cursor={cursor}: {blocks} buffers");
        }
    }
    tx.rollback().await.unwrap();
    let store = PostgresMaintenanceStore::new(pool.clone());
    let mut progress = 0;
    let mut completed = false;
    // Calls span multiple 128-pass worker budgets. A productive scan must not
    // report idle on its trailing page, even after that batch boundary.
    for page in 0..1000 {
        let result = store.compact().await.unwrap();
        progress += result.processed;
        if page == 31 {
            assert!(result.scan_pending, "one hour does not exhaust this source");
        }
        if !result.scan_pending {
            completed = true;
            break;
        }
    }
    assert!(completed, "an exhausted scan must not wrap forever");
    assert!(
        progress >= 79_000,
        "all complete hours must be drained: {progress}"
    );
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM statisticssamples WHERE sourceid=$1")
            .bind(source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        remaining < 60,
        "only the newest minute stays raw: {remaining}"
    );
    let first: i64 = sqlx::query_scalar("SELECT min(id) FROM statisticssources")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
        VALUES($1,$2,1,100,5,200,0,0),($1,$2+120,1,100,5,200,120,120)").bind(first).bind(start).execute(&pool).await.unwrap();
    assert!(
        store.compact().await.unwrap().processed > 0,
        "next scan must revisit newly eligible earlier sources"
    );
    let watermark: i64 =
        sqlx::query_scalar("SELECT compacteduntil FROM statisticssources WHERE id=$1")
            .bind(first)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(watermark > start);
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn retention_analyzes_only_changed_parents_without_reanalyzing_children() {
    let (fixture, database) = storage_fixture().await;
    // One session makes explicit statistics flushing deterministic; no background
    // autovacuum timing or pg_stat_statements extension is needed by this test.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*fixture.connect_options()).clone())
        .await
        .unwrap();
    fixture.close().await;
    let store = PostgresMaintenanceStore::new(pool.clone());
    store.cleanup(None).await.unwrap();
    let scope = platform(&pool).await;
    let source: i64 = sqlx::query_scalar("INSERT INTO statisticssources(platformid,containerid) VALUES($1,gen_random_uuid()) RETURNING id")
        .bind(scope.platform_id).fetch_one(&pool).await.unwrap();
    let day = chrono::Utc::now().timestamp() / 86400;
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
        SELECT $1,$2+n,1,100,5,200,n,n FROM generate_series(1,100)n")
        .bind(source).bind(day * 86400).execute(&pool).await.unwrap();
    async fn counts(pool: &PgPool) -> Vec<(String, i64)> {
        sqlx::raw_sql("SELECT pg_stat_force_next_flush(); SELECT pg_stat_clear_snapshot();")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query_as("SELECT relname::text,analyze_count FROM pg_stat_user_tables WHERE relname ~ '^(statisticssamples|statisticsrollups|platformgaugerollups|platformiorollups)($|_)' ORDER BY relname")
            .fetch_all(pool).await.unwrap()
    }
    let before = counts(&pool).await;
    store.cleanup(None).await.unwrap();
    assert_eq!(
        counts(&pool).await,
        before,
        "idle retention must not analyze"
    );
    // Creating one missing future child must refresh only its parent, while
    // retaining inherited column statistics over the populated current child.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP TABLE statisticssamples_d{}",
        day + 2
    )))
    .execute(&pool)
    .await
    .unwrap();
    store.cleanup(None).await.unwrap();
    let after = counts(&pool).await;
    for (table, count) in &after {
        let old = before.iter().find(|row| row.0 == *table).unwrap().1;
        // Recreating this child resets its migration-time analysis counter.
        // It must stay at zero, while all surviving children retain their count.
        let expected = if *table == format!("statisticssamples_d{}", day + 2) {
            0
        } else {
            old + i64::from(table == "statisticssamples")
        };
        assert_eq!(*count, expected, "{table}");
    }
    let inherited: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_statistic WHERE starelid='statisticssamples'::regclass AND stainherit)")
        .fetch_one(&pool).await.unwrap();
    assert!(
        inherited,
        "ONLY must still collect inherited parent statistics"
    );
    // Expiring another family's partition must not reanalyze the raw family.
    sqlx::query("SELECT statistics_partition('statisticsrollups',$1)")
        .bind(day - 9)
        .execute(&pool)
        .await
        .unwrap();
    store.cleanup(None).await.unwrap();
    let expired = counts(&pool).await;
    for (table, count) in &expired {
        let old = after.iter().find(|row| row.0 == *table).unwrap().1;
        assert_eq!(
            *count,
            old + i64::from(table == "statisticsrollups"),
            "{table}"
        );
    }
    // Empty, compacted raw-day removal uses the same parent-scoped path.
    sqlx::query("SELECT statistics_partition('statisticssamples',$1)")
        .bind(day - 3)
        .execute(&pool)
        .await
        .unwrap();
    store.cleanup(None).await.unwrap();
    let compacted = counts(&pool).await;
    for (table, count) in &compacted {
        let old = expired.iter().find(|row| row.0 == *table).unwrap().1;
        assert_eq!(
            *count,
            old + i64::from(table == "statisticssamples"),
            "{table}"
        );
    }
    store.cleanup(None).await.unwrap();
    assert_eq!(counts(&pool).await, compacted);
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn partition_expiry_releases_raw_locks_before_waiting_on_rollup_compaction() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let source: i64 = sqlx::query_scalar("INSERT INTO statisticssources(platformid,containerid) VALUES($1,gen_random_uuid()) RETURNING id")
        .bind(scope.platform_id).fetch_one(&pool).await.unwrap();
    let day = chrono::Utc::now().timestamp() / 86400;
    for parent in ["statisticssamples", "statisticsrollups"] {
        sqlx::query("SELECT statistics_partition($1,$2)")
            .bind(parent)
            .bind(day - 9)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
        SELECT $1,$2+n,1,100,5,200,n,n FROM generate_series(0,7200)n UNION ALL SELECT $1,$3,1,100,5,200,0,0")
        .bind(source).bind((day - 3) * 86400).bind((day - 9) * 86400).execute(&pool).await.unwrap();
    // Pause actual retention immediately after it acquires the expired rollup
    // partition's locks. The advisory gate controls ordering, not SQL timing.
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE FUNCTION pause_rollup_expiry() RETURNS event_trigger LANGUAGE plpgsql AS $$
        BEGIN IF EXISTS(SELECT 1 FROM pg_event_trigger_dropped_objects() WHERE object_name='statisticsrollups_d{}') THEN
            PERFORM set_config('lock_timeout','5s',true);
            PERFORM pg_advisory_xact_lock(760114);
            PERFORM set_config('lock_timeout','500ms',true);
        END IF; END $$;
        CREATE EVENT TRIGGER pause_rollup_expiry ON sql_drop EXECUTE FUNCTION pause_rollup_expiry();", day - 9)))
        .execute(&pool).await.unwrap();
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(760114)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let store = std::sync::Arc::new(PostgresMaintenanceStore::new(pool.clone()));
    let retention = tokio::spawn({
        let store = store.clone();
        async move { store.cleanup(None).await }
    });
    async fn wait_for_lock(pool: &PgPool, query: &str) {
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if sqlx::query_scalar::<_, bool>(sqlx::AssertSqlSafe(query))
                    .fetch_one(pool)
                    .await
                    .unwrap()
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("maintenance must reach the controlled lock boundary");
    }
    wait_for_lock(&pool, "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND objid=760114 AND NOT granted)").await;
    let compaction = tokio::spawn({
        let store = store.clone();
        async move { store.compact().await }
    });
    wait_for_lock(&pool, "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE relation='statisticsrollups'::regclass AND NOT granted)").await;
    gate.commit().await.unwrap();
    // With the old cross-family transaction, retention now requests the raw
    // parent held by compaction, while compaction waits for its rollup parent.
    let (retention, compaction) = tokio::join!(retention, compaction);
    assert!(
        retention.unwrap().is_ok(),
        "partition expiry must not deadlock compaction"
    );
    assert!(compaction.unwrap().unwrap().processed > 0);
    let expired: i64 =
        sqlx::query_scalar("SELECT count(*) FROM statisticssamples WHERE created<$1")
            .bind((day - 7) * 86400)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(expired, 0);
    let summaries: i64 =
        sqlx::query_scalar("SELECT count(*) FROM statisticsrollups WHERE sourceid=$1")
            .bind(source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(summaries > 0);
    drop(store);
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn later_partition_failure_preserves_completed_expiry_and_retries_remaining_family() {
    let (pool, database) = storage_fixture().await;
    let day = chrono::Utc::now().timestamp() / 86400 - 9;
    for parent in ["statisticssamples", "statisticsrollups"] {
        sqlx::query("SELECT statistics_partition($1,$2)")
            .bind(parent)
            .bind(day)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::raw_sql("ALTER FUNCTION statistics_partition(text,bigint) RENAME TO original_statistics_partition;
        CREATE FUNCTION statistics_partition(parent text,day bigint) RETURNS boolean LANGUAGE plpgsql AS $$ BEGIN
        IF parent='statisticsrollups' THEN RAISE EXCEPTION 'fixture partition failure'; END IF;
        RETURN original_statistics_partition(parent,day); END $$;")
        .execute(&pool).await.unwrap();
    let store = PostgresMaintenanceStore::new(pool.clone());
    assert!(store.cleanup(None).await.is_err());
    async fn exists(pool: &PgPool, parent: &str, day: i64) -> bool {
        sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
            .bind(format!("public.{parent}_d{day}"))
            .fetch_one(pool)
            .await
            .unwrap()
    }
    assert!(!exists(&pool, "statisticssamples", day).await);
    assert!(exists(&pool, "statisticsrollups", day).await);
    sqlx::raw_sql("DROP FUNCTION statistics_partition(text,bigint); ALTER FUNCTION original_statistics_partition(text,bigint) RENAME TO statistics_partition;")
        .execute(&pool).await.unwrap();
    store.cleanup(None).await.unwrap();
    assert!(!exists(&pool, "statisticsrollups", day).await);
    close_storage_fixture(pool, database).await;
}
