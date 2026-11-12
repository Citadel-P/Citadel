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
    let group = format!("containers:{}", f.platform_id);
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
    assert_eq!(
        receive(&mut socket).await["target"],
        "ContainersInfoUpdated"
    );
    let mut lock = f.pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE containers IN ACCESS EXCLUSIVE MODE")
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
