use super::*;

async fn node_scope(pool: &PgPool, parent: &StatsScope, node: &str) -> StatsScope {
    let scope = StatsScope {
        connector: "EdgeAgent".into(),
        address: None,
        node_id: Some(node.into()),
        agent_id: Some(Uuid::now_v7()),
        connected_at: Some(
            sqlx::query_scalar("SELECT now()")
                .fetch_one(pool)
                .await
                .unwrap(),
        ),
        ..parent.clone()
    };
    sqlx::query("INSERT INTO edgeagentbindings(id,agentfingerprint,agentid,agentpublickey,connectionstatus,platformid,resourceid,resourcetype,dockernodeid,lastconnectedatutc) VALUES($1,$2,$1,'fixture','Connected',$3,$3,'Platform',$4,$5)")
        .bind(scope.agent_id).bind(scope.agent_id.unwrap().to_string()).bind(scope.platform_id)
        .bind(node).bind(scope.connected_at).execute(pool).await.unwrap();
    scope
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn host_wakes_require_committed_changes_without_changing_admission_counts() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let node = node_scope(&pool, &scope, "worker").await;
    let store = PostgresStatsBatchStore::new(pool.clone());
    let channel = citadel_runtime::RuntimeSignal::PlatformStats.channel();
    let mut listener = sqlx::postgres::PgListener::connect_with(&pool)
        .await
        .unwrap();
    listener.listen(channel).await.unwrap();
    async fn no_wake(listener: &mut sqlx::postgres::PgListener) {
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(150), listener.recv())
                .await
                .is_err()
        );
    }
    async fn wake(listener: &mut sqlx::postgres::PgListener) {
        tokio::time::timeout(std::time::Duration::from_secs(2), listener.recv())
            .await
            .unwrap()
            .unwrap();
    }
    let mut sample = host(&scope, chrono::Utc::now().timestamp(), 1);
    assert_eq!(
        store
            .persist_batch(&[sample.clone()])
            .await
            .unwrap()
            .persisted,
        1
    );
    wake(&mut listener).await;
    // An unchanged replay leaves already-pending work durable without another hint.
    assert_eq!(
        store
            .persist_batch(&[sample.clone()])
            .await
            .unwrap()
            .persisted,
        1
    );
    no_wake(&mut listener).await;
    let pending: bool =
        sqlx::query_scalar("SELECT alertpending FROM platformstats WHERE platformid=$1")
            .bind(scope.platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(pending);
    sqlx::query("UPDATE platformstats SET alertpending=false WHERE platformid=$1")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    let replay = store.persist_batch(&[sample.clone()]).await.unwrap();
    assert_eq!((replay.persisted, replay.stale), (1, 0));
    no_wake(&mut listener).await;
    sample.cpu_usage += 2.;
    assert_eq!(
        store
            .persist_batch(&[sample.clone()])
            .await
            .unwrap()
            .persisted,
        1
    );
    wake(&mut listener).await;
    let pending: bool =
        sqlx::query_scalar("SELECT alertpending FROM platformstats WHERE platformid=$1")
            .bind(scope.platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(pending);
    let node_sample = host(&node, sample.created, 1);
    assert_eq!(
        store.persist_batch(&[node_sample]).await.unwrap().persisted,
        1
    );
    no_wake(&mut listener).await;
    let nodes: i64 =
        sqlx::query_scalar("SELECT count(*) FROM platformnodeiostats WHERE platformid=$1")
            .bind(scope.platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(nodes, 1);
    // Reject COMMIT after the writer has queued its notification. Neither the
    // changed tuple nor the transactional notification may become visible.
    sqlx::raw_sql("CREATE FUNCTION reject_capture_commit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture commit rejection'; END $$; CREATE CONSTRAINT TRIGGER reject_capture_commit AFTER INSERT OR UPDATE ON platformstats DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_capture_commit();")
        .execute(&pool).await.unwrap();
    sample.cpu_usage += 2.;
    assert!(store.persist_batch(&[sample.clone()]).await.is_err());
    no_wake(&mut listener).await;
    let cpu: f64 = sqlx::query_scalar("SELECT cpuusage FROM platformstats WHERE platformid=$1")
        .bind(scope.platform_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(cpu, (sample.cpu_usage - 2.) / 2.);
    drop(listener);
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn host_capture_bounds_intermediates_and_preserves_mixed_scope_admission() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    sqlx::query("UPDATE platforms SET platformdescriptor=json_build_object('$type','Docker','nodeID','manager','fixture',repeat('descriptor-',6000)) WHERE id=$1")
        .bind(scope.platform_id).execute(&pool).await.unwrap();
    let node = node_scope(&pool, &scope, "worker").await;
    let manager = node_scope(&pool, &scope, "manager").await;
    let mut obsolete = node.clone();
    obsolete.connected_at = obsolete
        .connected_at
        .map(|at| at - chrono::Duration::seconds(1));
    let mut wrong_address = scope.clone();
    wrong_address.address = Some("wrong".into());
    let start = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO platformhistorysources(platformid,nodeid,compacteduntil) VALUES($1,'',$2)",
    )
    .bind(scope.platform_id)
    .bind(start)
    .execute(&pool)
    .await
    .unwrap();
    let mut samples: Vec<_> = (0..128).map(|n| host(&scope, start + n, 1)).collect();
    // The original global ordinal decides the last same-timestamp capture.
    let mut last = host(&scope, start, 1);
    last.memory_active = 600.;
    last.metadata.as_mut().unwrap().mem_total = 2000;
    samples.extend([
        host(&node, start, 1),
        host(&obsolete, start + 1, 1),
        host(&manager, start, 1),
        host(&wrong_address, start, 1),
        host(&scope, start - 1, 1),
        last,
    ]);
    let wire = serde_json::to_value(&samples).unwrap();
    for mode in ["force_custom_plan", "force_generic_plan"] {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SET LOCAL plan_cache_mode={mode}"
        )))
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(concat!(
            "PREPARE host_capture(jsonb) AS ",
            include_str!("../../src/persistence/postgres/platforms/statistics/platform_batch.sql")
        ))
        .execute(&mut *tx)
        .await
        .unwrap();
        let execute = format!(
            "EXECUTE host_capture('{}'::jsonb)",
            wire.to_string().replace('\'', "''")
        );
        let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "EXPLAIN(ANALYZE,BUFFERS,VERBOSE,FORMAT JSON) {execute}"
        )))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(plan[0]["Plan"]["Temp Written Blocks"], 0, "{plan}");
        fn check(node: &serde_json::Value) -> bool {
            if node["Subplan Name"] == "CTE open" {
                let output = node["Output"].to_string();
                assert!(!output.contains("connected_at"), "{output}");
                assert!(!output.contains("connectortype"), "{output}");
                assert!(!output.contains("::jsonb"), "{output}");
                return true;
            }
            node["Plans"]
                .as_array()
                .is_some_and(|children| children.iter().any(check))
        }
        assert!(
            check(&plan[0]["Plan"]),
            "must inspect the materialized capture projection"
        );
        // The EXPLAIN performed the writes. Replaying remains accepted but causes
        // no physical host update, even for the duplicate timestamp.
        let result: (i64, bool) = sqlx::query_as(sqlx::AssertSqlSafe(execute))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(result, (130, false));
        let rows: (i64,i64,f64) = sqlx::query_as("SELECT (SELECT count(*) FROM platformstats),(SELECT count(*) FROM platformnodeiostats),(SELECT memoryusage FROM platformstats WHERE created=$1)")
            .bind(start).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(rows, (128, 1, 30.));
        eprintln!(
            "host capture {mode}: bytes={} elapsed_ms={} temporary_blocks={}",
            serde_json::to_vec(&wire).unwrap().len(),
            plan[0]["Execution Time"],
            plan[0]["Plan"]["Temp Written Blocks"]
        );
        sqlx::query("DEALLOCATE host_capture")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    }
    close_storage_fixture(pool, database).await;
}
