use super::realtime_groups::{cleanup, invoke, reader, receive, token};
use super::*;
use citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository;
use citadel_identity::{UserRepository, UserResourceAccessInput};
use citadel_primitives::{PermissionLevel, ResourceType};
use citadel_server::{
    config::RealtimeConfig,
    metrics::Metrics,
    realtime::{IdentityRealtimeReader, RealtimeService},
};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

fn counter(family: &str, units: bool) -> u64 {
    let mut metrics = String::new();
    citadel_runtime::runtime_metrics::render_runtime_metrics(&mut metrics);
    let prefix = format!(
        "citadel_runtime_{}_total{{family=\"{family}\"}} ",
        if units { "units" } else { "iterations" }
    );
    metrics
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap()
        .parse()
        .unwrap()
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL; run with --test-threads=1 for process counters"]
async fn realtime_generation_burst_shares_reads_and_revokes_only_committed_actor() {
    let f = fixture().await;
    let alice = super::lookup::subject(&f).await;
    let bob = super::lookup::subject(&f).await;
    let users = PostgresUserRepository::new(f.pool.clone());
    let access = UserResourceAccessInput {
        resource_type: ResourceType::Platform,
        resource_id: f.platform_id,
        permission_level: PermissionLevel::Read,
        specific_permissions: vec![],
    };
    for actor in [&alice, &bob] {
        users
            .add_resource_access(
                actor.subject_id,
                &access,
                ActorId::new(SYSTEM_ACTOR_ID),
                Utc::now(),
                true,
            )
            .await
            .unwrap();
    }
    let cancellation = CancellationToken::new();
    let _guard = cancellation.clone().drop_guard();
    let service = RealtimeService::new(
        &RealtimeConfig {
            queue_capacity: 256,
            max_connections: 8,
            subscribe_timeout: StdDuration::from_secs(5),
            send_timeout: StdDuration::from_secs(2),
            authorization_recheck_interval: StdDuration::from_secs(3600),
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
    let hub = service.hub();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = cancellation.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, service.router())
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .unwrap()
    });
    let group = format!("containers:{}", f.platform_id);
    let mut clients = Vec::new();
    for actor in [&alice, &alice, &bob, &bob] {
        let (mut socket, _) =
            tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
                .await
                .unwrap();
        socket.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(actor)}).to_string().into())).await.unwrap();
        assert_eq!(receive(&mut socket).await["kind"], "subscribed");
        assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
        assert_eq!(
            receive(&mut socket).await["target"],
            "ContainersInfoUpdated"
        );
        clients.push(socket);
    }
    let authentication = counter("RealtimeAuthentication", false);
    let misses = counter("RealtimePermissionMiss", false);
    let reads = counter("RealtimeSharedRead", false);
    let acl_queries = counter("AuthorizationResourceQuery", false);
    for _ in 0..100 {
        hub.publish_container_stats(f.platform_id, &[]);
        for client in &mut clients {
            assert_eq!(receive(client).await["target"], "ContainersStatsUpdated");
        }
    }
    assert_eq!(counter("RealtimeAuthentication", false) - authentication, 0);
    assert_eq!(counter("RealtimePermissionMiss", false) - misses, 0);
    assert_eq!(
        counter("AuthorizationResourceQuery", false) - acl_queries,
        0
    );
    assert_eq!(counter("RealtimeSharedRead", false) - reads, 100);
    println!(
        "phase15 burst: connections=4 events=100 deliveries=400 authentication=0 permission_misses=0 ACL_SQL=0 shared_reads=100"
    );
    users
        .remove_resource_access(
            alice.subject_id,
            &access,
            ActorId::new(SYSTEM_ACTOR_ID),
            Utc::now(),
        )
        .await
        .unwrap();
    for client in &mut clients[..2] {
        let next = tokio::time::timeout(StdDuration::from_secs(2), client.next())
            .await
            .unwrap();
        assert!(
            matches!(next, Some(Ok(Message::Close(_))) | None),
            "revoked connection: {next:?}"
        );
    }
    hub.publish_container_stats(f.platform_id, &[]);
    for client in &mut clients[2..] {
        assert_eq!(receive(client).await["target"], "ContainersStatsUpdated");
    }
    assert_eq!(counter("RealtimeAuthentication", false) - authentication, 0);
    assert_eq!(counter("RealtimePermissionMiss", false) - misses, 0);
    assert_eq!(
        counter("AuthorizationResourceQuery", false) - acl_queries,
        0
    );
    println!(
        "phase15 revocation: Alice connections closed=2; Bob deliveries=2 authentication=0 permission_misses=0"
    );
    // A newly opened connection cannot rejoin using the now-removed ACL.
    let (mut denied, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    denied.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&alice)}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut denied).await["kind"], "subscribed");
    assert!(invoke(&mut denied, "JoinGroup", &group).await["error"].is_string());
    denied.close(None).await.unwrap();
    for client in &mut clients[2..] {
        client.close(None).await.unwrap();
    }
    cancellation.cancel();
    server.await.unwrap();
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL; run with --test-threads=1 for process counters"]
async fn committed_platform_and_image_rows_are_shared_only_for_the_event() {
    use citadel_server::realtime_groups::{Group, GroupReadPort};
    let f = fixture().await;
    let reader = reader(&f);
    let denied = super::lookup::subject(&f).await;
    let hub = citadel_server::realtime::RealtimeHub::new(8, Arc::new(Metrics::default()));
    let mut receiver = hub.subscribe();
    hub.publish_runtime_change(f.platform_id, "image", "update", "image");
    let event = receiver.recv().await.unwrap();
    let images = Group::parse(&format!("images:{}", f.platform_id)).unwrap();
    let platforms = Group::parse("platforms").unwrap();
    let before = counter("RealtimeSharedRead", false);
    let original_images = reader
        .read(&f.administrator, &images, Some(&event))
        .await
        .unwrap();
    let original_platform = reader
        .read(&f.administrator, &platforms, Some(&event))
        .await
        .unwrap();
    assert_eq!(counter("RealtimeSharedRead", false) - before, 2);
    sqlx::query("UPDATE platforms SET name='after-shared-read' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE images SET name='after-shared-read' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let reused_images = reader
        .read(&f.administrator, &images, Some(&event))
        .await
        .unwrap();
    let reused_platform = reader
        .read(&f.administrator, &platforms, Some(&event))
        .await
        .unwrap();
    assert_eq!(
        original_images.events[0].arguments,
        reused_images.events[0].arguments
    );
    assert_eq!(original_platform.rows[0].rows, reused_platform.rows[0].rows);
    assert_eq!(counter("RealtimeSharedRead", false) - before, 2);
    assert!(reader.read(&denied, &images, Some(&event)).await.is_err());
    hub.publish_runtime_change(f.platform_id, "image", "update", "image");
    let fresh = receiver.recv().await.unwrap();
    let fresh_platform = reader
        .read(&f.administrator, &platforms, Some(&fresh))
        .await
        .unwrap();
    let fresh_images = reader
        .read(&f.administrator, &images, Some(&fresh))
        .await
        .unwrap();
    assert_eq!(fresh_platform.rows[0].rows[0]["name"], "after-shared-read");
    assert_eq!(
        fresh_images.events[0].arguments[0]["images"][0]["name"],
        "after-shared-read"
    );
    assert_eq!(counter("RealtimeSharedRead", false) - before, 4);
    cleanup(f).await;
}
