use super::*;
use sqlx::{Column, Connection};

const HISTORY: &str =
    include_str!("../../src/persistence/postgres/platforms/statistics/container_history.sql");

async fn values(pool: &PgPool, ids: &[Uuid], start: i64, bucket: i64) -> Vec<serde_json::Value> {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT row_to_json(h) FROM ({HISTORY}) h"
    )))
    .bind(ids)
    .bind(start)
    .bind(bucket)
    .bind(start + 86399)
    .fetch_all(pool)
    .await
    .unwrap()
}

fn check_aggregation_work(node: &serde_json::Value) {
    if node["Node Type"] == "Aggregate" {
        assert!(
            node["Actual Loops"].as_f64().unwrap_or_default() <= 1.,
            "history aggregates must not be recomputed for each output bucket: {node}"
        );
    }
    if let Some(children) = node["Plans"].as_array() {
        for child in children {
            check_aggregation_work(child);
        }
    }
}

async fn check_plans(pool: &PgPool, id: Uuid, start: i64, expected: &[serde_json::Value]) {
    for mode in ["force_custom_plan", "force_generic_plan"] {
        // A new physical session models pool recycling: no warmed generic plan
        // can hide the custom-plan regression after the partition shrinks.
        let mut connection = sqlx::PgConnection::connect_with(&pool.connect_options())
            .await
            .unwrap();
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "SET plan_cache_mode={mode}; SET jit=off; SET statement_timeout='5s'"
        )))
        .execute(&mut connection)
        .await
        .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "PREPARE reclaimed_history(uuid[],bigint,bigint,bigint) AS {HISTORY}"
        )))
        .execute(&mut connection)
        .await
        .unwrap();
        let execute = format!(
            "EXECUTE reclaimed_history(ARRAY['{id}']::uuid[],{start},60,{})",
            start + 86399
        );
        let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "EXPLAIN(ANALYZE,BUFFERS,TIMING OFF,FORMAT JSON) {execute}"
        )))
        .fetch_one(&mut connection)
        .await
        .unwrap();
        check_aggregation_work(&plan[0]["Plan"]);
        let rows = sqlx::query(sqlx::AssertSqlSafe(execute))
            .fetch_all(&mut connection)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1440, "mode={mode}");
        let actual: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| {
                let mut value = serde_json::Map::new();
                for column in row.columns() {
                    let name = column.name();
                    value.insert(
                        name.to_owned(),
                        match name {
                            "containerid" => serde_json::json!(row.get::<Uuid, _>(name)),
                            "created" => serde_json::json!(row.get::<i64, _>(name)),
                            _ => serde_json::json!(row.get::<Option<f64>, _>(name)),
                        },
                    );
                }
                value.into()
            })
            .collect();
        assert_history_close(expected, &actual);
        connection.close().await.unwrap();
    }
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn reclaimed_history_aggregates_once_with_fresh_custom_and_generic_plans() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() / 86400 - 3) * 86400;
    let mut ids = Vec::new();
    let mut sources = Vec::new();
    for n in 0..20 {
        let id = add_container(&pool, &scope, &format!("history-plan-{n}")).await;
        ids.push(id);
        sources.push(
            sqlx::query_scalar::<_, i64>("SELECT statistics_source($1,$2,NULL)")
                .bind(scope.platform_id)
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap(),
        );
    }
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memoryactivecache,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
        SELECT s,$2+m*60+t,10,100,CASE WHEN m%7=0 THEN NULL ELSE 5 END,5,200,
            (m%100)*60+t,m*120+t,CASE WHEN m%11=0 THEN NULL ELSE m*60+t END,m*60+t
        FROM unnest($1::bigint[]) s CROSS JOIN generate_series(0,1439)m CROSS JOIN generate_series(0,50,10)t
        WHERE NOT(s=$3 AND (m%60=0 OR m=1439) AND t>0) ORDER BY m,t,s")
        .bind(&sources).bind(start).bind(sources[0]).execute(&pool).await.unwrap();
    // Current-day observations keep the final old minute eligible for compaction.
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
        SELECT s,$2+n*10,10,100,5,200,n,n FROM unnest($1::bigint[])s CROSS JOIN generate_series(1,180)n")
        .bind(&sources).bind(start+3*86400).execute(&pool).await.unwrap();
    sqlx::raw_sql("ANALYZE statisticssources; ANALYZE statisticssamples;")
        .execute(&pool)
        .await
        .unwrap();
    let mut expected = Vec::new();
    for selected in [&ids[0..1], &ids[1..2], &ids[0..2]] {
        for bucket in [60, 300, 3600] {
            expected.push((
                selected.to_vec(),
                bucket,
                values(&pool, selected, start, bucket).await,
            ));
        }
    }
    check_plans(&pool, ids[1], start, &expected[3].2).await;
    for source in sources {
        for hour in 0..24 {
            sqlx::query(include_str!(
                "../../src/persistence/postgres/platforms/statistics/compact.sql"
            ))
            .bind(source)
            .bind(start + hour * 3600)
            .bind(start + (hour + 1) * 3600)
            .execute(&pool)
            .await
            .unwrap();
        }
    }
    let child = format!("statisticssamples_d{}", start / 86400);
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("VACUUM (ANALYZE) {child}")))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql("ANALYZE ONLY statisticssamples; ANALYZE statisticsrollups;")
        .execute(&pool)
        .await
        .unwrap();
    for reclaimed in [false, true] {
        if reclaimed {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "CLUSTER {child} USING {child}_pkey"
            )))
            .execute(&pool)
            .await
            .unwrap();
        }
        for (selected, bucket, expected) in &expected {
            assert_history_close(expected, &values(&pool, selected, start, *bucket).await);
        }
        check_plans(&pool, ids[1], start, &expected[3].2).await;
    }
    close_storage_fixture(pool, database).await;
}

