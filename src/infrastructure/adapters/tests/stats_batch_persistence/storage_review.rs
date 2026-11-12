use super::*;

async fn measurement_bytes(pool: &PgPool, raw: bool) -> i64 {
    let sql = if raw {
        "SELECT pg_total_relation_size('platformstats')+pg_total_relation_size('platformnodeiostats')+pg_total_relation_size('platformhistorysources')"
    } else {
        // pg_total_relation_size already includes each leaf's TOAST and indexes.
        // Partitioned parents have no physical heap; never add child indexes twice.
        "SELECT coalesce(sum(pg_total_relation_size(relid)),0)::bigint+pg_total_relation_size('platformhistorysources') FROM (
            SELECT relid FROM pg_partition_tree('platformgaugerollups') WHERE isleaf
            UNION ALL SELECT relid FROM pg_partition_tree('platformiorollups') WHERE isleaf) parts"
    };
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn platform_compaction_storage_gate_reduces_dense_history_and_keeps_unsuitable_minutes_raw() {
    for (name, containers, interval, churn, suitable) in [
        ("dense-small", 3_i64, 10_i64, false, true),
        ("dense-2-second", 50, 2, false, true),
        ("stable-10-second", 50, 10, false, false),
        ("15-second", 50, 15, false, false),
        ("churn", 20, 10, true, false),
    ] {
        let (pool, database) = storage_fixture().await;
        let scope = platform(&pool).await;
        let start = (chrono::Utc::now().timestamp() - 2 * 86400) / 3600 * 3600;
        let empty_raw = measurement_bytes(&pool, true).await;
        let empty_summary = measurement_bytes(&pool, false).await;
        sqlx::query(
            "INSERT INTO platformhistorysources(platformid,nodeid) VALUES($1,''),($1,'worker')",
        )
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,containerio)
            SELECT gen_random_uuid(),$1,$2+n*$3,10,20,n*100,n*200,
                (SELECT jsonb_agg(jsonb_build_object('containerId',repeat(md5((CASE WHEN $5 THEN n*$4+c ELSE c END)::text),2),
                    'rxBytes',n*100,'txBytes',n*200,'blockReadBytes',n*300,'blockWriteBytes',n*400)) FROM generate_series(1,$4)c)
            FROM generate_series(0,3600/$3)n")
            .bind(scope.platform_id).bind(start).bind(interval).bind(containers).bind(churn).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO platformnodeiostats(platformid,nodeid,created,containerio) SELECT platformid,'worker',created,containerio FROM platformstats WHERE platformid=$1")
            .bind(scope.platform_id).execute(&pool).await.unwrap();
        let raw = measurement_bytes(&pool, true).await - empty_raw;
        let before = platform_history(&pool, scope.platform_id, start, start + 3600, 300).await;
        for _ in 0..3 {
            citadel_adapters::persistence::postgres::maintenance::compact(&pool)
                .await
                .unwrap();
        }
        let summary = measurement_bytes(&pool, false).await - empty_summary;
        let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM platformstats),(SELECT count(*) FROM platformnodeiostats),(SELECT count(*) FROM platformiorollups)").fetch_one(&pool).await.unwrap();
        eprintln!(
            "native storage {name}: original heap/TOAST/index bytes={raw}, replacement bytes={summary}, rows={counts:?}"
        );
        if suitable {
            assert_eq!((counts.0, counts.1), (1, 1));
            assert!(
                summary * 10 < raw * 9,
                "dense history should materially shrink: {raw} -> {summary}"
            );
        } else {
            assert_eq!(counts, (3600 / interval + 1, 3600 / interval + 1, 0));
            let kept = measurement_bytes(&pool, true).await - empty_raw;
            assert!(
                kept * 100 <= raw * 105,
                "unsuitable raw representation must not materially expand"
            );
            assert!(
                summary < raw / 20,
                "no replacement measurements for rejected minutes"
            );
        }
        assert_history_close(
            &before,
            &platform_history(&pool, scope.platform_id, start, start + 3600, 300).await,
        );
        let watermarks: Vec<i64> =
            sqlx::query_scalar("SELECT compacteduntil FROM platformhistorysources ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            watermarks,
            vec![start + 3600; 2],
            "even rejected minutes are sealed once"
        );
        assert_eq!(
            citadel_adapters::persistence::postgres::maintenance::compact(&pool)
                .await
                .unwrap(),
            0
        );
        close_storage_fixture(pool, database).await;
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn swarm_manager_handover_reuses_a_verified_physical_task_with_both_projections_present() {
    use citadel_adapters::persistence::postgres::platforms::{
        PostgresPlatformReader, inventory::store::PostgresInventoryProjectionStore,
    };
    use citadel_platforms::{
        InventoryProjectionStore, PlatformReader, RuntimeContainerSummary,
        RuntimeInventorySnapshot, RuntimePlatformInfo, RuntimeSwarmInfo, ServiceStatIdentity,
        StatisticsReader, StatsWindow,
    };
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    sqlx::query("UPDATE platforms SET clusterid='cluster',platformdescriptor='{\"$type\":\"DockerSwarm\",\"nodeID\":\"manager-m\",\"daemonId\":\"daemon-m\"}' WHERE id=$1")
        .bind(scope.platform_id).execute(&pool).await.unwrap();
    let node = history_node(&pool, &scope, "manager-n").await;
    let old = add_container(&pool, &node, "physical-container").await;
    sqlx::query("UPDATE containers SET isswarmtask=true WHERE id=$1")
        .bind(old)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO swarmserviceprojections(platformid,dockerserviceid,configids,desiredtaskcount,image,labels,mode,name,networkids,observedat,ports,runningtaskcount,secretids,updatestate,versionindex) VALUES($1,'service','[]',1,'image','{}','replicated','service','[]',now(),'[]',1,'[]','completed',1)")
        .bind(scope.platform_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockercontainerid,dockernodeid,dockerserviceid,image,name,nodehostname,observedat,ports,servicename,slot,state,versionindex) VALUES($1,'task','running','physical-container','manager-n','service','image','task','manager-n',now(),'[]','service',1,'running',1)")
        .bind(scope.platform_id).execute(&pool).await.unwrap();
    let batch = PostgresStatsBatchStore::new(pool.clone());
    let now = chrono::Utc::now().timestamp();
    let mut before = container(&node, "physical-container", now - 20, 10.);
    before.sample.rx_bytes = 100.;
    assert_eq!(batch.persist_batch(&[before]).await.unwrap().persisted, 1);
    let source: i64 = sqlx::query_scalar("SELECT id FROM statisticssources WHERE containerid=$1")
        .bind(old)
        .fetch_one(&pool)
        .await
        .unwrap();
    let current = PostgresPlatformReader::new(pool.clone())
        .get_platform(scope.platform_id)
        .await
        .unwrap()
        .unwrap();
    let input = citadel_platforms::management::patch_input(
        &current,
        &serde_json::json!({"connectorType":"Agent","address":"https://manager-n:5001"}),
    )
    .unwrap();
    let info = RuntimePlatformInfo {
        daemon_id: "daemon-n".into(),
        server_version: "29".into(),
        operating_system: "Linux".into(),
        os_type: "linux".into(),
        architecture: "x86_64".into(),
        cpu_count: 2,
        memory_total: 1000,
        container_count: 1,
        containers_running: 1,
        containers_paused: 0,
        containers_stopped: 0,
        api_version: "1.49".into(),
        minimum_api_version: "1.24".into(),
        agent_version: None,
        swarm: Some(RuntimeSwarmInfo {
            cluster_id: Some("cluster".into()),
            node_id: "manager-n".into(),
            control_available: true,
            local_node_state: "active".into(),
            ..Default::default()
        }),
    };
    citadel_platforms::management::validate_target(&current, &info).unwrap();
    citadel_adapters::persistence::postgres::platforms::management::update(
        &pool,
        &current,
        &input,
        Some(&info),
        citadel_primitives::ActorId::new(Uuid::from_u128(1)),
        false,
    )
    .await
    .unwrap();
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&RuntimeInventorySnapshot {
            platform_id: scope.platform_id,
            info: info.clone(),
            containers: vec![RuntimeContainerSummary {
                id: "physical-container".into(),
                name: "task".into(),
                image: "image".into(),
                image_id: "image".into(),
                state: "running".into(),
                is_swarm_task: true,
                ports: serde_json::json!([]),
                ..Default::default()
            }],
            images: vec![],
            networks: vec![],
            volumes: vec![],
            swarm: Some(citadel_platforms::RuntimeSwarmInventory {
                services: vec![citadel_platforms::RuntimeSwarmService {
                    id: "service".into(),
                    name: "service".into(),
                    image: "image".into(),
                    mode: "replicated".into(),
                    version_index: 1,
                    ..Default::default()
                }],
                tasks: vec![citadel_platforms::RuntimeSwarmTask {
                    id: "task".into(),
                    node_id: "manager-n".into(),
                    container_id: Some("physical-container".into()),
                    service_id: "service".into(),
                    slot: Some(1),
                    desired_state: "running".into(),
                    state: "running".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            observed_at: chrono::Utc::now(),
        })
        .await
        .unwrap();
    let projections:Vec<(Uuid,Option<String>)>=sqlx::query_as("SELECT id,dockernodeid FROM containers WHERE platformid=$1 AND dockercontainerid='physical-container' ORDER BY dockernodeid NULLS FIRST")
        .bind(scope.platform_id).fetch_all(&pool).await.unwrap();
    assert_eq!(
        projections.len(),
        2,
        "host inventory must coexist with the node projection"
    );
    let new = projections[0].0;
    assert_ne!(new, old);
    let mut host_scope = scope.clone();
    host_scope.connector = "Agent".into();
    host_scope.address = input.address.clone();
    let mut after = container(&host_scope, "physical-container", now - 10, 10.);
    after.sample.rx_bytes = 1100.;
    assert_eq!(
        batch
            .persist_batch(&[after.clone()])
            .await
            .unwrap()
            .persisted,
        1
    );
    assert_eq!(batch.persist_batch(&[after]).await.unwrap().persisted, 1);
    let association: (i64, Uuid) = sqlx::query_as(
        "SELECT id,containerid FROM statisticssources WHERE dockertaskid='task' AND platformid=$1",
    )
    .bind(scope.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(association, (source, new));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM statisticssamples WHERE sourceid=$1")
        .bind(source)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, 2,
        "handover and retries must not duplicate physical measurements"
    );
    let reader=citadel_adapters::persistence::postgres::platforms::statistics::reader::PostgresStatisticsReader::new(pool.clone());
    let stats = reader
        .service(
            ServiceStatIdentity {
                platform_id: scope.platform_id,
                docker_service_id: "service",
                managed_service_id: None,
                stack_id: None,
                service_name: "service",
            },
            StatsWindow::between(now - 60, now, now).unwrap(),
            now,
        )
        .await
        .unwrap();
    assert_eq!(
        stats.iter().filter_map(|s| s.rx_bytes_delta).sum::<f64>(),
        1000.
    );
    // Both projections can be captured in one flush. Deduplicating identity
    // lookups must still leave the last observed projection attached to the Task.
    let mut overlapping = Vec::new();
    for (capture_scope, age, rx) in [
        (&host_scope, 9, 1190.),
        (&node, 8, 1200.),
        (&host_scope, 7, 1250.),
    ] {
        let mut sample = container(capture_scope, "physical-container", now - age, 10.);
        sample.sample.rx_bytes = rx;
        overlapping.push(sample);
    }
    assert_eq!(
        batch.persist_batch(&overlapping).await.unwrap().persisted,
        3
    );
    let associated: Uuid =
        sqlx::query_scalar("SELECT containerid FROM statisticssources WHERE id=$1")
            .bind(source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(associated, new);
    // Move the connection back before the host snapshot has removed C2. The
    // previous NULL-node host projection belongs to Task T on N, despite the
    // Platform descriptor now pointing to M. Its node capture must still work.
    let current_back = PostgresPlatformReader::new(pool.clone())
        .get_platform(scope.platform_id)
        .await
        .unwrap()
        .unwrap();
    let back_input = citadel_platforms::management::patch_input(
        &current_back,
        &serde_json::json!({"address":"https://manager-m:5001"}),
    )
    .unwrap();
    let mut old_info = info.clone();
    old_info.daemon_id = "daemon-m".into();
    old_info.swarm.as_mut().unwrap().node_id = "manager-m".into();
    citadel_platforms::management::validate_target(&current_back, &old_info).unwrap();
    citadel_adapters::persistence::postgres::platforms::management::update(
        &pool,
        &current_back,
        &back_input,
        Some(&old_info),
        citadel_primitives::ActorId::new(Uuid::from_u128(1)),
        false,
    )
    .await
    .unwrap();
    let mut returned = container(&node, "physical-container", now - 5, 10.);
    returned.sample.rx_bytes = 1300.;
    assert_eq!(batch.persist_batch(&[returned]).await.unwrap().persisted, 1);
    let associated: Uuid =
        sqlx::query_scalar("SELECT containerid FROM statisticssources WHERE id=$1")
            .bind(source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(associated, old);
    let stats = reader
        .service(
            ServiceStatIdentity {
                platform_id: scope.platform_id,
                docker_service_id: "service",
                managed_service_id: None,
                stack_id: None,
                service_name: "service",
            },
            StatsWindow::between(now - 60, now, now).unwrap(),
            now,
        )
        .await
        .unwrap();
    assert_eq!(
        stats.iter().filter_map(|s| s.rx_bytes_delta).sum::<f64>(),
        1200.
    );
    for (docker, node) in [
        ("unrelated", "manager-n"),
        ("physical-container", "other-node"),
    ] {
        let mut invalid_scope = scope.clone();
        invalid_scope.node_id = Some(node.into());
        let unrelated = add_container(&pool, &invalid_scope, docker).await;
        let error = sqlx::query_scalar::<_, i64>("SELECT statistics_source($1,$2,'task')")
            .bind(scope.platform_id)
            .bind(unrelated)
            .fetch_one(&pool)
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("P0001")
        );
    }
    let other = platform(&pool).await;
    let foreign = add_container(&pool, &other, "physical-container").await;
    assert!(
        sqlx::query_scalar::<_, i64>("SELECT statistics_source($1,$2,'task')")
            .bind(scope.platform_id)
            .bind(foreign)
            .fetch_one(&pool)
            .await
            .is_err(),
        "a container from another platform cannot take over the source"
    );
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn history_readers_use_one_resolution_for_staggered_replicas_and_mixed_windows() {
    use citadel_platforms::{
        ServiceStatIdentity, StatisticsReader, StatisticsWorkload, StatsWindow,
    };
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let stack = Uuid::now_v7();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,'history-resolution','00000000-0000-0000-0000-000000000001','WebEditor','{}','{}')")
        .bind(stack).execute(&pool).await.unwrap();
    let now = chrono::Utc::now().timestamp();
    let start = (now - 26 * 3600) / 60 * 60;
    let recent = (now - 600) / 60 * 60;
    let mut ids = vec![];
    for kind in ["Workload", "Service"] {
        for replica in 0..2_i64 {
            let id = add_container(&pool, &scope, &format!("{kind}-{replica}")).await;
            if kind == "Workload" {
                ids.push(id);
            }
            let source: i64 = sqlx::query_scalar("SELECT statistics_source($1,$2,$3)")
                .bind(scope.platform_id)
                .bind(id)
                .bind((kind == "Service").then(|| format!("task-{replica}")))
                .fetch_one(&pool)
                .await
                .unwrap();
            let owner: i64 =
                sqlx::query_scalar("SELECT statistics_owner($1,$2,NULL,$3,NULL,$4,'web',$5,$6)")
                    .bind(source)
                    .bind(kind)
                    .bind((kind == "Workload").then_some(stack))
                    .bind((kind == "Service").then_some("service"))
                    .bind(format!("replica:{replica}"))
                    .bind((kind == "Service").then(|| format!("task-{replica}")))
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            for epoch in [start, start + 3 * 3600, recent] {
                sqlx::query("INSERT INTO statisticssamples(sourceid,ownerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
                    SELECT $1,$2,$3+n*10,10,100,10,200,n*1000,n*2000,NULL,n*3000 FROM generate_series(0,11)n")
                    .bind(source).bind(owner).bind(epoch+replica*15).execute(&pool).await.unwrap();
            }
        }
    }
    let host_source: i64 = sqlx::query_scalar(
        "INSERT INTO platformhistorysources(platformid,nodeid) VALUES($1,'') RETURNING id",
    )
    .bind(scope.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let node_source: i64 = sqlx::query_scalar(
        "INSERT INTO platformhistorysources(platformid,nodeid) VALUES($1,'worker') RETURNING id",
    )
    .bind(scope.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    for epoch in [start, start + 3 * 3600, recent] {
        sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,containerio)
            SELECT gen_random_uuid(),$1,$2+n*10,10,20,n*1000,n*2000,jsonb_build_array(jsonb_build_object('containerId','same','rxBytes',n*1000,'txBytes',n*2000)) FROM generate_series(0,11)n")
            .bind(scope.platform_id).bind(epoch).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO platformnodeiostats(platformid,nodeid,created,containerio)
            SELECT $1,'worker',$2+15+n*10,jsonb_build_array(jsonb_build_object('containerId','same','rxBytes',n*1000,'txBytes',n*2000)) FROM generate_series(0,11)n")
            .bind(scope.platform_id).bind(epoch).execute(&pool).await.unwrap();
    }
    let reader=citadel_adapters::persistence::postgres::platforms::statistics::reader::PostgresStatisticsReader::new(pool.clone());
    let windows = [
        StatsWindow::between(start, start + 3600, now).unwrap(),
        StatsWindow::between(start, start + 4 * 3600, now).unwrap(),
    ];
    let service = || ServiceStatIdentity {
        platform_id: scope.platform_id,
        docker_service_id: "service",
        managed_service_id: None,
        stack_id: None,
        service_name: "web",
    };
    let mut expected = vec![];
    for window in windows {
        expected.push((
            reader.containers(&ids, window, now).await.unwrap(),
            reader
                .workload_history(StatisticsWorkload::Stack, stack, window, now)
                .await
                .unwrap()
                .unwrap()
                .remove(0)
                .stats,
            reader.service(service(), window, now).await.unwrap(),
            reader
                .platform(scope.platform_id, window, now)
                .await
                .unwrap(),
        ));
    }
    assert_eq!(windows[0].bucket_seconds(), 10);
    assert_eq!(windows[1].bucket_seconds(), 30);
    assert_eq!(expected[0].1[0].cpu_usage, 20.);
    assert_eq!(expected[0].1[0].rx_bytes_per_second, Some(200.));
    assert_eq!(expected[0].3[0].rx_bytes_per_second, Some(200.));
    let recent_window = StatsWindow::between(recent, recent + 180, now).unwrap();
    let recent_before = reader.containers(&ids, recent_window, now).await.unwrap();
    assert!(
        recent_before.len() > 6,
        "recent raw-only history keeps sub-minute observations"
    );
    for _ in 0..8 {
        citadel_adapters::persistence::postgres::maintenance::compact(&pool)
            .await
            .unwrap();
    }
    // Directly assert both host and node were summarized, not just a raw-read equality.
    for source in [host_source, node_source] {
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM platformiorollups WHERE sourceid=$1")
                .bind(source)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(count > 0);
    }
    for (window, (containers, workload, tasks, host)) in windows.into_iter().zip(expected) {
        assert_eq!(
            reader.containers(&ids, window, now).await.unwrap(),
            containers
        );
        assert_eq!(
            reader
                .workload_history(StatisticsWorkload::Stack, stack, window, now)
                .await
                .unwrap()
                .unwrap()
                .remove(0)
                .stats,
            workload
        );
        assert_eq!(reader.service(service(), window, now).await.unwrap(), tasks);
        assert_eq!(
            reader
                .platform(scope.platform_id, window, now)
                .await
                .unwrap(),
            host
        );
    }
    assert_eq!(
        reader.containers(&ids, recent_window, now).await.unwrap(),
        recent_before
    );
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn native_compaction_makes_fair_progress_across_1024_container_sources() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() - 2 * 86400) / 3600 * 3600;
    sqlx::query("INSERT INTO statisticssources(platformid,dockertaskid) SELECT $1,'scale-'||n FROM generate_series(1,1024)n")
        .bind(scope.platform_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
        SELECT id,$2+n*10,10,100,10,200,n*1000,n*2000 FROM statisticssources CROSS JOIN generate_series(0,360)n WHERE platformid=$1")
        .bind(scope.platform_id).bind(start).execute(&pool).await.unwrap();
    sqlx::raw_sql("ANALYZE statisticssamples; ANALYZE statisticssources")
        .execute(&pool)
        .await
        .unwrap();
    let began = std::time::Instant::now();
    for _ in 0..1024 {
        assert!(
            citadel_adapters::persistence::postgres::maintenance::compact(&pool)
                .await
                .unwrap()
                > 0
        );
    }
    let seconds = began.elapsed().as_secs_f64();
    let sealed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM statisticssources WHERE platformid=$1 AND compacteduntil=$2",
    )
    .bind(scope.platform_id)
    .bind(start + 3600)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        sealed, 1024,
        "oldest-first progress must reach every source"
    );
    let rows:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM statisticssamples),(SELECT count(*) FROM statisticsrollups),(SELECT sum(samplecount)::bigint FROM statisticsobservations)")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(rows, (1024, 1024 * 60, 1024 * 361));
    eprintln!(
        "native 1024-source compaction: 1024 source-hours / 369664 raw samples in {seconds:.3}s; counts={rows:?}"
    );
    close_storage_fixture(pool, database).await;
}
