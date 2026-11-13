use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn platform_history_bounds_temporary_writes_without_losing_counter_totals() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() / 86400 - 1) * 86400;
    sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,containerio)
        SELECT gen_random_uuid(),$1,$2+n*10,2,100,n*200,n*400,
            (SELECT jsonb_agg(jsonb_build_object('containerId',repeat(md5(c::text),2),
                'rxBytes',n*10,'txBytes',n*20,'blockReadBytes',n*30,'blockWriteBytes',n*40))
             FROM generate_series(1,50)c) FROM generate_series(0,8640)n")
        .bind(scope.platform_id).bind(start).execute(&pool).await.unwrap();
    sqlx::query("ANALYZE platformstats")
        .execute(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(
        "SET LOCAL jit=off; SET LOCAL work_mem='4MB'; SET LOCAL max_parallel_workers_per_gather=0;",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(concat!(
        "PREPARE bounded_platform(uuid,bigint,bigint,bigint) AS ",
        include_str!("../../src/persistence/postgres/platforms/statistics/platform_history.sql")
    ))
    .execute(&mut *tx)
    .await
    .unwrap();
    let execute = format!(
        "EXECUTE bounded_platform('{}',{start},60,{})",
        scope.platform_id,
        start + 86400
    );
    let mut expected = None;
    for mixed in [false, true] {
        if mixed {
            // A zero-traffic summary from another node selects the mixed path
            // without changing any expected chart values from the raw host.
            sqlx::query("WITH src AS (
                INSERT INTO platformhistorysources(platformid,nodeid) VALUES($1,'summary-node') RETURNING id
            ) INSERT INTO platformiorollups(sourceid,containerid,created,lastcreated,
                firstcounters,lastcounters,counterdeltas,counterdurations,
                blockreadbytes,blockwritebytes,readcount,writecount)
            SELECT id,'summary-container',$2,$2+50,
                ARRAY[0,0,0,0]::float8[],ARRAY[0,0,0,0]::float8[],ARRAY[0,0,0,0]::float8[],
                ARRAY[50,50,50,50]::bigint[],0,0,6,6 FROM src")
                .bind(scope.platform_id).bind(start).execute(&mut *tx).await.unwrap();
        }
        for mode in ["force_custom_plan", "force_generic_plan"] {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "SET LOCAL plan_cache_mode={mode}"
            )))
            .execute(&mut *tx)
            .await
            .unwrap();
            let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "EXPLAIN(ANALYZE,BUFFERS,TIMING OFF,FORMAT JSON) {execute}"
            )))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
            let block_size: i64 =
                sqlx::query_scalar("SELECT current_setting('block_size')::bigint")
                    .fetch_one(&mut *tx)
                    .await
                    .unwrap();
            let temporary_bytes =
                plan[0]["Plan"]["Temp Written Blocks"].as_i64().unwrap() * block_size;
            // Bound real spill volume rather than machine-dependent latency. With 432,050
            // raw observations with full-length Docker IDs at 4 MiB work_mem, both
            // paths fit below 112 MiB. The old generic mixed-history hash
            // aggregate wrote about 240 MiB for this fixture.
            assert!(
                temporary_bytes < 112 * 1024 * 1024,
                "mixed={mixed}, mode={mode}, temporary writes={temporary_bytes}: {plan}"
            );
            let rows = sqlx::query(sqlx::AssertSqlSafe(execute.clone()))
                .fetch_all(&mut *tx)
                .await
                .unwrap();
            assert_eq!(rows.len(), 1441);
            assert_eq!(
                rows.iter()
                    .map(|r| r.get::<Option<f64>, _>("rxbytesdelta").unwrap_or_default())
                    .sum::<f64>(),
                86400. * 50.
            );
            assert_eq!(
                rows.iter()
                    .map(|r| r
                        .get::<Option<f64>, _>("blockwritebytesdelta")
                        .unwrap_or_default())
                    .sum::<f64>(),
                86400. * 200.
            );
            assert!(
                rows.iter().all(|r| r.get::<f64, _>("cpuusage") == 2.
                    && r.get::<f64, _>("rxbytespersecond") == 50.)
            );
            let values: Vec<_> = rows
                .iter()
                .map(|row| {
                    (
                        row.get::<i64, _>("created"),
                        [
                            "cpuusage",
                            "memoryusage",
                            "rxbytes",
                            "txbytes",
                            "blockreadbytes",
                            "blockwritebytes",
                            "rxbytesdelta",
                            "txbytesdelta",
                            "blockreadbytesdelta",
                            "blockwritebytesdelta",
                            "rxbytespersecond",
                            "txbytespersecond",
                            "blockreadbytespersecond",
                            "blockwritebytespersecond",
                        ]
                        .map(|field| row.get::<Option<f64>, _>(field)),
                    )
                })
                .collect();
            if let Some(expected) = &expected {
                assert_eq!(
                    &values, expected,
                    "plan type and idle summary node must not change chart values"
                );
            } else {
                expected = Some(values);
            }
            eprintln!(
                "platform history mixed={mixed}, mode={mode}: temporary bytes={temporary_bytes}"
            );
        }
    }
    sqlx::query("DEALLOCATE bounded_platform")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn container_history_bounds_sources_and_days_in_custom_and_generic_plans() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() / 86400 - 3) * 86400;
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,state,created,updated,ports,issystem)
        SELECT gen_random_uuid(),$1,'history-'||n,'image','history-'||n,'Running',0,0,'{}',false FROM generate_series(1,1000)n")
        .bind(scope.platform_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO statisticssources(platformid,containerid) SELECT platformid,id FROM containers WHERE platformid=$1")
        .bind(scope.platform_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
        SELECT src.id,$2+n*10,1,100,5,200,n*100,n*200 FROM statisticssources src JOIN containers c ON c.id=src.containerid
        CROSS JOIN generate_series(0,25919)n WHERE src.platformid=$1 AND c.dockercontainerid IN(SELECT 'history-'||c FROM generate_series(1,12)c)")
        .bind(scope.platform_id).bind(start).execute(&pool).await.unwrap();
    sqlx::raw_sql("ANALYZE containers; ANALYZE statisticssources; ANALYZE statisticssamples; ANALYZE statisticsrollups;")
        .execute(&pool).await.unwrap();
    let id: Uuid = sqlx::query_scalar(
        "SELECT id FROM containers WHERE platformid=$1 AND dockercontainerid='history-1'",
    )
    .bind(scope.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let from = start + 86400;
    let until = from + 86400 - 1;
    fn samples_scanned(node: &serde_json::Value, day: i64) -> f64 {
        let mut rows = 0.;
        if node["Relation Name"]
            .as_str()
            .is_some_and(|name| name.starts_with("statisticssamples"))
            && node["Actual Loops"].as_f64().unwrap_or_default() > 0.
        {
            assert_eq!(node["Relation Name"], format!("statisticssamples_d{day}"));
            rows += (node["Actual Rows"].as_f64().unwrap_or_default()
                + node["Rows Removed by Filter"].as_f64().unwrap_or_default()
                + node["Rows Removed by Index Recheck"]
                    .as_f64()
                    .unwrap_or_default())
                * node["Actual Loops"].as_f64().unwrap();
        }
        if let Some(children) = node["Plans"].as_array() {
            rows += children
                .iter()
                .map(|child| samples_scanned(child, day))
                .sum::<f64>();
        }
        rows
    }
    for mode in ["force_custom_plan", "force_generic_plan"] {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SET LOCAL plan_cache_mode={mode}"
        )))
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(concat!(
            "PREPARE bounded_history(uuid[],bigint,bigint,bigint) AS ",
            include_str!(
                "../../src/persistence/postgres/platforms/statistics/container_history.sql"
            )
        ))
        .execute(&mut *tx)
        .await
        .unwrap();
        let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "EXPLAIN(ANALYZE,BUFFERS,FORMAT JSON) EXECUTE bounded_history(ARRAY['{id}']::uuid[],{from},60,{until})")))
            .fetch_one(&mut *tx).await.unwrap();
        let scanned = samples_scanned(&plan[0]["Plan"], from / 86400);
        assert!(
            (8640. ..=17280.).contains(&scanned),
            "gauge/counter passes must only read the requested source/day: {plan}"
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "EXECUTE bounded_history(ARRAY['{id}']::uuid[],{from},60,{until})"
        )))
        .fetch_all(&mut *tx)
        .await
        .unwrap();
        assert_eq!(rows.len(), 1440);
        assert!(
            rows.iter()
                .all(|row| row.get::<Uuid, _>("containerid") == id
                    && row.get::<f64, _>("cpuusage") == 1.)
        );
        sqlx::query("DEALLOCATE bounded_history")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    }
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn capture_resolves_identities_once_per_batch_and_preserves_watermarks() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    add_container(&pool, &scope, "ordinary").await;
    let managed = add_container(&pool, &scope, "managed").await;
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,createdbyactorid,name,platformid,spec,status) VALUES($1,'00000000-0000-0000-0000-000000000001','batch-owner',$2,'{}','Stopped')")
        .bind(deployment).bind(scope.platform_id).execute(&pool).await.unwrap();
    sqlx::query("UPDATE containers SET deploymentid=$1 WHERE id=$2")
        .bind(deployment)
        .bind(managed)
        .execute(&pool)
        .await
        .unwrap();
    // Count actual identity-function calls in an isolated database. The wrappers
    // delegate to the real implementation, including its identity validation.
    sqlx::raw_sql("CREATE TABLE identity_calls(kind text);
        ALTER FUNCTION statistics_source(uuid,uuid,text) RENAME TO original_statistics_source;
        CREATE FUNCTION statistics_source(p uuid,c uuid,t text) RETURNS bigint LANGUAGE plpgsql AS $$
        BEGIN INSERT INTO identity_calls VALUES('source'); RETURN original_statistics_source(p,c,t); END $$;
        ALTER FUNCTION statistics_owner(bigint,text,uuid,uuid,uuid,text,text,text,text) RENAME TO original_statistics_owner;
        CREATE FUNCTION statistics_owner(s bigint,k text,d uuid,st uuid,sv uuid,ds text,n text,r text,task text DEFAULT NULL)
        RETURNS bigint LANGUAGE plpgsql AS $$ BEGIN INSERT INTO identity_calls VALUES('owner');
        RETURN original_statistics_owner(s,k,d,st,sv,ds,n,r,task); END $$;")
        .execute(&pool).await.unwrap();
    let store = PostgresStatsBatchStore::new(pool.clone());
    let start = chrono::Utc::now().timestamp() - 180;
    for (offset, expected_sources) in [(0, 2_i64), (60, 0)] {
        let captures: Vec<_> = ["ordinary", "managed"]
            .into_iter()
            .flat_map(|name| {
                let scope = &scope;
                (0..6).map(move |n| container(scope, name, start + offset + n * 10, 1.))
            })
            .collect();
        assert_eq!(store.persist_batch(&captures).await.unwrap().persisted, 12);
        let calls: (i64,i64) = sqlx::query_as("SELECT count(*) FILTER(WHERE kind='source'),count(*) FILTER(WHERE kind='owner') FROM identity_calls")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(
            calls,
            (expected_sources, 1),
            "lookups must scale with identities, not samples"
        );
        sqlx::query("TRUNCATE identity_calls")
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE statisticssources SET compacteduntil=$1 WHERE platformid=$2")
        .bind(start + 90)
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    let captures: Vec<_> = ["ordinary", "managed"]
        .into_iter()
        .flat_map(|name| {
            let scope = &scope;
            [80, 90, 100]
                .into_iter()
                .map(move |n| container(scope, name, start + n, 99.))
        })
        .collect();
    let outcome = store.persist_batch(&captures).await.unwrap();
    assert_eq!((outcome.persisted, outcome.stale), (4, 2));
    let sealed: Vec<f64> =
        sqlx::query_scalar("SELECT cpuusage FROM statisticssamples WHERE created=$1")
            .bind(start + 80)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(sealed, vec![1., 1.]);
    let changed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM statisticssamples WHERE cpuusage=99")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(changed, 4);
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn latest_container_stats_do_not_scan_other_sources_when_redeploy_has_no_samples() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let busy = add_container(&pool, &scope, "busy").await;
    let empty = add_container(&pool, &scope, "replacement").await;
    add_container(&pool, &scope, "not-sampled").await;
    let start = chrono::Utc::now().timestamp() / 86400 * 86400;
    sqlx::query("INSERT INTO statisticssources(platformid,containerid) VALUES($1,$2),($1,$3)")
        .bind(scope.platform_id)
        .bind(busy)
        .bind(empty)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
        SELECT src.id,$2+n,1,100,5,200,0,0 FROM statisticssources src CROSS JOIN generate_series(1,80000)n WHERE src.containerid=$1")
        .bind(busy).bind(start).execute(&pool).await.unwrap();
    sqlx::raw_sql("ANALYZE statisticssources; ANALYZE statisticssamples;")
        .execute(&pool)
        .await
        .unwrap();
    // A newer compacted observation must beat the raw one. Include a dense
    // unrelated summary history so missing-source lookups bound both branches.
    sqlx::query("INSERT INTO statisticsrollups(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,
        samplecount,readcount,writecount,lastcreated,firstcounters,lastcounters,counterdeltas,counterdurations,peaks)
        SELECT src.id,$2+n*10+1,1,100,5,200,0,0,1,0,0,$2+n*10+1,
        ARRAY[0,0,0,0]::float8[],ARRAY[0,0,0,0]::float8[],ARRAY[0,0,0,0]::float8[],ARRAY[0,0,0,0]::bigint[],ARRAY[1,100,5,200]::float8[]
        FROM statisticssources src CROSS JOIN generate_series(1,8000)n WHERE src.containerid=$1")
        .bind(busy).bind(start).execute(&pool).await.unwrap();
    sqlx::raw_sql("ANALYZE statisticsrollups")
        .execute(&pool)
        .await
        .unwrap();
    let select = include_str!("../../src/persistence/postgres/platforms/queries.rs")
        .split("pub(super) const CONTAINER_SELECT: &str = r#\"")
        .nth(1)
        .unwrap()
        .split("\"#;")
        .next()
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("PREPARE latest(uuid,text[]) AS {select} WHERE container.platformid=$1 AND container.dockercontainerid=ANY($2) ORDER BY container.id"))).execute(&mut *tx).await.unwrap();
    for mode in ["force_custom_plan", "force_generic_plan"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SET LOCAL plan_cache_mode={mode}"
        )))
        .execute(&mut *tx)
        .await
        .unwrap();
        for docker_id in ["replacement", "not-sampled", "busy"] {
            let execute = format!(
                "EXECUTE latest('{}',ARRAY['{docker_id}'])",
                scope.platform_id
            );
            let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "EXPLAIN(ANALYZE,BUFFERS,TIMING OFF,FORMAT JSON) {execute}"
            )))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
            let blocks = plan[0]["Plan"]["Shared Hit Blocks"].as_i64().unwrap()
                + plan[0]["Plan"]["Shared Read Blocks"].as_i64().unwrap();
            assert!(
                blocks < 300,
                "{mode} {docker_id} touched {blocks} blocks: {plan}"
            );
            let row = sqlx::query(sqlx::AssertSqlSafe(execute))
                .fetch_one(&mut *tx)
                .await
                .unwrap();
            assert_eq!(
                row.get::<Option<i64>, _>("stat_created"),
                if docker_id == "busy" {
                    Some(start + 80001)
                } else {
                    None
                }
            );
        }
    }
    tx.rollback().await.unwrap();
    close_storage_fixture(pool, database).await;
}
