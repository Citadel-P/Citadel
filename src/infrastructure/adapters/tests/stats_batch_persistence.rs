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
async fn unchanged_retries_reuse_statistics_tuples_but_corrected_counters_are_persisted() {
    use citadel_adapters::persistence::postgres::platforms::statistics::store::PostgresContainerStatsStore;
    use citadel_platforms::ContainerStatsStore;

    let pool = fixture().await;
    for batched in [false, true] {
        let scope = platform(&pool).await;
        let id = add_container(&pool, &scope, "retry").await;
        let deployment = Uuid::now_v7();
        sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,'retry',$2,'{}','Stopped','00000000-0000-0000-0000-000000000001')")
            .bind(deployment).bind(scope.platform_id).execute(&pool).await.unwrap();
        sqlx::query("UPDATE containers SET deploymentid=$2 WHERE id=$1")
            .bind(id)
            .bind(deployment)
            .execute(&pool)
            .await
            .unwrap();
        let direct = PostgresContainerStatsStore::new(pool.clone());
        let batch = PostgresStatsBatchStore::new(pool.clone());
        let mut sample = container(&scope, "retry", 10, 1.0);
        let mut previous_tuples = Vec::<String>::new();
        for attempt in 0..3 {
            if attempt == 2 {
                sample.sample.rx_bytes = 1234.0;
                sample.sample.block_read_bytes = Some(4321.0);
            }
            let accepted = if batched {
                batch
                    .persist_batch(std::slice::from_ref(&sample))
                    .await
                    .unwrap()
                    .persisted
            } else {
                direct
                    .persist(scope.platform_id, std::slice::from_ref(&sample.sample))
                    .await
                    .unwrap()
            };
            assert_eq!(accepted, 1, "unchanged retries are still accepted");
            let mut tuples = Vec::new();
            for table in ["containerstats", "workloadstats"] {
                let rows: Vec<(String, f64, Option<f64>)> =
                    sqlx::query_as(sqlx::AssertSqlSafe(format!(
                        "SELECT s.ctid::text,h.rxbytes,h.blockreadbytes FROM {table} h JOIN statisticssources src USING(containerid) JOIN statisticssamples s ON s.sourceid=src.id AND s.created=h.created WHERE h.containerid=$1"
                    )))
                    .bind(id)
                    .fetch_all(&pool)
                    .await
                    .unwrap();
                assert_eq!(rows.len(), 1);
                assert_eq!(
                    (rows[0].1, rows[0].2),
                    (sample.sample.rx_bytes, sample.sample.block_read_bytes)
                );
                tuples.push(rows[0].0.clone());
            }
            if attempt == 1 {
                assert_eq!(
                    tuples, previous_tuples,
                    "replaying an unchanged sample must not write new tuples"
                );
            } else if attempt == 2 {
                assert!(
                    tuples.iter().zip(&previous_tuples).all(|(a, b)| a != b),
                    "corrected samples must update both histories"
                );
            }
            previous_tuples = tuples;
        }
    }
    pool.close().await;
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
    let transactions: i64 = sqlx::query_scalar("SELECT count(DISTINCT s.xmin::text) FROM statisticssamples s JOIN statisticssources src ON src.id=s.sourceid WHERE src.platformid=ANY($1)")
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
            sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes) SELECT statistics_source(platformid,id,NULL),$2,10,100,20,200,$3,$4,$3*2,$4*2 FROM containers WHERE id=$1")
                .bind(id).bind(created as i64).bind(rx).bind(tx).execute(&pool).await.unwrap();
            sqlx::query("UPDATE statisticssamples SET ownerid=statistics_owner(sourceid,'Workload',$2,NULL,NULL,NULL,'','deployment'),rxbytes=$5,txbytes=$6 WHERE sourceid=statistics_source($1,$3,NULL) AND created=$4")
                .bind(scope.platform_id).bind(deployment).bind(id).bind(created as i64).bind(rx).bind(tx).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO statisticssamples(sourceid,ownerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes) SELECT src,statistics_owner(src,'Service',NULL,NULL,NULL,'network-service','network-service','slot:1'),$3,10,100,20,200,$4,$5,$4*2,$5*2 FROM (SELECT statistics_source($1,NULL,$2) AS src) s")
                .bind(scope.platform_id).bind(id.to_string()).bind(created as i64).bind(rx).bind(tx).execute(&pool).await.unwrap();
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

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn platform_history_large_window_preserves_totals_and_restores_connection_settings() {
    use citadel_adapters::persistence::postgres::platforms::statistics::reader::PostgresStatisticsReader;
    use citadel_platforms::{StatisticsReader, StatsWindow};
    let migration_pool = fixture().await;
    migration_pool.close().await;
    // Force reuse of the same connection, including after an aborted query.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap())
        .await
        .unwrap();
    sqlx::query("SET jit = on").execute(&pool).await.unwrap();
    let scope = platform(&pool).await;
    // Two days of 30-second captures, fifteen physical containers. This expands
    // to 86,415 counter rows but only 577 chart buckets; don't assert wall time.
    sqlx::query(r#"
        INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,containerio)
        SELECT gen_random_uuid(),$1,t,2,100,t*15,t*30,
            (SELECT jsonb_agg(jsonb_build_object(
                'containerId',md5(c::text),'rxBytes',t,'txBytes',t*2,
                'blockReadBytes',t*3,'blockWriteBytes',t*4))
             FROM generate_series(1,15) c)
        FROM generate_series(0,172800,30) t
    "#).bind(scope.platform_id).execute(&pool).await.unwrap();
    let reader = PostgresStatisticsReader::new(pool.clone());
    let window = StatsWindow::between(0, 172800, 172800).unwrap();
    let stats = reader
        .platform(scope.platform_id, window, 172800)
        .await
        .unwrap();
    assert_eq!(stats.len(), 577);
    assert_eq!(stats.first().unwrap().created, 0);
    assert_eq!(stats.last().unwrap().created, 172800);
    for (index, stat) in stats.iter().enumerate() {
        let elapsed = if index == 0 {
            270.
        } else if index == 576 {
            30.
        } else {
            300.
        };
        assert_eq!(stat.rx_bytes_delta, Some(elapsed * 15.));
        assert_eq!(stat.tx_bytes_delta, Some(elapsed * 30.));
        assert_eq!(stat.block_read_bytes_delta, Some(elapsed * 45.));
        assert_eq!(stat.block_write_bytes_delta, Some(elapsed * 60.));
        assert_eq!(stat.rx_bytes_per_second, Some(15.));
        assert_eq!(stat.tx_bytes_per_second, Some(30.));
        assert_eq!(stat.block_read_bytes_per_second, Some(45.));
        assert_eq!(stat.block_write_bytes_per_second, Some(60.));
        assert_eq!(stat.cpu_usage, Some(2.));
        assert_eq!(stat.memory_usage, Some(100.));
    }
    let jit: String = sqlx::query_scalar("SHOW jit")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        jit, "on",
        "history must not change the pooled connection's JIT setting"
    );
    // Invalid capture shape causes SQL execution to fail after SET LOCAL. The
    // transaction must roll back and leave the connection usable and unchanged.
    sqlx::query("UPDATE platformstats SET containerio='{}' WHERE platformid=$1 AND created=0")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        reader
            .platform(scope.platform_id, window, 172800)
            .await
            .is_err()
    );
    let jit: String = sqlx::query_scalar("SHOW jit")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        jit, "on",
        "failed history reads must restore the setting too"
    );
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

