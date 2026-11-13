use super::realtime_groups::{cleanup, invoke, reader, receive, token};
use super::*;
use citadel_server::{
    config::RealtimeConfig,
    metrics::Metrics,
    realtime::{IdentityRealtimeReader, RealtimeHub, RealtimeService},
    realtime_groups::{Group, GroupReadPort},
};
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn realtime_collection_recheck_does_not_reload_locked_inventory() {
    recheck_with_locked_tables(false).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn realtime_container_recheck_does_not_load_statistics_images_or_deployments() {
    recheck_with_locked_tables(true).await;
}

async fn recheck_with_locked_tables(detail: bool) {
    let f = fixture().await;
    let cancellation = CancellationToken::new();
    let _guard = cancellation.clone().drop_guard();
    let service = RealtimeService::new(
        &RealtimeConfig {
            queue_capacity: 32,
            max_connections: 4,
            subscribe_timeout: StdDuration::from_secs(2),
            send_timeout: StdDuration::from_secs(2),
            authorization_recheck_interval: StdDuration::from_millis(100),
            snapshot_limit: 1000,
        },
        Arc::new(IdentityRealtimeReader::new(
            f.lookup_state.platforms.identity.clone(),
            f.lookup_state.platforms.platforms.clone(),
        )),
        Arc::new(Metrics::default()),
        cancellation.clone(),
    )
    .with_groups(Arc::new(reader(&f)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = cancellation.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, service.router())
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .unwrap()
    });
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&f, &f.administrator).await}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut socket).await["kind"], "subscribed");
    let group = if detail {
        let id: Uuid = sqlx::query_scalar(
            "SELECT id FROM containers WHERE platformid=$1 AND dockercontainerid='container-1'",
        )
        .bind(f.platform_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
        format!("container-info:{id}")
    } else {
        format!("containers:{}", f.platform_id)
    };
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
    let initial = receive(&mut socket).await;
    assert_eq!(
        initial["target"],
        if detail {
            "ReceiveContainerInfo"
        } else {
            "ContainersInfoUpdated"
        }
    );
    let mut lock = f.pool.begin().await.unwrap();
    sqlx::query(if detail { "LOCK TABLE statisticssamples, statisticsrollups, images, deployments IN ACCESS EXCLUSIVE MODE" } else { "LOCK TABLE containers IN ACCESS EXCLUSIVE MODE" })
        .execute(&mut *lock)
        .await
        .unwrap();
    // Several safety ticks must authenticate and authorize without reading containers.
    tokio::time::sleep(StdDuration::from_millis(350)).await;
    let response = tokio::time::timeout(
        StdDuration::from_secs(1),
        invoke(&mut socket, "LeaveGroup", &group),
    )
    .await
    .expect("authorization recheck blocked on inventory");
    assert!(response["error"].is_null());
    lock.rollback().await.unwrap();
    socket.close(None).await.unwrap();
    cancellation.cancel();
    server.await.unwrap();
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn realtime_stack_statistics_do_not_reload_stack_configuration() {
    let f = fixture().await;
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    let spec: citadel_stacks::StackSpec =
        serde_json::from_value(json!({"$type":"WebEditor","composeFile":"services: {}"})).unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate,controlstate) VALUES($1,$2,$3,'WebEditor',$5,$4,'Idle')")
        .bind(stack).bind(format!("stats-{stack}")).bind(SYSTEM_ACTOR_ID)
        .bind(citadel_stacks::StackUpdateState::new(&spec).to_storage_value().unwrap())
        .bind(citadel_stacks::StackDriftPolicy::default().to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,$5,'Healthy','1')")
        .bind(release).bind(stack).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).bind(spec.to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE containers SET stackid=$1 WHERE platformid=$2 AND dockercontainerid='container-1'",
    )
    .bind(stack)
    .bind(f.platform_id)
    .execute(&f.pool)
    .await
    .unwrap();
    let groups = reader(&f);
    let group = Group::parse(&format!("stack-info:{stack}")).unwrap();
    let initial = groups.read(&f.administrator, &group, None).await.unwrap();
    assert_eq!(initial.events[0].target, "ReceiveStackContainersInfo");
    assert_eq!(initial.events[0].arguments[0].as_array().unwrap().len(), 1);
    let hub = RealtimeHub::new(8, Arc::new(Metrics::default()));
    let mut receiver = hub.subscribe();
    hub.publish_container_stats(
        f.platform_id,
        &[citadel_platforms::RuntimeContainerStat {
            docker_container_id: "container-1".into(),
            cpu_usage: 42.0,
            memory_active: 10.0,
            memory_cache: 2.0,
            memory_active_cache: None,
            memory_limit: 128.0,
            rx_bytes: 0.0,
            tx_bytes: 0.0,
            block_read_bytes: None,
            block_write_bytes: None,
            created: 2,
        }],
    );
    let change = receiver.recv().await.unwrap();
    let mut lock = f.pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE stacks, stackreleases IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let snapshot = tokio::time::timeout(
        StdDuration::from_secs(1),
        groups.read(&f.administrator, &group, Some(&change)),
    )
    .await
    .expect("statistics reread full stack configuration")
    .unwrap();
    assert_eq!(snapshot.events[0].target, "ReceiveStackContainerStats");
    assert_eq!(snapshot.events[0].arguments[0][0]["id"], "container-1");
    assert_eq!(
        snapshot.events[0].arguments[0][0]["containerStat"]["cpuUsage"],
        42.0
    );
    assert!(snapshot.events[0].arguments[0][0].get("state").is_none());
    lock.rollback().await.unwrap();
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn realtime_scoped_workloads_query_only_the_platform_and_preserve_unaffected_rows() {
    use citadel_server::realtime_groups::GroupSubscription;
    let f = fixture().await;
    let other = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$2,$2,'Local',2,0,1000,0,0,'{\"$type\":\"Docker\"}','Online')")
        .bind(other).bind(other.to_string()).execute(&f.pool).await.unwrap();
    let spec: citadel_deployments::DeploymentSpec = serde_json::from_value(json!({"image":{"$type":"External","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx"}})).unwrap();
    for platform in [f.platform_id, other] {
        for _ in 0..3 {
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,controlstate,createdbyactorid) VALUES($1,$2,$3,$4,'Healthy','Idle',$5)")
                .bind(id).bind(id.to_string()).bind(platform).bind(spec.to_storage_value().unwrap()).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
        }
    }
    let reader = reader(&f);
    let group = Group::parse("deployments").unwrap();
    let initial = reader.read(&f.administrator, &group, None).await.unwrap();
    assert_eq!(initial.rows[0].rows.len(), 6);
    let mut subscription = GroupSubscription::new(group.clone());
    subscription.apply(initial, 100).unwrap();
    // Malformed configuration on another platform must never be loaded by this scoped read.
    sqlx::query("UPDATE deployments SET spec='{}' WHERE platformid=$1")
        .bind(other)
        .execute(&f.pool)
        .await
        .unwrap();
    let hub = RealtimeHub::new(8, Arc::new(Metrics::default()));
    let mut receiver = hub.subscribe();
    hub.publish_runtime_change(f.platform_id, "platform", "update", "");
    let change = receiver.recv().await.unwrap();
    let scoped = reader
        .read(&f.administrator, &group, Some(&change))
        .await
        .unwrap();
    assert_eq!(scoped.rows[0].rows.len(), 3);
    assert!(
        scoped.rows[0]
            .rows
            .iter()
            .all(|row| row["platformId"] == json!(f.platform_id))
    );
    let events = subscription.apply(scoped, 100).unwrap();
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|e| e.target == "DeploymentInfoUpdated"));
    // Full snapshots still validate/read all authorized rows.
    assert!(reader.read(&f.administrator, &group, None).await.is_err());
    sqlx::query("UPDATE deployments SET spec=$1 WHERE platformid=$2")
        .bind(spec.to_storage_value().unwrap())
        .bind(other)
        .execute(&f.pool)
        .await
        .unwrap();
    let reconnect = reader.read(&f.administrator, &group, None).await.unwrap();
    assert_eq!(reconnect.rows[0].rows.len(), 6);
    let mut denied = f.administrator.clone();
    denied.roles.clear();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    assert!(reader.read(&denied, &group, Some(&change)).await.is_err());
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn realtime_telemetry_context_is_shared_without_bypassing_actor_access() {
    use citadel_platforms::stats_ingestion::{PlatformStatsSample, StatsScope};
    let f = fixture().await;
    let reader = reader(&f);
    let group = Group::parse("platforms").unwrap();
    let total = PlatformStatsSample {
        scope: StatsScope {
            platform_id: f.platform_id,
            node_id: None,
            connector: "Local".into(),
            address: None,
            agent_id: None,
            connected_at: None,
            closed: None,
        },
        created: 1,
        memory_active: 100.,
        cpu_usage: 20.,
        rx_bytes: 1.,
        tx_bytes: 2.,
        metadata: None,
        container_io: vec![],
    };
    let hub = RealtimeHub::new(8, Arc::new(Metrics::default()));
    let mut receiver = hub.subscribe();
    hub.publish_scoped_container_stats_with_total(f.platform_id, None, &[], Some(&total));
    let change = receiver.recv().await.unwrap();
    let initial = reader
        .read(&f.administrator, &group, Some(&change))
        .await
        .unwrap();
    assert_eq!(initial.events[0].target, "PlatformStatsUpdated");
    let mut lock = f.pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE platforms IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    for _ in 0..8 {
        let snapshot = tokio::time::timeout(
            StdDuration::from_secs(1),
            reader.read(&f.administrator, &group, Some(&change)),
        )
        .await
        .expect("subscriber reread the shared telemetry context")
        .unwrap();
        assert_eq!(snapshot.events[0].arguments, initial.events[0].arguments);
    }
    lock.rollback().await.unwrap();
    let mut denied = f.administrator.clone();
    denied.roles.clear();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    assert!(reader.read(&denied, &group, Some(&change)).await.is_err());
    sqlx::query("UPDATE platforms SET memtotal=memtotal+1000 WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    hub.publish_scoped_container_stats_with_total(f.platform_id, None, &[], Some(&total));
    let fresh = receiver.recv().await.unwrap();
    let refreshed = reader
        .read(&f.administrator, &group, Some(&fresh))
        .await
        .unwrap();
    assert_ne!(refreshed.events[0].arguments, initial.events[0].arguments);
    cleanup(f).await;
}