fn scanned_history_rows(node: &serde_json::Value) -> f64 {
    let relation = node["Relation Name"].as_str().unwrap_or_default();
    let own =
        if relation.starts_with("statisticssamples") || relation.starts_with("statisticsrollups") {
            (node["Actual Rows"].as_f64().unwrap_or_default()
                + node["Rows Removed by Filter"].as_f64().unwrap_or_default()
                + node["Rows Removed by Index Recheck"]
                    .as_f64()
                    .unwrap_or_default())
                * node["Actual Loops"].as_f64().unwrap_or_default()
        } else {
            0.
        };
    own + node["Plans"]
        .as_array()
        .map(|children| children.iter().map(scanned_history_rows).sum::<f64>())
        .unwrap_or_default()
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn mixed_platform_history_bounds_sort_work_for_small_installations() {
    for containers in [5i64, 15] {
        let (pool, database) = storage_fixture().await;
        let scope = platform(&pool).await;
        let start = chrono::Utc::now().timestamp() / 86400 * 86400;
        // One platform and five or fifteen full-length Docker identities: six compacted
        // days followed by one raw day, with counters resetting at each midnight.
        sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,containerio)
        SELECT gen_random_uuid(),$1,$2+n*10,2,100,n*200,n*400,
            (SELECT jsonb_agg(jsonb_build_object('containerId',repeat(md5(c::text),2),
                'rxBytes',n*10,'txBytes',n*20,'blockReadBytes',n*30,'blockWriteBytes',n*40))
             FROM generate_series(1,$3)c) FROM generate_series(0,8639)n")
        .bind(scope.platform_id).bind(start).bind(containers).execute(&pool).await.unwrap();
        let source: i64 = sqlx::query_scalar(
            "INSERT INTO platformhistorysources(platformid,nodeid) VALUES($1,'') RETURNING id",
        )
        .bind(scope.platform_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        for day in 1..=6 {
            sqlx::query("INSERT INTO platformiorollups(sourceid,containerid,created,lastcreated,firstcounters,lastcounters,counterdeltas,counterdurations,blockreadbytes,blockwritebytes,readcount,writecount)
            SELECT $1,repeat(md5(c::text),2),$2+n*60,$2+n*60+50,
                ARRAY[n*60,n*120,n*180,n*240]::float8[],ARRAY[n*60+50,n*120+100,n*180+150,n*240+200]::float8[],
                ARRAY[50,100,150,200]::float8[],ARRAY[50,50,50,50]::bigint[],n*180+75,n*240+100,6,6
            FROM generate_series(0,1439)n CROSS JOIN generate_series(1,$3)c ORDER BY n,c")
            .bind(source).bind(start-day*86400).bind(containers).execute(&pool).await.unwrap();
        }
        sqlx::raw_sql(
            "ANALYZE platformstats; ANALYZE platformhistorysources; ANALYZE platformiorollups",
        )
        .execute(&pool)
        .await
        .unwrap();
        let history = include_str!(
            "../../src/persistence/postgres/platforms/statistics/platform_history.sql"
        );
        let mut expected: Option<Vec<serde_json::Value>> = None;
        let mut expected_windows = Vec::new();
        let mut fine_work = Vec::new();
        for coarse in [false, true] {
            if coarse {
                sqlx::query("UPDATE platformhistorysources SET compacteduntil=$2 WHERE id=$1")
                    .bind(source)
                    .bind(start)
                    .execute(&pool)
                    .await
                    .unwrap();
                for block in 0..576 {
                    let from = start - 6 * 86400 + block * 900;
                    sqlx::query(include_str!(
                        "../../src/persistence/postgres/platforms/statistics/platform_coarse.sql"
                    ))
                    .bind(source)
                    .bind(from)
                    .bind(from + 900)
                    .execute(&pool)
                    .await
                    .unwrap();
                }
                sqlx::query("ANALYZE platformiocoarserollups")
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            for (mode_index, mode) in ["force_custom_plan", "force_generic_plan"]
                .into_iter()
                .enumerate()
            {
                let mut connection = sqlx::PgConnection::connect_with(&pool.connect_options())
                    .await
                    .unwrap();
                sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                    "SET plan_cache_mode={mode}; SET jit=off; SET work_mem='4MB';
             SET max_parallel_workers_per_gather=0; SET statement_timeout='15s';
             PREPARE mixed_platform_history(uuid,bigint,bigint,bigint) AS
             SELECT row_to_json(h) FROM ({history}) h"
                )))
                .execute(&mut connection)
                .await
                .unwrap();
                let execute = format!(
                    "EXECUTE mixed_platform_history('{}',{},900,{})",
                    scope.platform_id,
                    start - 6 * 86400,
                    start + 86399
                );
                let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "EXPLAIN(ANALYZE,BUFFERS,TIMING OFF,FORMAT JSON) {execute}"
                )))
                .fetch_one(&mut connection)
                .await
                .unwrap();
                // The previous whole-window sort wrote 48 MiB even with numeric IDs.
                // Bounded aggregation must avoid spilling at the same memory allowance.
                fn window_rows(node: &serde_json::Value) -> f64 {
                    let own = if node["Node Type"] == "WindowAgg" {
                        node["Actual Rows"].as_f64().unwrap()
                            * node["Actual Loops"].as_f64().unwrap()
                    } else {
                        0.
                    };
                    own + node["Plans"]
                        .as_array()
                        .map(|p| p.iter().map(window_rows).sum::<f64>())
                        .unwrap_or_default()
                }
                let work = window_rows(&plan[0]["Plan"]);
                if coarse {
                    assert!(
                        work < fine_work[mode_index] * 0.65,
                        "coarse work={work}, minute work={}",
                        fine_work[mode_index]
                    );
                } else {
                    fine_work.push(work);
                }
                let written = plan[0]["Plan"]["Temp Written Blocks"].as_u64().unwrap();
                assert_eq!(written, 0, "mode={mode}, temporary blocks={written}");
                let actual: Vec<serde_json::Value> =
                    sqlx::query_scalar(sqlx::AssertSqlSafe(execute))
                        .fetch_all(&mut connection)
                        .await
                        .unwrap();
                assert_eq!(actual.len(), 7 * 96);
                for (field, multiplier) in [
                    ("rxbytesdelta", 1.),
                    ("txbytesdelta", 2.),
                    ("blockreadbytesdelta", 3.),
                    ("blockwritebytesdelta", 4.),
                ] {
                    let total: f64 = actual.iter().map(|row| row[field].as_f64().unwrap()).sum();
                    assert_eq!(
                        total,
                        containers as f64 * 7. * 86390. * multiplier,
                        "{mode}: {field}"
                    );
                }
                assert_eq!(actual[0]["rxbytespersecond"], containers as f64);
                assert_eq!(actual[671]["rxbytespersecond"], containers as f64);
                if let Some(expected) = &expected {
                    assert_history_close(expected, &actual);
                } else {
                    expected = Some(actual);
                }
                for (case, (minutes, bucket)) in [
                    (15, 10),
                    (60, 10),
                    (360, 30),
                    (1440, 60),
                    (2880, 300),
                    (4320, 300),
                    (5760, 900),
                    (7200, 900),
                ]
                .into_iter()
                .enumerate()
                {
                    let execute = format!(
                        "EXECUTE mixed_platform_history('{}',{},{bucket},{})",
                        scope.platform_id,
                        start + 86400 - minutes * 60,
                        start + 86399
                    );
                    let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                        "EXPLAIN(ANALYZE,BUFFERS,TIMING OFF,FORMAT JSON) {execute}"
                    )))
                    .fetch_one(&mut connection)
                    .await
                    .unwrap();
                    assert_eq!(
                        plan[0]["Plan"]["Temp Written Blocks"], 0,
                        "{mode}, {minutes} minutes"
                    );
                    let actual: Vec<serde_json::Value> =
                        sqlx::query_scalar(sqlx::AssertSqlSafe(execute))
                            .fetch_all(&mut connection)
                            .await
                            .unwrap();
                    assert_eq!(actual.len() as i64, minutes * 60 / bucket);
                    let total: f64 = actual
                        .iter()
                        .filter_map(|row| row["rxbytesdelta"].as_f64())
                        .sum();
                    let per_container = if minutes < 1440 {
                        minutes * 60 - 10
                    } else {
                        minutes / 1440 * 86390
                    };
                    assert_eq!(
                        total,
                        containers as f64 * per_container as f64,
                        "{mode}, {minutes} minutes"
                    );
                    if mode == "force_custom_plan" && !coarse {
                        expected_windows.push(actual);
                    } else {
                        assert_history_close(&expected_windows[case], &actual);
                    }
                }
                connection.close().await.unwrap();
            }
        }
        close_storage_fixture(pool, database).await;
    }
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn small_catalog_history_seeks_selected_sources_in_raw_and_rollup_partitions() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = chrono::Utc::now().timestamp() / 86400 * 86400;
    let mut ids = Vec::new();
    let mut sources = Vec::new();
    // One platform, twenty established containers and one recent replacement.
    // A large source catalog hides the generic planner's timestamp-index choice.
    for n in 0..21 {
        let id = add_container(&pool, &scope, &format!("source-plan-{n}")).await;
        ids.push(id);
        sources.push(
            sqlx::query_scalar::<_, i64>("SELECT statistics_source($1,$2,NULL)")
                .bind(scope.platform_id)
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap(),
        );
    }
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
        SELECT s,$2+n*10,1,100,5,200,n*100,n*200,n*10,n*20
        FROM generate_series(0,8639)n CROSS JOIN unnest($1::bigint[])s ORDER BY n,s")
        .bind(&sources[..20]).bind(start).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
        SELECT $1,$2+n*10,1,100,5,200,n*100,n*200,n*10,n*20
        FROM generate_series(8000,8359)n")
        .bind(sources[20]).bind(start).execute(&pool).await.unwrap();
    for days in 1..=3 {
        sqlx::query("INSERT INTO statisticsrollups(sourceid,created,lastcreated,samplecount,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,
            readcount,writecount,blockreadbytes,blockwritebytes,firstcounters,lastcounters,counterdeltas,counterdurations,peaks)
            SELECT s,$2+n*60,$2+n*60+50,6,1,100,5,200,n*100,n*200,6,6,0,0,
                ARRAY[n*100,n*200,0,0]::float8[],ARRAY[n*100+50,n*200+100,0,0]::float8[],
                ARRAY[50,100,0,0]::float8[],ARRAY[50,50,50,50]::bigint[],ARRAY[1,100,5,200]::float8[]
            FROM generate_series(0,1439)n CROSS JOIN unnest($1::bigint[])s ORDER BY n,s")
            .bind(&sources[..20]).bind(start-days*86400).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql("ANALYZE statisticssources; ANALYZE containers; ANALYZE statisticssamples; ANALYZE statisticsrollups")
        .execute(&pool).await.unwrap();
    let mut expected = Vec::new();
    for mode in ["force_custom_plan", "force_generic_plan", "auto"] {
        let mut connection = sqlx::PgConnection::connect_with(&pool.connect_options())
            .await
            .unwrap();
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "SET plan_cache_mode={mode}; SET jit=off; SET statement_timeout='5s';
             PREPARE selected_history(uuid[],bigint,bigint,bigint) AS
             SELECT row_to_json(h) FROM ({HISTORY}) h"
        )))
        .execute(&mut connection)
        .await
        .unwrap();
        for (case, selected) in [vec![ids[20]], vec![ids[0]], vec![ids[0], ids[20]]]
            .iter()
            .enumerate()
        {
            let literals = selected
                .iter()
                .map(|id| format!("'{id}'"))
                .collect::<Vec<_>>()
                .join(",");
            let execute = format!(
                "EXECUTE selected_history(ARRAY[{literals}]::uuid[],{},900,{})",
                start - 3 * 86400,
                start + 86399
            );
            // Exercise ordinary prepared-statement reuse past the five custom
            // executions used to decide whether to adopt a generic plan.
            for _ in 0..if mode == "auto" { 8 } else { 1 } {
                let actual: Vec<serde_json::Value> =
                    sqlx::query_scalar(sqlx::AssertSqlSafe(execute.clone()))
                        .fetch_all(&mut connection)
                        .await
                        .unwrap();
                assert_eq!(
                    actual.len(),
                    [5, 384, 389][case],
                    "mode={mode}, case={case}"
                );
                if mode == "force_custom_plan" {
                    expected.push(actual);
                } else {
                    assert_history_close(&expected[case], &actual);
                }
            }
            let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "EXPLAIN(ANALYZE,BUFFERS,TIMING OFF,FORMAT JSON) {execute}"
            )))
            .fetch_one(&mut connection)
            .await
            .unwrap();
            // At most two raw passes and one materialized rollup pass. The new
            // container has no rollups; unrelated partitions must remain cheap.
            let scanned = scanned_history_rows(&plan[0]["Plan"]);
            assert!(
                scanned <= [720., 21600., 22320.][case],
                "mode={mode}, case={case}, scanned={scanned}: {plan}"
            );
            if case == 0 {
                let root = &plan[0]["Plan"];
                let blocks = root["Shared Hit Blocks"].as_u64().unwrap()
                    + root["Shared Read Blocks"].as_u64().unwrap();
                assert!(blocks < 2000, "mode={mode}, blocks={blocks}: {plan}");
            }
        }
        connection.close().await.unwrap();
    }
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn platform_history_bridges_chunks_without_losing_resets_nulls_or_node_identity() {
    use citadel_platforms::ContainerIoSnapshot;
    let (pool, database) = storage_fixture().await;
    let manager = platform(&pool).await;
    let worker = history_node(&pool, &manager, "worker").await;
    let start = chrono::Utc::now().timestamp() / 86400 * 86400;
    let times = [0, 890, 900, 910, 1800, 3590, 3600, 3610, 5400, 5410, 7190];
    let counters = [100., 110., 5., 15., 25., 35., 45., 55., 65., 75., 85.];
    let reads = [
        Some(100.),
        None,
        Some(5.),
        Some(15.),
        None,
        Some(35.),
        Some(45.),
        None,
        Some(65.),
        Some(75.),
        None,
    ];
    let mut captures = Vec::new();
    for (scope, factor) in [(&manager, 1.), (&worker, 10.)] {
        for (i, time) in times.iter().enumerate() {
            let mut capture = host(scope, start + time, 1);
            capture.container_io = vec![ContainerIoSnapshot {
                node_id: None,
                created: None,
                container_id: "same-id".into(),
                rx_bytes: counters[i] * factor,
                tx_bytes: counters[i] * factor * 2.,
                block_read_bytes: reads[i].map(|v| v * factor),
                block_write_bytes: None,
            }];
            captures.push(capture);
        }
    }
    PostgresStatsBatchStore::new(pool.clone())
        .persist_batch(&captures)
        .await
        .unwrap();
    fn sum(values: &[f64]) -> Option<f64> {
        (!values.is_empty()).then(|| values.iter().sum())
    }
    fn rate(values: &[f64], seconds: i64) -> Option<f64> {
        sum(values)
            .filter(|_| seconds > 0)
            .map(|v| v / seconds as f64)
    }
    for mixed in [false, true] {
        if mixed {
            // A node with unavailable counters exercises mixed execution without
            // changing the independently calculated expected host/worker values.
            sqlx::query("WITH src AS (
                INSERT INTO platformhistorysources(platformid,nodeid) VALUES($1,'summary') RETURNING id
            ) INSERT INTO platformiorollups(sourceid,containerid,created,lastcreated,
                firstcounters,lastcounters,counterdeltas,counterdurations,blockreadbytes,blockwritebytes,readcount,writecount)
                SELECT id,'unavailable',$2+900,$2+950,ARRAY[NULL,NULL,NULL,NULL]::float8[],ARRAY[NULL,NULL,NULL,NULL]::float8[],
                    ARRAY[NULL,NULL,NULL,NULL]::float8[],ARRAY[0,0,0,0]::bigint[],NULL,NULL,0,0 FROM src")
                .bind(manager.platform_id).bind(start).execute(&pool).await.unwrap();
        }
        for (from, until) in [(0, 7200), (1, 7195), (900, 5407), (0, 3500)] {
            for bucket in [60, 300, 900, 3600, 137] {
                // Independent sequential counter calculation: no SQL windows,
                // chunk boundaries or summary machinery in the expected result.
                let mut expected = std::collections::BTreeMap::<i64, [Vec<f64>; 5]>::new();
                for factor in [1., 10.] {
                    let mut buckets = std::collections::BTreeMap::<
                        i64,
                        (Vec<f64>, i64, Vec<f64>, i64, Vec<f64>),
                    >::new();
                    let mut previous = None;
                    for (i, time) in times
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| **t >= from && **t <= until)
                    {
                        let entry = buckets.entry((start + time) / bucket).or_default();
                        if let Some(v) = reads[i] {
                            entry.4.push(v * factor);
                        }
                        if let Some(p) = previous {
                            let p: usize = p;
                            entry.0.push(if counters[i] >= counters[p] {
                                (counters[i] - counters[p]) * factor
                            } else {
                                counters[i] * factor
                            });
                            entry.1 += time - times[p];
                            if let (Some(v), Some(prev)) = (reads[i], reads[p]) {
                                entry.2.push(if v >= prev {
                                    (v - prev) * factor
                                } else {
                                    v * factor
                                });
                                entry.3 += time - times[p];
                            }
                        }
                        previous = Some(i);
                    }
                    for (bucket, (rx, rx_seconds, read, read_seconds, means)) in buckets {
                        let values = [
                            sum(&rx),
                            rate(&rx, rx_seconds),
                            sum(&read),
                            rate(&read, read_seconds),
                            sum(&means).map(|v| v / means.len() as f64),
                        ];
                        let entry = expected.entry(bucket).or_default();
                        for (slot, value) in entry.iter_mut().zip(values) {
                            if let Some(v) = value {
                                slot.push(v);
                            }
                        }
                    }
                }
                let actual = platform_history(
                    &pool,
                    manager.platform_id,
                    start + from,
                    start + until,
                    bucket,
                )
                .await;
                assert_eq!(actual.len(), expected.len());
                for row in actual {
                    let expected = &expected[&(row["created"].as_i64().unwrap() / bucket)];
                    for (field, values) in [
                        "rxbytesdelta",
                        "rxbytespersecond",
                        "blockreadbytesdelta",
                        "blockreadbytespersecond",
                        "blockreadbytes",
                    ]
                    .into_iter()
                    .zip(expected)
                    {
                        match (row[field].as_f64(), sum(values)) {
                            (Some(actual), Some(expected)) => assert!(
                                (actual - expected).abs() <= expected.abs().max(1.) * 1e-10,
                                "{field}, mixed={mixed}, from={from}, until={until}, bucket={bucket}: {actual} != {expected}"
                            ),
                            (actual, expected) => assert_eq!(actual, expected, "{field}"),
                        }
                    }
                    assert!(row["blockwritebytesdelta"].is_null());
                    assert!(row["blockwritebytespersecond"].is_null());
                }
            }
        }
    }
    close_storage_fixture(pool, database).await;
}
