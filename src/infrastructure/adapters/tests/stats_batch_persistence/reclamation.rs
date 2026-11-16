use super::*;

async fn seed(pool: &PgPool, source: i64, day: i64, sparse: bool) -> String {
    sqlx::query("SELECT statistics_partition('statisticssamples',$1)")
        .bind(day)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memoryactivecache,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
        SELECT $1,$2+n,n,100,CASE WHEN n%2=0 THEN NULL ELSE 5 END,5,200,n,n,NULL,n FROM generate_series(0,79999)n")
        .bind(source).bind(day*86400).execute(pool).await.unwrap();
    let child = format!("statisticssamples_d{day}");
    if sparse {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {child} WHERE (created-{})%10000<>0 AND created<>{}",
            day * 86400,
            day * 86400 + 79999
        )))
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("VACUUM (ANALYZE) {child}")))
        .execute(pool)
        .await
        .unwrap();
    assert!(size(pool, &child).await >= 8 * 1024 * 1024);
    child
}

async fn size(pool: &PgPool, child: &str) -> i64 {
    sqlx::query_scalar("SELECT pg_total_relation_size($1::regclass)")
        .bind(child)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn fingerprint(pool: &PgPool, child: &str) -> String {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT md5(string_agg(row_to_json(s)::text,'' ORDER BY sourceid,created)) FROM {child} s"
    )))
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn cleanup(pool: &PgPool) {
    citadel_adapters::persistence::postgres::maintenance::cleanup(pool, None)
        .await
        .unwrap();
}

async fn source(pool: &PgPool) -> i64 {
    let scope = platform(pool).await;
    let container = add_container(pool, &scope, "sparse-history").await;
    sqlx::query_scalar("SELECT statistics_source($1,$2,NULL)")
        .bind(scope.platform_id)
        .bind(container)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn reclaims_one_sparse_day_preserving_observations_and_snapshot_visibility() {
    let (pool, database) = storage_fixture().await;
    let src = source(&pool).await;
    let today = chrono::Utc::now().timestamp() / 86400;
    let a = seed(&pool, src, today - 4, true).await;
    let b = seed(&pool, src, today - 3, true).await;
    let dense = seed(&pool, src, today - 2, false).await;
    let recent = seed(&pool, src, today, true).await;
    let before_a = fingerprint(&pool, &a).await;
    let before_b = fingerprint(&pool, &b).await;
    let dense_size = size(&pool, &dense).await;
    let recent_size = size(&pool, &recent).await;
    // Snapshot predates the rewrite but has not locked the raw table yet.
    let mut snapshot = pool.begin().await.unwrap();
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *snapshot)
        .await
        .unwrap();
    sqlx::query("SELECT pg_current_snapshot()")
        .execute(&mut *snapshot)
        .await
        .unwrap();
    cleanup(&pool).await;
    assert!(size(&pool, &a).await < 1024 * 1024);
    assert!(
        size(&pool, &b).await >= 8 * 1024 * 1024,
        "one rewrite per sweep"
    );
    assert_eq!(fingerprint(&pool, &a).await, before_a);
    let visible: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT md5(string_agg(row_to_json(s)::text,'' ORDER BY sourceid,created)) FROM {a} s"
    )))
    .fetch_one(&mut *snapshot)
    .await
    .unwrap();
    assert_eq!(
        visible, before_a,
        "older snapshots must retain the observations"
    );
    snapshot.rollback().await.unwrap();
    cleanup(&pool).await;
    assert!(size(&pool, &b).await < 1024 * 1024);
    assert_eq!(fingerprint(&pool, &b).await, before_b);
    assert_eq!(size(&pool, &dense).await, dense_size);
    assert_eq!(size(&pool, &recent).await, recent_size);
    let file: i64 = sqlx::query_scalar("SELECT pg_relation_filenode($1::regclass)::bigint")
        .bind(&a)
        .fetch_one(&pool)
        .await
        .unwrap();
    cleanup(&pool).await;
    let unchanged: i64 = sqlx::query_scalar("SELECT pg_relation_filenode($1::regclass)::bigint")
        .bind(&a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        file, unchanged,
        "already small partitions must not be rewritten again"
    );
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn skips_busy_partition_then_retries_without_losing_late_capture() {
    let (pool, database) = storage_fixture().await;
    let src = source(&pool).await;
    // Prepare upcoming partitions before the writer, as an ordinary sweep has
    // already done. Creating new partitions itself needs a parent DDL lock.
    cleanup(&pool).await;
    let day = chrono::Utc::now().timestamp() / 86400 - 3;
    let child = seed(&pool, src, day, true).await;
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) VALUES($1,$2,42,1,1,1,1,1)")
        .bind(src).bind(day*86400+80001).execute(&mut *writer).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), cleanup(&pool))
        .await
        .unwrap();
    assert!(size(&pool, &child).await >= 8 * 1024 * 1024);
    writer.commit().await.unwrap();
    let before = fingerprint(&pool, &child).await;
    cleanup(&pool).await;
    assert!(size(&pool, &child).await < 1024 * 1024);
    assert_eq!(fingerprint(&pool, &child).await, before);
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn cancellation_rolls_back_rewrite_and_does_not_lock_current_partition() {
    let (pool, database) = storage_fixture().await;
    let src = source(&pool).await;
    let day = chrono::Utc::now().timestamp() / 86400 - 3;
    let child = seed(&pool, src, day, true).await;
    // An expression index supplies a deterministic gate during index rebuild.
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE FUNCTION reclamation_gate(v bigint) RETURNS bigint LANGUAGE plpgsql IMMUTABLE AS $$ BEGIN PERFORM pg_advisory_xact_lock(914291); RETURN v; END $$; CREATE INDEX reclamation_gate_idx ON {child}(reclamation_gate(sourceid));")))
        .execute(&pool).await.unwrap();
    let before = fingerprint(&pool, &child).await;
    let initial_size = size(&pool, &child).await;
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(914291)")
        .execute(&mut *gate)
        .await
        .unwrap();
    let job = tokio::spawn({
        let pool = pool.clone();
        async move { cleanup(&pool).await }
    });
    tokio::time::timeout(std::time::Duration::from_secs(4),async {
        loop {
            let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND objid=914291 AND NOT granted AND database=(SELECT oid FROM pg_database WHERE datname=current_database()))")
                .fetch_one(&pool).await.unwrap();
            if waiting {break;}
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(1),sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) VALUES($1,$2,1,1,1,1,1,1)")
        .bind(src).bind(chrono::Utc::now().timestamp()).execute(&pool)).await.unwrap().unwrap();
    job.abort();
    assert!(job.await.unwrap_err().is_cancelled());
    gate.rollback().await.unwrap();
    assert_eq!(fingerprint(&pool, &child).await, before);
    assert_eq!(size(&pool, &child).await, initial_size);
    cleanup(&pool).await;
    assert!(size(&pool, &child).await < 1024 * 1024);
    assert_eq!(fingerprint(&pool, &child).await, before);
    close_storage_fixture(pool, database).await;
}