// Storage maintenance is global: these destructive probes get a child database
// so they can never race the other batching tests, even with parallel test threads.
async fn storage_fixture() -> (PgPool, String) {
    let admin_url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
    let admin = PgPool::connect(&admin_url).await.unwrap();
    let name = format!("citadel_storage_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
    let mut url = url::Url::parse(&admin_url).unwrap();
    url.set_path(&name);
    citadel_database::MigrationRunner::migrate(url.as_str())
        .await
        .unwrap();
    (PgPool::connect(url.as_str()).await.unwrap(), name)
}
async fn close_storage_fixture(pool: PgPool, name: String) {
    pool.close().await;
    let admin = PgPool::connect(&std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap())
        .await
        .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE {name} WITH (FORCE)"
    )))
    .execute(&admin)
    .await
    .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn compaction_prunes_unrelated_days_with_custom_and_generic_plans() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() / 86_400 - 7) * 86_400;
    for day in 0..7_i64 {
        sqlx::query("SELECT statistics_partition('statisticssamples',$1)")
            .bind(start / 86_400 + day)
            .execute(&pool)
            .await
            .unwrap();
    }
    let mut sources = Vec::new();
    for name in ["compact-bounded", "compact-unrelated"] {
        let id = add_container(&pool, &scope, name).await;
        let source: i64 = sqlx::query_scalar(
            "INSERT INTO statisticssources(platformid,containerid) VALUES($1,$2) RETURNING id",
        )
        .bind(scope.platform_id)
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) SELECT $1,$2+n*10,1,100,5,200,n*100,n*200 FROM generate_series(0,60479) n")
            .bind(source).bind(start).execute(&pool).await.unwrap();
        sources.push(source);
    }
    sqlx::query("ANALYZE statisticssamples")
        .execute(&pool)
        .await
        .unwrap();
    let from = start + 3 * 86_400 + 3600;
    let partition = format!("statisticssamples_d{}", from / 86_400);
    fn check_scans(node: &serde_json::Value, partition: &str, scans: &mut usize) {
        if node["Node Type"]
            .as_str()
            .is_some_and(|kind| kind.contains("Scan"))
            && node["Relation Name"]
                .as_str()
                .is_some_and(|name| name.starts_with("statisticssamples"))
            && node["Actual Loops"].as_u64().unwrap_or(0) > 0
        {
            assert_eq!(
                node["Relation Name"].as_str().unwrap(),
                partition,
                "compaction must not scan unrelated days: {node}"
            );
            *scans += 1;
        }
        if let Some(children) = node["Plans"].as_array() {
            for child in children {
                check_scans(child, partition, scans);
            }
        }
    }
    for mode in ["force_custom_plan", "force_generic_plan"] {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SET LOCAL plan_cache_mode='{mode}'"
        )))
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query(concat!(
            "PREPARE compact_bounded(bigint,bigint,bigint) AS ",
            include_str!("../src/persistence/postgres/platforms/statistics/compact.sql")
        ))
        .execute(&mut *tx)
        .await
        .unwrap();
        let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) EXECUTE compact_bounded({},{from},{})",
            sources[0],
            from + 3600
        )))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        sqlx::query("DEALLOCATE compact_bounded")
            .execute(&mut *tx)
            .await
            .unwrap();
        let mut scans = 0;
        check_scans(&plan[0]["Plan"], &partition, &mut scans);
        assert!(
            scans >= 2,
            "must inspect both sample selection and deletion scans"
        );
        let counts: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT sourceid,count(*) FROM statisticssamples GROUP BY sourceid ORDER BY sourceid",
        )
        .fetch_all(&mut *tx)
        .await
        .unwrap();
        assert_eq!(counts, vec![(sources[0], 60480 - 360), (sources[1], 60480)]);
        let remaining: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM statisticssamples WHERE sourceid=$1 AND created >= $2 AND created < $3",
        ).bind(sources[0]).bind(from).bind(from + 3600).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(remaining, 0);
        let summaries: (i64, i64) = sqlx::query_as(
            "SELECT count(*),sum(samplecount)::bigint FROM statisticsrollups WHERE sourceid=$1",
        )
        .bind(sources[0])
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(summaries, (60, 360));
        tx.rollback().await.unwrap();
    }
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn shared_history_compacts_with_weighted_gauges_resets_and_immutable_ownership() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let id = add_container(&pool, &scope, "compact").await;
    let a = Uuid::now_v7();
    let b = Uuid::now_v7();
    for dep in [a, b] {
        sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$1::text,$2,'{}','Stopped','00000000-0000-0000-0000-000000000001')")
            .bind(dep).bind(scope.platform_id).execute(&pool).await.unwrap();
    }
    let now = chrono::Utc::now().timestamp();
    let start = (now - 2 * 86_400) / 300 * 300;
    let store = PostgresStatsBatchStore::new(pool.clone());
    for minute in 0..12_i64 {
        sqlx::query("UPDATE containers SET deploymentid=$2 WHERE id=$1")
            .bind(id)
            .bind(if minute % 3 == 1 { b } else { a })
            .execute(&pool)
            .await
            .unwrap();
        let samples: Vec<_> = (0..6_i64)
            .map(|n| {
                let mut s = container(
                    &scope,
                    "compact",
                    start + minute * 60 + n * 10,
                    (minute * 6 + n) as f64,
                );
                s.sample.rx_bytes = [100., 110., 5., 15., 25., 35.][n as usize];
                s.sample.tx_bytes = (minute * 6 + n) as f64 * 100.;
                s.sample.block_read_bytes = if n == 2 {
                    None
                } else {
                    Some(s.sample.rx_bytes * 2.)
                };
                s.sample.block_write_bytes = Some(s.sample.tx_bytes * 2.);
                s
            })
            .collect();
        assert_eq!(store.persist_batch(&samples).await.unwrap().persisted, 6);
    }
    // Keep the newest actual capture raw, including for an offline source.
    store
        .persist_batch(&[container(&scope, "compact", start + 720, 5.)])
        .await
        .unwrap();
    let query = concat!(
        "SELECT to_jsonb(result) FROM (",
        include_str!("../src/persistence/postgres/platforms/statistics/container_history.sql"),
        ") result"
    );
    let before: Vec<serde_json::Value> = sqlx::query_scalar(query)
        .bind(vec![id])
        .bind(start)
        .bind(300_i64)
        .bind(now)
        .fetch_all(&pool)
        .await
        .unwrap();
    let workload_query = concat!(
        "SELECT to_jsonb(result) FROM (",
        include_str!("../src/persistence/postgres/platforms/statistics/workload_history.sql"),
        ") result"
    );
    let mut owners_before = Vec::new();
    for dep in [a, b] {
        let rows: Vec<serde_json::Value> = sqlx::query_scalar(workload_query)
            .bind(dep)
            .bind(true)
            .bind(start)
            .bind(now)
            .bind(300_i64)
            .fetch_all(&pool)
            .await
            .unwrap();
        owners_before.push(rows);
    }
    // Compaction is atomic if the maintenance future is cancelled/rolled back.
    let source: i64 = sqlx::query_scalar("SELECT id FROM statisticssources WHERE containerid=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(scope.platform_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let removed: i64 = sqlx::query_scalar(include_str!(
        "../src/persistence/postgres/platforms/statistics/compact.sql"
    ))
    .bind(source)
    .bind(start)
    .bind(start + 720)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(removed, 72);
    tx.rollback().await.unwrap();
    let raw: i64 = sqlx::query_scalar("SELECT count(*) FROM statisticssamples WHERE sourceid=$1")
        .bind(source)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(raw, 73);
    for _ in 0..20 {
        maintain_statistics(&pool).await.unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM statisticsrollups WHERE sourceid=$1")
                .bind(source)
                .fetch_one(&pool)
                .await
                .unwrap();
        if count == 12 {
            break;
        }
    }
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM statisticssamples WHERE sourceid=$1),(SELECT count(*) FROM statisticsrollups WHERE sourceid=$1),(SELECT sum(samplecount)::bigint FROM statisticsobservations WHERE sourceid=$1)").bind(source).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (1, 12, 73));
    let after: Vec<serde_json::Value> = sqlx::query_scalar(query)
        .bind(vec![id])
        .bind(start)
        .bind(300_i64)
        .bind(now)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        before, after,
        "weighted gauges and reset-aware rates/totals survive minute compaction"
    );
    for (dep, before) in [a, b].into_iter().zip(owners_before) {
        let after: Vec<serde_json::Value> = sqlx::query_scalar(workload_query)
            .bind(dep)
            .bind(true)
            .bind(start)
            .bind(now)
            .bind(300_i64)
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(
            before, after,
            "ownership is captured, never inferred from current inventory"
        );
    }
    let peak: f64 =
        sqlx::query_scalar("SELECT max(peaks[1]) FROM statisticsrollups WHERE sourceid=$1")
            .bind(source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(peak, 71.);
    let late = store
        .persist_batch(&[container(&scope, "compact", start + 10, 999.)])
        .await
        .unwrap();
    assert_eq!((late.persisted, late.stale), (0, 1));
    let unchanged: Vec<serde_json::Value> = sqlx::query_scalar(query)
        .bind(vec![id])
        .bind(start)
        .bind(300_i64)
        .bind(now)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(after, unchanged);
    // A partial summary cannot claim traffic outside the requested boundaries.
    let partial: Vec<serde_json::Value> = sqlx::query_scalar(query)
        .bind(vec![id])
        .bind(start + 1)
        .bind(60_i64)
        .bind(start + 119)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(partial.len(), 1);
    assert_eq!(partial[0]["created"], start + 60);
    sqlx::query("DELETE FROM containers WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let retained: i64 = sqlx::query_scalar(
        "SELECT sum(samplecount)::bigint FROM workloadstats WHERE containerid=$1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        retained, 73,
        "redeployment/deletion retains all workload observations"
    );
    sqlx::query("DELETE FROM deployments WHERE id=ANY($1)")
        .bind(vec![a, b])
        .execute(&pool)
        .await
        .unwrap();
    let removed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM workloadstats WHERE containerid=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        removed, 0,
        "deleting the workloads removes their logical history"
    );
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn partition_retention_drops_expired_days_and_bounds_default_partition_cleanup() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let id = add_container(&pool, &scope, "partition").await;
    let source: i64 = sqlx::query_scalar("SELECT statistics_source($1,$2,NULL)")
        .bind(scope.platform_id)
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let now = chrono::Utc::now().timestamp();
    let old_day = now / 86400 - 9;
    let default_day = old_day - 2;
    sqlx::query("SELECT statistics_partition('statisticssamples',$1)")
        .bind(old_day)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) VALUES($1,$2,1,1,1,1,1,1),($1,$3,2,2,2,2,2,2)")
        .bind(source).bind(old_day*86400).bind(now).execute(&pool).await.unwrap();
    let compacted_day = now / 86400 - 3;
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) SELECT $1,$2+n*10,1,1,1,1,n,n FROM generate_series(0,5) n")
        .bind(source).bind(compacted_day*86400).execute(&pool).await.unwrap();
    // No precreated partition for this day: ingest must remain possible even if
    // maintenance is offline. Populated default ranges cannot be silently hidden.
    sqlx::query("INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) SELECT $1,$2+n,1,1,1,1,1,1 FROM generate_series(1,5001) n")
        .bind(source).bind(default_day*86400).execute(&pool).await.unwrap();
    sqlx::query("SELECT statistics_partition('statisticssamples',$1)")
        .bind(default_day)
        .execute(&pool)
        .await
        .unwrap();
    let default_exists: bool = sqlx::query_scalar(
        "SELECT to_regclass('public.statisticssamples_d'||$1::text) IS NOT NULL",
    )
    .bind(default_day.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!default_exists);
    maintain_statistics(&pool).await.unwrap();
    let dropped: bool =
        sqlx::query_scalar("SELECT to_regclass('public.statisticssamples_d'||$1::text) IS NULL")
            .bind(old_day.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(dropped, "expired physical partition must release its files");
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM statisticssamples_default WHERE sourceid=$1")
            .bind(source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        remaining, 1,
        "default cleanup stays bounded at 5000 rows per pass"
    );
    let fresh: f64 = sqlx::query_scalar(
        "SELECT cpuusage FROM statisticssamples WHERE sourceid=$1 AND created=$2",
    )
    .bind(source)
    .bind(now)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        fresh, 2.,
        "same physical ctid in another partition must not be deleted"
    );
    maintain_statistics(&pool).await.unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM statisticssamples WHERE sourceid=$1")
            .bind(source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, 1);
    let compacted_dropped: bool =
        sqlx::query_scalar("SELECT to_regclass('public.statisticssamples_d'||$1::text) IS NULL")
            .bind(compacted_day.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        compacted_dropped,
        "empty compacted days must release their old indexes too"
    );
    let observations: i64 = sqlx::query_scalar(
        "SELECT sum(samplecount)::bigint FROM statisticsobservations WHERE sourceid=$1",
    )
    .bind(source)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        observations, 7,
        "dropping empty raw partitions preserves summaries and the current capture"
    );
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn sparse_history_and_ownership_segments_remain_separate_during_compaction() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let id = add_container(&pool, &scope, "segments").await;
    let source: i64 = sqlx::query_scalar("SELECT statistics_source($1,$2,'segments-task')")
        .bind(scope.platform_id)
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    // Service snapshots intentionally survive missing/deleted managed resources.
    let a: i64 = sqlx::query_scalar(
        "SELECT statistics_owner($1,'Service',NULL,$2,NULL,'svc','svc','slot:1')",
    )
    .bind(source)
    .bind(Uuid::now_v7())
    .fetch_one(&pool)
    .await
    .unwrap();
    let b: i64 = sqlx::query_scalar(
        "SELECT statistics_owner($1,'Service',NULL,NULL,NULL,'svc','svc','slot:2')",
    )
    .bind(source)
    .fetch_one(&pool)
    .await
    .unwrap();
    let start = (chrono::Utc::now().timestamp() - 2 * 86400) / 60 * 60;
    sqlx::query("INSERT INTO statisticssamples(sourceid,ownerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) SELECT $1,CASE WHEN n BETWEEN 4 AND 7 THEN $3 ELSE $2 END,$4+n,n,1,1,1,n,n FROM generate_series(0,11) n")
        .bind(source).bind(a).bind(b).bind(start).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO statisticssamples(sourceid,ownerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) SELECT $1,$2,$3+n*60,1,1,1,1,n,n FROM generate_series(1,3) n")
        .bind(source).bind(a).bind(start).execute(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(scope.platform_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let removed: i64 = sqlx::query_scalar(include_str!(
        "../src/persistence/postgres/platforms/statistics/compact.sql"
    ))
    .bind(source)
    .bind(start)
    .bind(start + 180)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(removed, 12);
    tx.commit().await.unwrap();
    let summaries:Vec<(i64,i32,f64)>=sqlx::query_as("SELECT ownerid,samplecount,counterdeltas[1] FROM statisticsrollups WHERE sourceid=$1 ORDER BY created").bind(source).fetch_all(&pool).await.unwrap();
    assert_eq!(
        summaries,
        vec![(a, 4, 3.), (b, 4, 3.), (a, 4, 3.)],
        "A/B/A captures must not merge noncontiguous ownership"
    );
    let sparse: i64 =
        sqlx::query_scalar("SELECT count(*) FROM statisticssamples WHERE sourceid=$1")
            .bind(source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        sparse, 3,
        "minute polling does not inflate sparse history with larger summaries"
    );
    // Source identity can be used by Swarm-only history without a container copy.
    sqlx::query("UPDATE statisticssources SET dockertaskid='segments-task' WHERE id=$1")
        .bind(source)
        .execute(&pool)
        .await
        .unwrap();
    let service_rows: i64 = sqlx::query_scalar(
        "SELECT sum(samplecount)::bigint FROM swarmservicestats WHERE platformid=$1",
    )
    .bind(scope.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(service_rows, 15);
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    close_storage_fixture(pool, database).await;
}

async fn history_node(pool: &PgPool, manager: &StatsScope, node: &str) -> StatsScope {
    let mut scope = manager.clone();
    scope.node_id = Some(node.into());
    scope.connector = "EdgeAgent".into();
    scope.agent_id = Some(Uuid::now_v7());
    scope.connected_at = Some(
        sqlx::query_scalar("SELECT now()")
            .fetch_one(pool)
            .await
            .unwrap(),
    );
    scope.address = None;
    sqlx::query("INSERT INTO edgeagentbindings(id,agentfingerprint,agentid,agentpublickey,connectionstatus,platformid,resourceid,resourcetype,dockernodeid,lastconnectedatutc) VALUES($1,$2,$1,'fixture','Connected',$3,$3,'Platform',$4,$5)")
        .bind(scope.agent_id).bind(scope.agent_id.unwrap().to_string()).bind(scope.platform_id)
        .bind(node).bind(scope.connected_at).execute(pool).await.unwrap();
    scope
}

async fn platform_history(
    pool: &PgPool,
    id: Uuid,
    from: i64,
    until: i64,
    bucket: i64,
) -> Vec<serde_json::Value> {
    let query = format!(
        "SELECT to_jsonb(h) FROM ({}) h",
        include_str!("../src/persistence/postgres/platforms/statistics/platform_history.sql")
    );
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL jit=off")
        .execute(&mut *tx)
        .await
        .unwrap();
    let rows = sqlx::query_scalar(sqlx::AssertSqlSafe(query))
        .bind(id)
        .bind(from)
        .bind(bucket)
        .bind(until)
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    rows
}

fn assert_history_close(before: &[serde_json::Value], after: &[serde_json::Value]) {
    assert_eq!(before.len(), after.len());
    for (before, after) in before.iter().zip(after) {
        assert_eq!(before["created"], after["created"]);
        for (key, value) in before.as_object().unwrap() {
            match (value.as_f64(), after[key].as_f64()) {
                (Some(a), Some(b)) => assert!(
                    (a - b).abs() <= a.abs().max(1.) * 1e-10,
                    "{key}: {a} != {b}"
                ),
                _ => assert_eq!(value, &after[key], "{key}"),
            }
        }
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn platform_history_compacts_host_and_nodes_without_losing_resets_gaps_or_gauges() {
    use citadel_platforms::ContainerIoSnapshot;
    let (pool, database) = storage_fixture().await;
    let manager = platform(&pool).await;
    let node = history_node(&pool, &manager, "worker").await;
    let start = (chrono::Utc::now().timestamp() - 2 * 86_400) / 300 * 300;
    let store = PostgresStatsBatchStore::new(pool.clone());
    let mut captures = Vec::new();
    for scope in [&manager, &node] {
        for minute in 0..12_i64 {
            // A host gap must not erase the independent node-only bucket.
            if scope.node_id.is_none() && (5..10).contains(&minute) {
                continue;
            }
            for n in 0..6_i64 {
                let mut sample = host(scope, start + minute * 60 + n * 10, 1);
                sample.cpu_usage = (minute * 6 + n) as f64;
                sample.memory_active = sample.cpu_usage * 10.;
                sample.rx_bytes = sample.cpu_usage * 100.;
                sample.tx_bytes = sample.cpu_usage * 200.;
                if n == 2 {
                    sample.metadata = None;
                }
                let factor = if scope.node_id.is_some() { 10. } else { 1. };
                sample.container_io = vec![ContainerIoSnapshot {
                    node_id: None,
                    created: None,
                    container_id: "same-id".into(),
                    rx_bytes: [100., 110., 5., 15., 25., 35.][n as usize] * factor,
                    tx_bytes: (minute * 6 + n) as f64 * factor,
                    block_read_bytes: if n == 2 {
                        None
                    } else {
                        Some((minute * 6 + n) as f64 * factor)
                    },
                    block_write_bytes: None,
                }];
                // Disappearance, replacement, and a lone observation inside a minute.
                if minute < 4 && n != 3 {
                    let mut second = sample.container_io[0].clone();
                    second.container_id = "removed".into();
                    sample.container_io.push(second);
                } else if minute == 8 && n == 4 {
                    let mut second = sample.container_io[0].clone();
                    second.container_id = "one-sample".into();
                    sample.container_io.push(second);
                }
                // Empty capture does not reset another container's baseline.
                if minute == 3 && n == 2 {
                    sample.container_io.clear();
                }
                captures.push(sample);
            }
        }
        captures.push(host(scope, start + 720, 1)); // latest offline observation stays raw
    }
    assert_eq!(
        store.persist_batch(&captures).await.unwrap().persisted,
        captures.len()
    );
    sqlx::query("UPDATE platformstats SET alertpending=false WHERE platformid=$1")
        .bind(manager.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    let before = platform_history(&pool, manager.platform_id, start, start + 800, 300).await;
    assert!(before[1]["cpuusage"].is_null());
    assert!(before[1]["rxbytesdelta"].as_f64().unwrap() > 0.);
    let minute_before = platform_history(&pool, manager.platform_id, start, start + 800, 60).await;
    let source: i64 = sqlx::query_scalar(
        "SELECT id FROM platformhistorysources WHERE platformid=$1 AND nodeid=''",
    )
    .bind(manager.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(manager.platform_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let removed: i64 = sqlx::query_scalar(include_str!(
        "../src/persistence/postgres/platforms/statistics/platform_compact.sql"
    ))
    .bind(source)
    .bind(start)
    .bind(start + 720)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(removed, 36);
    tx.rollback().await.unwrap();
    assert_eq!(
        before,
        platform_history(&pool, manager.platform_id, start, start + 800, 300).await
    );
    let watermark: i64 =
        sqlx::query_scalar("SELECT compacteduntil FROM platformhistorysources WHERE id=$1")
            .bind(source)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(watermark, 0);
    for _ in 0..3 {
        maintain_statistics(&pool).await.unwrap();
    }
    assert_history_close(
        &before,
        &platform_history(&pool, manager.platform_id, start, start + 800, 300).await,
    );
    assert_history_close(
        &minute_before,
        &platform_history(&pool, manager.platform_id, start, start + 800, 60).await,
    );
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM platformstats),(SELECT count(*) FROM platformnodeiostats),(SELECT sum(samplecount)::bigint FROM platformgaugerollups)")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(
        counts,
        (7, 13, 36),
        "sparse container minutes stay wholly raw alongside dense summaries"
    );
    // Neither a valid host retry nor a valid Edge node retry may resurrect sealed raw data.
    let retried = store
        .persist_batch(&[captures[0].clone(), captures[43].clone()])
        .await
        .unwrap();
    assert_eq!(retried.persisted, 0);
    assert_eq!(retried.stale, 2);
    // The legacy direct writer must respect the same watermark.
    let _id = add_container(&pool, &manager, "same-id").await;
    use citadel_platforms::ContainerStatsStore;
    let direct=citadel_adapters::persistence::postgres::platforms::statistics::store::PostgresContainerStatsStore::new(pool.clone());
    direct
        .persist(
            manager.platform_id,
            &[container(&manager, "same-id", start, 1.).sample],
        )
        .await
        .unwrap();
    let raw: i64 = sqlx::query_scalar("SELECT count(*) FROM platformstats WHERE platformid=$1")
        .bind(manager.platform_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(raw, 7);
    // Removing inventory/bindings does not remove already captured platform traffic.
    sqlx::query("DELETE FROM containers WHERE platformid=$1")
        .bind(manager.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM edgeagentbindings WHERE platformid=$1")
        .bind(manager.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_history_close(
        &before,
        &platform_history(&pool, manager.platform_id, start, start + 800, 300).await,
    );
    let partial = platform_history(&pool, manager.platform_id, start + 1, start + 49, 60).await;
    assert!(
        partial.is_empty(),
        "partial historical minutes must not count outside-range traffic"
    );
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(manager.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        direct
            .persist_with_platform_stats(manager.platform_id, &[], None)
            .await
            .unwrap(),
        0,
        "a delayed empty capture after platform deletion stays a no-op"
    );
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn platform_compaction_protects_pending_alert_revisions_and_sparse_latest_samples() {
    let (pool, database) = storage_fixture().await;
    let manager = platform(&pool).await;
    let other = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() - 2 * 86_400) / 60 * 60;
    let store = PostgresStatsBatchStore::new(pool.clone());
    for scope in [&manager, &other] {
        let samples: Vec<_> = (0..18)
            .map(|n| host(scope, start + n * 10, 1))
            .chain([host(scope, start + 180, 1), host(scope, start + 240, 1)])
            .collect();
        store.persist_batch(&samples).await.unwrap();
    }
    sqlx::query("UPDATE platformstats SET alertpending=false WHERE platformid<>$1 OR created<>$2")
        .bind(manager.platform_id)
        .bind(start + 80)
        .execute(&pool)
        .await
        .unwrap();
    let pending: (Uuid, String) =
        sqlx::query_as("SELECT id,xmin::text FROM platformstats WHERE alertpending")
            .fetch_one(&pool)
            .await
            .unwrap();
    let latest: (Uuid, String) = sqlx::query_as(
        "SELECT id,xmin::text FROM platformstats WHERE platformid=$1 ORDER BY created DESC LIMIT 1",
    )
    .bind(manager.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    for _ in 0..4 {
        maintain_statistics(&pool).await.unwrap();
    }
    let pending_after: (Uuid, String) =
        sqlx::query_as("SELECT id,xmin::text FROM platformstats WHERE alertpending")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        pending, pending_after,
        "pending claims must remain byte/revision identical"
    );
    let counts:Vec<(Uuid,i64)>=sqlx::query_as("SELECT src.platformid,count(*) FROM platformgaugerollups r JOIN platformhistorysources src ON src.id=r.sourceid GROUP BY src.platformid ORDER BY src.platformid")
        .fetch_all(&pool).await.unwrap();
    assert!(counts.contains(&(manager.platform_id, 1)));
    assert!(
        counts.contains(&(other.platform_id, 3)),
        "a pending source cannot starve other platforms"
    );
    sqlx::query("UPDATE platformstats SET alertpending=false WHERE id=$1 AND xmin::text=$2")
        .bind(pending.0)
        .bind(&pending.1)
        .execute(&pool)
        .await
        .unwrap();
    maintain_statistics(&pool).await.unwrap();
    let raw: Vec<i64> = sqlx::query_scalar(
        "SELECT created FROM platformstats WHERE platformid=$1 ORDER BY created",
    )
    .bind(manager.platform_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        raw,
        vec![start + 180, start + 240],
        "sparse and latest offline samples stay raw"
    );
    let latest_after: (Uuid, String) = sqlx::query_as(
        "SELECT id,xmin::text FROM platformstats WHERE platformid=$1 ORDER BY created DESC LIMIT 1",
    )
    .bind(manager.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(latest, latest_after);
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn platform_summary_expiry_reclaims_partitions_and_bounds_default_cleanup() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let source: i64 = sqlx::query_scalar(
        "INSERT INTO platformhistorysources(platformid,nodeid) VALUES($1,'') RETURNING id",
    )
    .bind(scope.platform_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let day = chrono::Utc::now().timestamp() / 86_400;
    for parent in ["platformgaugerollups", "platformiorollups"] {
        sqlx::query("SELECT statistics_partition($1,$2)")
            .bind(parent)
            .bind(day - 10)
            .execute(&pool)
            .await
            .unwrap();
    }
    for (created, count) in [
        ((day - 12) * 86_400, 5005_i64),
        ((day - 10) * 86_400, 1),
        (day * 86_400, 1),
    ] {
        sqlx::query("INSERT INTO platformgaugerollups(sourceid,created,lastcreated,samplecount,cpuusage,memoryusage,rxbytes,txbytes,diskcounts)
            SELECT $1,$2+n,$2+n,4,1,2,3,4,ARRAY[0,0,0] FROM generate_series(0,$3-1) n")
            .bind(source).bind(created).bind(count).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO platformiorollups(sourceid,containerid,created,lastcreated,firstcounters,lastcounters,counterdeltas,counterdurations,readcount,writecount)
            SELECT $1,'removed',$2+n,$2+n,ARRAY[1,2,NULL,NULL]::double precision[],ARRAY[4,5,NULL,NULL]::double precision[],ARRAY[3,3,NULL,NULL]::double precision[],ARRAY[30,30,0,0]::bigint[],0,0 FROM generate_series(0,$3-1) n")
            .bind(source).bind(created).bind(count).execute(&pool).await.unwrap();
    }
    PostgresStatsBatchStore::new(pool.clone())
        .persist_batch(&[host(&scope, (day - 12) * 86_400, 1)])
        .await
        .unwrap();
    let pending: (Uuid, String) =
        sqlx::query_as("SELECT id,xmin::text FROM platformstats WHERE alertpending")
            .fetch_one(&pool)
            .await
            .unwrap();
    maintain_statistics(&pool).await.unwrap();
    for parent in ["platformgaugerollups", "platformiorollups"] {
        let dropped: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NULL")
            .bind(format!("{parent}_d{}", day - 10))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(dropped, "expired summary files must be released");
        let left: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {parent}_default"
        )))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(left, 5, "default cleanup must retain its 5000-row bound");
        let recent: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {parent} WHERE created=$1"
        )))
        .bind(day * 86_400)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(recent, 1);
    }
    let after: (Uuid, String) =
        sqlx::query_as("SELECT id,xmin::text FROM platformstats WHERE alertpending")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        pending, after,
        "expiry never drops even an expired pending alert sample"
    );
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_STATISTICS_DATABASE_URL"]
async fn platform_compactor_rechecks_alert_state_after_waiting_for_a_capture() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() - 2 * 86_400) / 60 * 60;
    let captures: Vec<_> = (0..7).map(|n| host(&scope, start + n * 10, 1)).collect();
    PostgresStatsBatchStore::new(pool.clone())
        .persist_batch(&captures)
        .await
        .unwrap();
    sqlx::query("UPDATE platformstats SET alertpending=false")
        .execute(&pool)
        .await
        .unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(scope.platform_id)
        .execute(&mut *writer)
        .await
        .unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap();
    // This correction is invisible to the compactor's initial candidate snapshot.
    sqlx::query(
        "UPDATE platformstats SET cpuusage=99,alertpending=true WHERE platformid=$1 AND created=$2",
    )
    .bind(scope.platform_id)
    .bind(start)
    .execute(&mut *writer)
    .await
    .unwrap();
    let worker_pool = pool.clone();
    let task = tokio::spawn(async move {
        citadel_adapters::persistence::postgres::maintenance::compact(&worker_pool).await
    });
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut blocked = false;
    while tokio::time::Instant::now() < deadline && !task.is_finished() {
        blocked=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)) AND query LIKE 'SELECT id FROM platforms%')")
            .bind(pid).fetch_one(&pool).await.unwrap();
        if blocked {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    writer.commit().await.unwrap();
    let result = task.await.unwrap();
    assert!(
        blocked,
        "the compactor must serialize with a concurrent capture"
    );
    result.unwrap();
    let state:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM platformstats),(SELECT count(*) FROM platformgaugerollups),(SELECT compacteduntil FROM platformhistorysources LIMIT 1)")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(
        state,
        (7, 0, 0),
        "the fresh alert revision must prevent compaction"
    );
    sqlx::query("UPDATE platformstats SET alertpending=false")
        .execute(&pool)
        .await
        .unwrap();
    maintain_statistics(&pool).await.unwrap();
    let raw: i64 = sqlx::query_scalar("SELECT count(*) FROM platformstats")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(raw, 1);
    close_storage_fixture(pool, database).await;
}

async fn maintain_statistics(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let retained =
        citadel_adapters::persistence::postgres::maintenance::cleanup(pool, None).await?;
    Ok(retained + citadel_adapters::persistence::postgres::maintenance::compact(pool).await?)
}

#[path = "stats_batch_persistence/query_efficiency.rs"]
mod query_efficiency;
#[path = "stats_batch_persistence/storage_review.rs"]
mod storage_review;
