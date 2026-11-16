use super::*;
use sqlx::Connection;

const COARSE: &str =
    include_str!("../../src/persistence/postgres/platforms/statistics/platform_coarse.sql");
const HISTORY: &str =
    include_str!("../../src/persistence/postgres/platforms/statistics/platform_history.sql");

async fn source(pool: &PgPool, platform: Uuid, node: &str, until: i64) -> i64 {
    sqlx::query_scalar("INSERT INTO platformhistorysources(platformid,nodeid,compacteduntil) VALUES($1,$2,$3) RETURNING id")
        .bind(platform).bind(node).bind(until).fetch_one(pool).await.unwrap()
}

async fn summarize(pool: &PgPool, source: i64, start: i64) {
    sqlx::query(COARSE)
        .bind(source)
        .bind(start)
        .bind(start + 900)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn coarse_history_preserves_fine_partial_and_long_ranges_and_physical_identity() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() / 86400 - 3) * 86400;
    let mut sources = Vec::new();
    for node in ["", "worker"] {
        let s = source(&pool, scope.platform_id, node, start + 7200).await;
        sources.push(s);
        // Deliberately reset every 23 minutes, omit minutes/containers and break
        // nullable counter baselines. The same Docker IDs exist on both nodes.
        sqlx::query("INSERT INTO platformiorollups
            SELECT $1,c::text,$2+m*60,$2+m*60+50,
            ARRAY[(m%23)*10,(m%23)*20,CASE WHEN m%17=0 THEN NULL ELSE (m%23)*30 END,0]::float8[],
            ARRAY[(m%23)*10+5,(m%23)*20+10,CASE WHEN m%19=0 THEN NULL ELSE (m%23)*30+15 END,0]::float8[],
            ARRAY[5,10,CASE WHEN m%17=0 OR m%19=0 THEN NULL ELSE 15 END,0]::float8[],
            ARRAY[50,50,CASE WHEN m%17=0 OR m%19=0 THEN 0 ELSE 50 END,50]::bigint[],
            CASE WHEN m%17=0 THEN NULL ELSE (m%23)*30+7 END,0,CASE WHEN m%17=0 THEN 0 ELSE 6 END,6
            FROM generate_series(0,119)m CROSS JOIN generate_series(1,3)c
            WHERE m%13<>0 AND NOT(c=2 AND m<8) AND NOT(c=3 AND m>=80)
                AND NOT($3 AND m=28)")
            .bind(s).bind(start).bind(!node.is_empty()).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO platformgaugerollups SELECT $1,$2+m*60,$2+m*60+50,6,m,100,2,3,50,100,50,ARRAY[6,6,6] FROM generate_series(0,119)m")
        .bind(sources[0]).bind(start).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO platformnodeiostats(platformid,nodeid,created,containerio) VALUES($1,'worker',$2,'[{\"containerId\":\"1\",\"rxBytes\":1,\"txBytes\":2,\"blockReadBytes\":null,\"blockWriteBytes\":0}]')")
        .bind(scope.platform_id).bind(start+28*60).execute(&pool).await.unwrap();
    let windows = [
        (0, 7199, 10),
        (0, 7199, 30),
        (0, 7199, 60),
        (0, 7199, 300),
        (0, 7199, 900),
        (0, 7199, 3600),
        (73, 7143, 900),
        (301, 6477, 300),
        (311, 1393, 60),
        (440, 581, 10),
        (1440, 2399, 900),
        (1800, 2399, 300),
        (0, 7199, 137),
    ];
    let mut expected = Vec::new();
    for (from, until, bucket) in windows {
        expected.push(
            platform_history(
                &pool,
                scope.platform_id,
                start + from,
                start + until,
                bucket,
            )
            .await,
        );
    }
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM platformiorollups")
        .fetch_one(&pool)
        .await
        .unwrap();
    // Rolling back a completed statement must roll back BOTH summaries and its
    // durable progress. The next maintenance call can safely retry it.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(COARSE)
        .bind(sources[0])
        .bind(start)
        .bind(start + 900)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    let state: (i64,i64) = sqlx::query_as("SELECT coarseneduntil,(SELECT count(*) FROM platformiocoarserollups) FROM platformhistorysources WHERE id=$1")
        .bind(sources[0]).fetch_one(&pool).await.unwrap();
    assert_eq!(state, (0, 0));
    // Exercise the actual worker/store path, including a cold cursor and idempotent
    // restart. No raw minute is eligible for further minute compaction here.
    for _ in 0..20 {
        if citadel_adapters::persistence::postgres::maintenance::compact(&pool)
            .await
            .unwrap()
            == 0
        {
            break;
        }
    }
    for s in &sources {
        let end: i64 =
            sqlx::query_scalar("SELECT coarseneduntil FROM platformhistorysources WHERE id=$1")
                .bind(s)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(end, start + 7200);
    }
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM platformiocoarserollups")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(rows > 0 && rows < before / 5);
    let skipped: i64 = sqlx::query_scalar("SELECT count(*) FROM platformiocoarserollups WHERE sourceid=$1 AND created>=$2 AND created<$2+900")
        .bind(sources[1]).bind(start+900).fetch_one(&pool).await.unwrap();
    assert_eq!(
        skipped, 0,
        "a raw observation forbids bridging over this block"
    );
    assert_eq!(
        citadel_adapters::persistence::postgres::maintenance::compact(&pool)
            .await
            .unwrap(),
        0
    );
    for mode in ["force_custom_plan", "force_generic_plan"] {
        let mut connection = sqlx::PgConnection::connect_with(&pool.connect_options())
            .await
            .unwrap();
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("SET jit=off; SET plan_cache_mode={mode}; PREPARE coarse_history(uuid,bigint,bigint,bigint) AS SELECT to_jsonb(h) FROM ({HISTORY}) h")))
            .execute(&mut connection).await.unwrap();
        for (i, (from, until, bucket)) in windows.into_iter().enumerate() {
            let actual: Vec<serde_json::Value> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "EXECUTE coarse_history('{}',{},{},{})",
                scope.platform_id,
                start + from,
                bucket,
                start + until
            )))
            .fetch_all(&mut connection)
            .await
            .unwrap();
            assert_history_close(&expected[i], &actual);
        }
        connection.close().await.unwrap();
    }
    assert_eq!(
        before,
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM platformiorollups")
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn coarse_history_skips_oversized_blocks_and_never_seals_unfinished_minutes() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() / 86400 - 3) * 86400;
    let s = source(&pool, scope.platform_id, "", start + 899).await;
    sqlx::query("INSERT INTO platformiorollups SELECT $1,c::text,$2,$2+50,ARRAY[0,0,0,0]::float8[],ARRAY[1,1,1,1]::float8[],ARRAY[1,1,1,1]::float8[],ARRAY[50,50,50,50]::bigint[],1,1,6,6 FROM generate_series(1,10001)c")
        .bind(s).bind(start).execute(&pool).await.unwrap();
    summarize(&pool, s, start).await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT coarseneduntil FROM platformhistorysources WHERE id=$1"
        )
        .bind(s)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    sqlx::query("UPDATE platformhistorysources SET compacteduntil=$2 WHERE id=$1")
        .bind(s)
        .bind(start + 900)
        .execute(&pool)
        .await
        .unwrap();
    summarize(&pool, s, start).await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT coarseneduntil FROM platformhistorysources WHERE id=$1"
        )
        .bind(s)
        .fetch_one(&pool)
        .await
        .unwrap(),
        start + 900
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM platformiocoarserollups")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM platformiorollups")
            .fetch_one(&pool)
            .await
            .unwrap(),
        10001
    );
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn coarse_history_retention_expires_daily_default_and_boundary_rows() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let now = chrono::Utc::now().timestamp();
    let s = source(&pool, scope.platform_id, "", now).await;
    let old_day = now / 86400 - 9;
    sqlx::query("SELECT statistics_partition('platformiocoarserollups',$1)")
        .bind(old_day)
        .execute(&pool)
        .await
        .unwrap();
    // One expired daily partition, an older default row, an expired boundary
    // row, and a retained row. Maintenance must preserve the latter and source.
    for t in [
        old_day * 86400,
        (old_day - 2) * 86400,
        now - 604800 - 60,
        now - 2 * 86400,
    ] {
        sqlx::query("INSERT INTO platformiocoarserollups SELECT $1,'retention',$2,$2+50,ARRAY[0,0,0,0]::float8[],ARRAY[1,1,1,1]::float8[],ARRAY[1,1,1,1]::float8[],ARRAY[50,50,50,50]::bigint[],1,1,6,6")
            .bind(s).bind(t).execute(&pool).await.unwrap();
    }
    citadel_adapters::persistence::postgres::maintenance::cleanup(&pool, None)
        .await
        .unwrap();
    let rows: Vec<i64> =
        sqlx::query_scalar("SELECT created FROM platformiocoarserollups WHERE sourceid=$1")
            .bind(s)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows, vec![now - 2 * 86400]);
    let dropped: bool = sqlx::query_scalar(
        "SELECT to_regclass('public.platformiocoarserollups_d'||$1::text) IS NULL",
    )
    .bind(old_day.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(dropped);
    // Platform deletion owns source and both tiers through existing FK cascades.
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM platformiocoarserollups")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    close_storage_fixture(pool, database).await;
}
