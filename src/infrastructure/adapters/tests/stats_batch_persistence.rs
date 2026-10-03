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
            .stale,
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
