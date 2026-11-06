use citadel_adapters::persistence::postgres::platforms::statistics::batch::PostgresStatsBatchStore;
use citadel_platforms::{RuntimeContainerStat, RuntimePlatformStats, stats_ingestion::*};
use sqlx::{PgPool, Row};
use uuid::Uuid;

async fn fixture() -> PgPool {
    let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    PgPool::connect(&url).await.unwrap()
}
async fn platform(pool: &PgPool) -> StatsScope {
    let id = Uuid::now_v7();
    let address = id.to_string();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$2,$2,'Local',2,0,1000,0,0,'{\"$type\":\"Docker\"}','Online')")
        .bind(id).bind(&address).execute(pool).await.unwrap();
    StatsScope {
        platform_id: id,
        node_id: None,
        connector: "Local".into(),
        address: Some(address),
        agent_id: None,
        connected_at: None,
        closed: None,
    }
}
fn container(scope: &StatsScope, id: &str, created: i64, cpu: f64) -> ContainerStatsSample {
    ContainerStatsSample {
        scope: scope.clone(),
        sample: RuntimeContainerStat {
            docker_container_id: id.into(),
            created,
            memory_active: 100.0,
            memory_cache: 5.0,
            cpu_usage: cpu,
            memory_limit: 200.0,
            rx_bytes: 3.0,
            tx_bytes: 4.0,
            block_read_bytes: None,
            block_write_bytes: None,
        },
    }
}
async fn add_container(pool: &PgPool, scope: &StatsScope, docker_id: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,state,created,updated,ports,issystem,dockernodeid) VALUES($1,$2,$3,'image',$3,'Running',0,0,'{}',false,$4)")
        .bind(id).bind(scope.platform_id).bind(docker_id).bind(&scope.node_id).execute(pool).await.unwrap();
    id
}
fn host(scope: &StatsScope, created: i64, count: i64) -> PlatformStatsSample {
    PlatformStatsSample {
        container_io: Vec::new(),
        scope: scope.clone(),
        created,
        memory_active: 100.0,
        cpu_usage: 20.0,
        rx_bytes: 3.0,
        tx_bytes: 4.0,
        metadata: Some(RuntimePlatformStats {
            mem_total: 1000,
            image_count: count,
            disk_used_bytes: Some(50),
            disk_total_bytes: Some(100),
            disk_usage: Some(50.0),
            ..Default::default()
        }),
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn cross_platform_batches_filter_stale_identity_and_deduplicate_retry_keys() {
    let pool = fixture().await;
    let a = platform(&pool).await;
    let b = platform(&pool).await;
    let a_id = add_container(&pool, &a, "same-id").await;
    add_container(&pool, &b, "same-id").await;
    let store = PostgresStatsBatchStore::new(pool.clone());
    let mut stale = a.clone();
    stale.node_id = Some("wrong-node".into());
    let mut wrong_address = a.clone();
    wrong_address.address = Some("old-daemon".into());
    let mut closed = a.clone();
    let token = tokio_util::sync::CancellationToken::new();
    token.cancel();
    closed.closed = Some(token);
    let samples = vec![
        container(&a, "same-id", 10, 1.0),
        container(&b, "same-id", 10, 2.0),
        container(&a, "same-id", 10, 9.0),
        container(&stale, "same-id", 10, 8.0),
        container(&a, "deleted", 10, 8.0),
        container(&wrong_address, "same-id", 10, 8.0),
        container(&closed, "same-id", 10, 8.0),
    ];
    let result = store.persist_batch(&samples).await.unwrap();
    assert_eq!((result.persisted, result.stale), (3, 4));
    store.persist_batch(&samples).await.unwrap();
    let rows: Vec<(Uuid,f64)> = sqlx::query_as("SELECT containerid,cpuusage FROM containerstats WHERE containerid IN (SELECT id FROM containers WHERE platformid=ANY($1))")
        .bind(vec![a.platform_id,b.platform_id]).fetch_all(&pool).await.unwrap();
    assert_eq!(rows.len(), 2);
    let transactions: i64 = sqlx::query_scalar("SELECT count(DISTINCT xmin::text) FROM containerstats WHERE containerid IN (SELECT id FROM containers WHERE platformid=ANY($1))")
        .bind(vec![a.platform_id,b.platform_id]).fetch_one(&pool).await.unwrap();
    assert_eq!(
        transactions, 1,
        "one cross-platform flush uses one transaction"
    );
    assert_eq!(rows.iter().find(|r| r.0 == a_id).unwrap().1, 9.0);
    // A configuration change between capture and flush fences the whole scope.
    sqlx::query("UPDATE platforms SET address='replacement' WHERE id=$1")
        .bind(b.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    let result = store
        .persist_batch(&[container(&b, "same-id", 11, 2.0)])
        .await
        .unwrap();
    assert_eq!((result.persisted, result.stale), (0, 1));
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn locked_platform_times_out_retryably_and_healthy_partition_can_commit() {
    let pool = fixture().await;
    let a = platform(&pool).await;
    let b = platform(&pool).await;
    add_container(&pool, &a, "locked").await;
    add_container(&pool, &b, "healthy").await;
    let store = PostgresStatsBatchStore::new(pool.clone());
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(a.platform_id)
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    let samples = [
        container(&a, "locked", 10, 1.0),
        container(&b, "healthy", 10, 2.0),
    ];
    let failure = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        store.persist_batch(&samples),
    )
    .await
    .expect("database lock waits must be bounded")
    .unwrap_err();
    assert!(failure.retryable);
    // The writer splits a failed mixed batch; exercise the resulting store calls
    // while A's original transaction still owns the lock.
    assert_eq!(
        store.persist_batch(&samples[1..]).await.unwrap().persisted,
        1
    );
    lock.rollback().await.unwrap();
    assert_eq!(
        store.persist_batch(&samples[..1]).await.unwrap().persisted,
        1
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM containerstats WHERE containerid IN (SELECT id FROM containers WHERE platformid=ANY($1))")
        .bind(vec![a.platform_id, b.platform_id]).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 2);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn platform_batch_updates_latest_metadata_preserves_missing_disk_and_retry_alert_revision() {
    let pool = fixture().await;
    let a = platform(&pool).await;
    let b = platform(&pool).await;
    let store = PostgresStatsBatchStore::new(pool.clone());
    let mut invalid_disk = host(&a, 11, 7);
    invalid_disk.metadata.as_mut().unwrap().disk_used_bytes = Some(-1);
    let samples = vec![invalid_disk.clone(), host(&a, 10, 2), host(&b, 10, 3)];
    assert_eq!(store.persist_batch(&samples).await.unwrap().persisted, 3);
    let row = sqlx::query("SELECT p.imagecount,s.memoryusage,s.cpuusage,s.diskusage,s.alertpending FROM platforms p JOIN platformstats s ON s.platformid=p.id WHERE p.id=$1 AND s.created=11")
        .bind(a.platform_id).fetch_one(&pool).await.unwrap();
    assert_eq!(row.get::<i32, _>("imagecount"), 7);
    assert_eq!(row.get::<f64, _>("memoryusage"), 10.0);
    assert_eq!(row.get::<f64, _>("cpuusage"), 10.0);
    assert_eq!(row.get::<Option<f64>, _>("diskusage"), None);
    assert!(row.get::<bool, _>("alertpending"));
    sqlx::query("UPDATE platformstats SET alertpending=false WHERE platformid=$1")
        .bind(a.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    store.persist_batch(&samples).await.unwrap();
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM platformstats WHERE platformid=$1 AND alertpending",
    )
    .bind(a.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        pending, 0,
        "an acknowledged duplicate must not re-arm an already evaluated alert"
    );
    store.persist_batch(&[host(&a, 9, 1)]).await.unwrap();
    let count: i32 = sqlx::query_scalar("SELECT imagecount FROM platforms WHERE id=$1")
        .bind(a.platform_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, 7,
        "older buffered summaries must not overwrite newer metadata"
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn edge_node_samples_are_fenced_by_binding_session_and_never_write_manager_totals() {
    let pool = fixture().await;
    let mut node = platform(&pool).await;
    node.connector = "EdgeAgent".into();
    node.address = None;
    node.node_id = Some("worker".into());
    node.agent_id = Some(Uuid::now_v7());
    let connected: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT now()")
        .fetch_one(&pool)
        .await
        .unwrap();
    node.connected_at = Some(connected);
    add_container(&pool, &node, "task").await;
    sqlx::query("INSERT INTO edgeagentbindings(id,agentfingerprint,agentid,agentpublickey,connectionstatus,platformid,resourceid,resourcetype,dockernodeid,lastconnectedatutc) VALUES($1,$2,$1,'fixture','Connected',$3,$3,'Platform','worker',$4)")
        .bind(node.agent_id).bind(node.agent_id.unwrap().to_string()).bind(node.platform_id).bind(connected).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO swarmserviceprojections(platformid,dockerserviceid,configids,desiredtaskcount,image,labels,mode,name,networkids,observedat,ports,runningtaskcount,secretids,updatestate,versionindex) VALUES($1,'service','[]',1,'image','{}','replicated','service','[]',now(),'[]',1,'[]','completed',1)")
        .bind(node.platform_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockercontainerid,dockernodeid,dockerserviceid,image,name,nodehostname,observedat,ports,servicename,slot,state,versionindex) VALUES($1,'task-id','running','task','worker','service','image','task','worker',now(),'[]','service',2,'running',1)")
        .bind(node.platform_id).execute(&pool).await.unwrap();
    let store = PostgresStatsBatchStore::new(pool.clone());
    assert_eq!(
        store
            .persist_batch(&[container(&node, "task", 10, 1.0)])
            .await
            .unwrap()
            .persisted,
        1
    );
    assert_eq!(
        store
            .persist_batch(&[host(&node, 10, 1)])
            .await
            .unwrap()
            .persisted,
        1
    );
    let key: String = sqlx::query_scalar(
        "SELECT taskkey FROM swarmservicestats WHERE platformid=$1 AND dockertaskid='task-id'",
    )
    .bind(node.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(key, "slot:2");
    sqlx::query("UPDATE edgeagentbindings SET lastconnectedatutc=lastconnectedatutc+interval '1 second' WHERE agentid=$1").bind(node.agent_id).execute(&pool).await.unwrap();
    assert_eq!(
        store
            .persist_batch(&[container(&node, "task", 11, 1.0)])
            .await
            .unwrap()
            .stale,
        1
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn network_history_counts_raw_increases_before_bucketing_and_keeps_instances_separate() {
    use citadel_adapters::persistence::postgres::platforms::statistics::reader::PostgresStatisticsReader;
    use citadel_platforms::{ServiceStatIdentity, StatisticsReader, StatsWindow};
    let pool = fixture().await;
    let scope = platform(&pool).await;
    let first = add_container(&pool, &scope, "network-first").await;
    let second = add_container(&pool, &scope, "network-second").await;
    let reader = PostgresStatisticsReader::new(pool.clone());
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$2,$3,'{}','Stopped',$4)")
        .bind(deployment).bind(deployment.to_string()).bind(scope.platform_id).bind(citadel_identity::SYSTEM_ACTOR_ID)
        .execute(&pool).await.unwrap();
    // A reset and recovery happen inside one 10-second bucket. The first
    // lifetime reading and samples outside the requested range must not count.
    for (id, samples) in [
        (
            first,
            vec![
                (80, 1., 1.),
                (100, 1000., 2000.),
                (102, 1100., 2040.),
                (104, 5., 2060.),
                (108, 25., 3.),
                (120, 100., 30.),
            ],
        ),
        (second, vec![(100, 5000., 10000.), (105, 5100., 10100.)]),
    ] {
        for (created, rx, tx) in samples {
            sqlx::query("INSERT INTO containerstats(id,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes) VALUES($1,$2,$3,10,100,20,200,$4,$5,$4*2,$5*2)")
                .bind(Uuid::now_v7()).bind(id).bind(created as i64).bind(rx).bind(tx).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO workloadstats(id,platformid,deploymentid,servicename,replicakey,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes) VALUES($1,$2,$3,'','deployment',$4,$5,10,100,20,200,$6,$7,$6*2,$7*2)")
                .bind(Uuid::now_v7()).bind(scope.platform_id).bind(deployment).bind(id).bind(created as i64).bind(rx).bind(tx).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO swarmservicestats(id,platformid,dockerserviceid,dockertaskid,servicename,taskkey,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes) VALUES($1,$2,'network-service',$3,'network-service','slot:1',$4,10,100,20,200,$5,$6,$5*2,$6*2)")
                .bind(Uuid::now_v7()).bind(scope.platform_id).bind(id.to_string()).bind(created as i64).bind(rx).bind(tx).execute(&pool).await.unwrap();
        }
    }
    let window = StatsWindow::between(100, 110, 120).unwrap();
    let history = reader
        .containers(&[first, second], window, 120)
        .await
        .unwrap();
    assert_eq!(history.len(), 2);
    let a = history.iter().find(|s| s.container_id == first).unwrap();
    assert_eq!(
        (a.rx_bytes_delta, a.tx_bytes_delta),
        (Some(125.), Some(63.))
    );
    assert_eq!(a.cpu_usage, 10.);
    assert_eq!(
        (a.block_read_bytes_delta, a.block_write_bytes_delta),
        (Some(250.), Some(126.))
    );
    assert!((a.rx_bytes_per_second.unwrap() - 125. / 8.).abs() < 0.000001);
    assert_eq!(a.tx_bytes_per_second, Some(63. / 8.));
    assert_eq!(a.block_write_bytes_per_second, Some(126. / 8.));
    let b = history.iter().find(|s| s.container_id == second).unwrap();
    assert_eq!(
        (b.rx_bytes_delta, b.tx_bytes_delta),
        (Some(100.), Some(100.))
    );

    // Different physical tasks in one logical slot must never share a baseline.
    let service = reader
        .service(
            ServiceStatIdentity {
                platform_id: scope.platform_id,
                docker_service_id: "network-service",
                managed_service_id: None,
                stack_id: None,
                service_name: "network-service",
            },
            window,
            120,
        )
        .await
        .unwrap();
    assert_eq!(service.len(), 1);
    assert_eq!(service[0].block_read_bytes_delta, Some(450.));
    assert_eq!(service[0].block_write_bytes_per_second, Some(326. / 13.));
    assert_eq!(
        (service[0].rx_bytes_delta, service[0].tx_bytes_delta),
        (Some(225.), Some(163.))
    );

    let workload = reader
        .workload_history(
            citadel_platforms::StatisticsWorkload::Deployment,
            deployment,
            window,
            120,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(workload.len(), 1);
    assert_eq!(workload[0].stats[0].block_read_bytes_delta, Some(450.));
    assert_eq!(
        (
            workload[0].stats[0].rx_bytes_delta,
            workload[0].stats[0].tx_bytes_delta
        ),
        (Some(225.), Some(163.))
    );

    let short = reader
        .containers(&[first], StatsWindow::between(104, 110, 120).unwrap(), 120)
        .await
        .unwrap();
    assert_eq!(
        (short[0].rx_bytes_delta, short[0].tx_bytes_delta),
        (Some(20.), Some(3.))
    );
    let single = reader
        .containers(&[first], StatsWindow::between(108, 110, 120).unwrap(), 120)
        .await
        .unwrap();
    assert_eq!(
        (single[0].rx_bytes_delta, single[0].tx_bytes_delta),
        (None, None)
    );
    assert_eq!(single[0].block_read_bytes_delta, None);
    assert_eq!(single[0].rx_bytes_per_second, None);
    assert_eq!(single[0].block_read_bytes_per_second, None);
    assert!(
        reader
            .containers(&[first], StatsWindow::between(90, 99, 120).unwrap(), 120)
            .await
            .unwrap()
            .is_empty()
    );
    // Every supported relative window returns traffic, not the lifetime counter;
    // a reset remains visible even at the coarsest resolution.
    for minutes in StatsWindow::MINUTES {
        let rows = reader
            .containers(&[first], StatsWindow::minutes(minutes).unwrap(), 120)
            .await
            .unwrap();
        assert_eq!(
            rows.iter().filter_map(|s| s.rx_bytes_delta).sum::<f64>(),
            1199.
        );
    }
    sqlx::query("DELETE FROM deployments WHERE id=$1")
        .bind(deployment)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn block_io_persists_through_batch_and_legacy_writes_and_missing_samples_are_not_zero() {
    use citadel_adapters::persistence::postgres::platforms::statistics::{
        reader::PostgresStatisticsReader, store::PostgresContainerStatsStore,
    };
    use citadel_platforms::{ContainerStatsStore, StatisticsReader, StatsWindow};
    let pool = fixture().await;
    let scope = platform(&pool).await;
    let id = add_container(&pool, &scope, "io-persistence").await;
    let batch = PostgresStatsBatchStore::new(pool.clone());
    let legacy = PostgresContainerStatsStore::new(pool.clone());
    let reader = PostgresStatisticsReader::new(pool.clone());
    for (created, bytes) in [
        (100, None),
        (110, Some(1000.)),
        (120, Some(1020.)),
        (130, Some(5.)),
    ] {
        let mut sample = container(&scope, "io-persistence", created, 1.);
        sample.sample.block_read_bytes = bytes;
        sample.sample.block_write_bytes = bytes;
        if created == 130 {
            legacy
                .persist(scope.platform_id, &[sample.sample])
                .await
                .unwrap();
        } else {
            batch
                .persist_batch(&[sample.clone(), sample])
                .await
                .unwrap();
        }
    }
    let rows = reader
        .containers(&[id], StatsWindow::between(100, 140, 140).unwrap(), 140)
        .await
        .unwrap();
    assert_eq!(rows[0].block_read_bytes, None);
    assert_eq!(rows[1].block_read_bytes_delta, None);
    assert_eq!(rows[2].block_read_bytes_delta, Some(20.));
    assert_eq!(rows[2].block_read_bytes_per_second, Some(2.));
    assert_eq!(rows[3].block_read_bytes_delta, Some(5.));
    assert_eq!(rows[3].block_write_bytes_per_second, Some(0.5));
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn throughput_weights_elapsed_time_and_excludes_intervals_without_a_counter_baseline() {
    use citadel_adapters::persistence::postgres::platforms::statistics::reader::PostgresStatisticsReader;
    use citadel_platforms::{StatisticsReader, StatsWindow};
    let pool = fixture().await;
    let scope = platform(&pool).await;
    let id = add_container(&pool, &scope, "weighted-throughput").await;
    let batch = PostgresStatsBatchStore::new(pool.clone());
    for (created, network, disk) in [
        (301, 1000., None),
        (302, 2000., Some(1000.)),
        (322, 2000., Some(1200.)),
    ] {
        let mut sample = container(&scope, "weighted-throughput", created, 1.);
        sample.sample.rx_bytes = network;
        sample.sample.tx_bytes = network;
        sample.sample.block_read_bytes = disk;
        sample.sample.block_write_bytes = disk;
        batch.persist_batch(&[sample]).await.unwrap();
    }
    let rows = PostgresStatisticsReader::new(pool.clone())
        .containers(&[id], StatsWindow::between(300, 4000, 4000).unwrap(), 4000)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].rx_bytes_delta, Some(1000.));
    assert_eq!(rows[0].rx_bytes_per_second, Some(1000. / 21.));
    assert_eq!(rows[0].tx_bytes_per_second, Some(1000. / 21.));
    assert_eq!(rows[0].block_read_bytes_per_second, Some(10.));
    assert_eq!(rows[0].block_write_bytes_per_second, Some(10.));
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn platform_io_history_preserves_physical_baselines_and_survives_container_deletion() {
    use citadel_adapters::persistence::postgres::platforms::statistics::reader::PostgresStatisticsReader;
    use citadel_platforms::{ContainerIoSnapshot, StatisticsReader, StatsWindow};
    let pool = fixture().await;
    let scope = platform(&pool).await;
    let inventory = add_container(&pool, &scope, "a").await;
    let store = PostgresStatsBatchStore::new(pool.clone());
    let io = |id: &str, value: f64, disk: Option<f64>| ContainerIoSnapshot {
        node_id: None,
        created: None,
        container_id: id.into(),
        rx_bytes: value,
        tx_bytes: value,
        block_read_bytes: disk,
        block_write_bytes: disk,
    };
    // Within one 10-second bucket: A resets while B's growing counter hides the
    // decrease in the host total. C starts with a high lifetime counter.
    let mut samples = Vec::new();
    for (created, counters) in [
        (3001, vec![io("a", 1000., Some(100.)), io("b", 100., None)]),
        (3003, vec![io("a", 20., Some(10.)), io("b", 2000., None)]),
        (
            3009,
            vec![io("b", 2060., Some(500.)), io("c", 9000., Some(900.))],
        ),
        (
            3011,
            vec![io("b", 2080., Some(520.)), io("c", 9040., Some(940.))],
        ),
    ] {
        let mut sample = host(&scope, created, 1);
        sample.container_io = counters;
        samples.push(sample);
    }
    store.persist_batch(&samples).await.unwrap();
    store.persist_batch(&samples).await.unwrap(); // A retried sample must not count twice.
    use citadel_platforms::PlatformReader;
    let current = citadel_adapters::persistence::postgres::platforms::PostgresPlatformReader::new(
        pool.clone(),
    )
    .get_platform(scope.platform_id)
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        current.stats.as_ref().unwrap()[0].container_io.as_ref(),
        Some(&samples[3].container_io)
    );
    let reader = PostgresStatisticsReader::new(pool.clone());
    let window = StatsWindow::between(3000, 6600, 6600).unwrap(); // 10-second buckets
    let stats = reader
        .platform(scope.platform_id, window, 6600)
        .await
        .unwrap();
    assert_eq!(stats.len(), 2);
    assert_eq!(stats[0].rx_bytes_delta, Some(1980.)); // A:20 + B:1900+60; C baseline only
    assert_eq!(stats[0].rx_bytes_per_second, Some(10. + 1960. / 8.));
    assert_eq!(stats[0].block_read_bytes_delta, Some(10.));
    assert_eq!(stats[0].block_read_bytes_per_second, Some(5.));
    assert_eq!(stats[1].rx_bytes_delta, Some(60.));
    assert_eq!(stats[1].rx_bytes_per_second, Some(30.));
    assert_eq!(stats[1].block_read_bytes_delta, Some(60.));
    assert_eq!(stats[1].block_read_bytes_per_second, Some(30.));
    sqlx::query("DELETE FROM containers WHERE id=$1")
        .bind(inventory)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        reader
            .platform(scope.platform_id, window, 6600)
            .await
            .unwrap(),
        stats
    );
    // A clipped range starts each physical container with a fresh baseline.
    let clipped = reader
        .platform(
            scope.platform_id,
            StatsWindow::between(3009, 6600, 6600).unwrap(),
            6600,
        )
        .await
        .unwrap();
    assert_eq!(clipped[0].rx_bytes_delta, None);
    assert_eq!(clipped[1].rx_bytes_delta, Some(60.));
    // Reconfiguration still fences I/O writes together with the rest of telemetry.
    let mut stale = host(&scope, 3012, 1);
    stale.scope.address = Some("stale-address".into());
    stale.container_io = vec![io("a", 99999., Some(99999.))];
    assert_eq!(store.persist_batch(&[stale]).await.unwrap().stale, 1);
    assert_eq!(
        reader
            .platform(scope.platform_id, window, 6600)
            .await
            .unwrap(),
        stats
    );
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn swarm_platform_io_aggregates_nodes_without_losing_history_or_overwriting_host_gauges() {
    use citadel_adapters::persistence::postgres::platforms::statistics::reader::PostgresStatisticsReader;
    use citadel_platforms::{ContainerIoSnapshot, StatisticsReader, StatsWindow};
    let pool = fixture().await;
    let manager = platform(&pool).await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"DockerSwarm\",\"nodeID\":\"manager\"}' WHERE id=$1")
        .bind(manager.platform_id).execute(&pool).await.unwrap();
    let connected: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT now()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut nodes = Vec::new();
    for id in ["worker-a", "worker-b", "manager"] {
        let mut scope = manager.clone();
        scope.node_id = Some(id.into());
        scope.connector = "EdgeAgent".into();
        scope.agent_id = Some(Uuid::now_v7());
        scope.connected_at = Some(connected);
        scope.address = None;
        sqlx::query("INSERT INTO edgeagentbindings(id,agentfingerprint,agentid,agentpublickey,connectionstatus,platformid,resourceid,resourcetype,dockernodeid,lastconnectedatutc) VALUES($1,$2,$1,'fixture','Connected',$3,$3,'Platform',$4,$5)")
            .bind(scope.agent_id).bind(scope.agent_id.unwrap().to_string()).bind(scope.platform_id)
            .bind(id).bind(connected).execute(&pool).await.unwrap();
        nodes.push(scope);
    }
    let inventory = add_container(&pool, &nodes[0], "same-id").await;
    let capture = |scope: &StatsScope, created, id: &str, bytes, disk| {
        let mut sample = host(scope, created, 999);
        sample.metadata = None;
        sample.container_io = vec![ContainerIoSnapshot {
            node_id: None,
            created: None,
            container_id: id.into(),
            rx_bytes: bytes,
            tx_bytes: bytes,
            block_read_bytes: disk,
            block_write_bytes: disk,
        }];
        sample
    };
    let samples = vec![
        capture(&manager, 3001, "same-id", 1000., Some(100.)),
        capture(&manager, 3003, "same-id", 1020., Some(110.)),
        capture(&nodes[0], 3001, "same-id", 100., None),
        capture(&nodes[0], 3003, "same-id", 10., Some(10.)),
        capture(&nodes[1], 3001, "same-id", 10000., Some(1000.)),
        capture(&nodes[1], 3003, "same-id", 10040., Some(1020.)),
        // Only worker samples in the next bucket. A replacement starts a new baseline.
        capture(&nodes[0], 3011, "same-id", 30., Some(30.)),
        capture(&nodes[1], 3011, "replacement", 9000., Some(900.)),
        capture(&nodes[0], 3013, "same-id", 50., Some(50.)),
        capture(&nodes[1], 3013, "replacement", 9020., Some(920.)),
    ];
    let store = PostgresStatsBatchStore::new(pool.clone());
    assert_eq!(store.persist_batch(&samples).await.unwrap().persisted, 10);
    assert_eq!(store.persist_batch(&samples).await.unwrap().persisted, 10);
    assert_eq!(
        store
            .persist_batch(&[capture(&nodes[2], 3003, "same-id", 999999., Some(999999.))])
            .await
            .unwrap()
            .stale,
        1
    );
    let reader = PostgresStatisticsReader::new(pool.clone());
    let window = StatsWindow::between(3000, 6600, 6600).unwrap();
    let stats = reader
        .platform(manager.platform_id, window, 6600)
        .await
        .unwrap();
    assert_eq!(stats.len(), 2);
    assert_eq!(stats[0].cpu_usage, Some(10.));
    assert_eq!(stats[0].memory_usage, Some(10.));
    assert_eq!(stats[0].rx_bytes_delta, Some(70.));
    assert_eq!(stats[0].rx_bytes_per_second, Some(35.));
    assert_eq!(stats[0].block_read_bytes_delta, Some(30.));
    assert_eq!(stats[0].block_read_bytes_per_second, Some(15.));
    assert_eq!(stats[1].cpu_usage, None);
    assert_eq!(stats[1].memory_usage, None);
    assert_eq!(stats[1].rx_bytes_delta, Some(60.));
    assert_eq!(stats[1].rx_bytes_per_second, Some(14.));
    assert_eq!(stats[1].block_read_bytes_delta, Some(60.));
    assert_eq!(stats[1].block_read_bytes_per_second, Some(14.));
    let host_rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM platformstats WHERE platformid=$1")
            .bind(manager.platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(host_rows, 2);
    // Deleting inventory and revoking the node never delete its historical traffic.
    sqlx::query("DELETE FROM containers WHERE id=$1")
        .bind(inventory)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE edgeagentbindings SET revokedatutc=now() WHERE agentid=$1")
        .bind(nodes[0].agent_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        store
            .persist_batch(&[capture(&nodes[0], 3015, "same-id", 90000., None)])
            .await
            .unwrap()
            .stale,
        1
    );
    sqlx::query("UPDATE edgeagentbindings SET lastconnectedatutc=lastconnectedatutc+interval '1 second' WHERE agentid=$1")
        .bind(nodes[1].agent_id).execute(&pool).await.unwrap();
    assert_eq!(
        store
            .persist_batch(&[capture(&nodes[1], 3015, "replacement", 90000., None)])
            .await
            .unwrap()
            .stale,
        1
    );
    assert_eq!(
        reader
            .platform(manager.platform_id, window, 6600)
            .await
            .unwrap(),
        stats
    );
    let other = platform(&pool).await;
    assert!(
        reader
            .platform(other.platform_id, window, 6600)
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query("DELETE FROM platforms WHERE id IN ($1,$2)")
        .bind(manager.platform_id)
        .bind(other.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM platformnodeiostats WHERE platformid=$1")
            .bind(manager.platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    pool.close().await;
}
