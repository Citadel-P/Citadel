use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use citadel_adapters::container_stats_store::PostgresContainerStatsStore;
use citadel_adapters::platform_read_store::PostgresPlatformReadStore;
use citadel_database::MigrationRunner;
use citadel_domain::{ActorId, AuthenticatedPrincipalType};
use citadel_identity::ActorPrincipal;
use citadel_platforms::{
    ContainerStatsStore, ContainerView, PlatformCapabilitiesView, PlatformReadStore,
    PlatformStatView, PlatformView, RuntimeContainerStat, WorkloadStatusCounts,
};
use citadel_server::config::RealtimeConfig;
use citadel_server::metrics::Metrics;
use citadel_server::realtime::{RealtimeHub, RealtimeReadError, RealtimeReadPort, RealtimeService};
use futures_util::future::BoxFuture;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const TOKEN: &str = "phase0c-test-token-with-at-least-32-characters";

#[tokio::test]
async fn successful_mutations_invalidate_but_reads_and_failures_do_not() {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    let hub = RealtimeHub::new(8, Arc::new(Metrics::default()));
    let _subscriber = hub.subscribe();
    for (method, status, revision) in [
        (Method::GET, StatusCode::OK, 0),
        (Method::POST, StatusCode::BAD_REQUEST, 0),
        (Method::POST, StatusCode::INTERNAL_SERVER_ERROR, 0),
        (Method::POST, StatusCode::NO_CONTENT, 1),
        (Method::DELETE, StatusCode::OK, 2),
    ] {
        let app = citadel_server::realtime::notify_mutations(
            axum::Router::new().route(
                "/resource",
                axum::routing::any(move || async move { status }),
            ),
            "User",
        )
        .layer(axum::Extension(hub.clone()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/resource")
                    .method(method)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(hub.current_revision(), revision);
    }
}

#[tokio::test]
async fn subscription_sequences_events_and_resynchronizes_after_reconnect() {
    let platform_id = Uuid::now_v7();
    let allowed = Arc::new(AtomicBool::new(true));
    let reader = Arc::new(FakeReader {
        platform: platform(platform_id),
        allowed,
    });
    let metrics = Arc::new(Metrics::default());
    let shutdown = CancellationToken::new();
    let service = RealtimeService::new(&config(), reader, metrics, shutdown.clone());
    let hub = service.hub();
    let (address, server) = start_server(service, shutdown.clone()).await;

    let (mut first, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/phase0/realtime"))
            .await
            .unwrap();
    subscribe(&mut first, platform_id, 0).await;
    let snapshot = receive_json(&mut first).await;
    assert_eq!(snapshot["protocolVersion"], 1);
    assert_eq!(snapshot["sequence"], 1);
    assert_eq!(snapshot["resourceRevision"], 0);
    assert_eq!(snapshot["eventKind"], "snapshot");
    assert_eq!(snapshot["payload"]["containers"][0]["name"], "fixture");
    let first_connection = snapshot["connectionId"].as_str().unwrap().to_owned();

    hub.publish_runtime_change(Uuid::now_v7(), "container", "start", "other");
    let revision = hub.publish_runtime_change(platform_id, "container", "start", "container-1");
    assert_eq!(revision, 2);
    let event = receive_json(&mut first).await;
    assert_eq!(event["connectionId"], first_connection);
    assert_eq!(event["sequence"], 2);
    assert_eq!(event["resourceRevision"], 2);
    assert_eq!(event["eventKind"], "runtimeChanged");

    first
        .send(Message::Text(
            json!({
                "protocolVersion": 1,
                "kind": "resync",
                "lastSequence": 2,
                "lastResourceRevision": 2
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    let resync = receive_json(&mut first).await;
    assert_eq!(resync["sequence"], 3);
    assert_eq!(resync["resourceRevision"], 2);
    assert_eq!(resync["eventKind"], "snapshot");
    first.close(None).await.unwrap();

    let (mut second, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/phase0/realtime"))
            .await
            .unwrap();
    subscribe(&mut second, platform_id, 2).await;
    let reconnect = receive_json(&mut second).await;
    assert_ne!(reconnect["connectionId"], first_connection);
    assert_eq!(reconnect["sequence"], 1);
    assert_eq!(reconnect["resourceRevision"], 2);
    assert_eq!(reconnect["eventKind"], "snapshot");
    hub.publish_container_stats(platform_id, &[]);
    let scoped_stats = receive_json(&mut second).await;
    assert_eq!(scoped_stats["eventKind"], "runtimeChanged");
    assert_eq!(
        scoped_stats["payload"]["dockerResourceType"],
        "containerStats"
    );
    assert_eq!(scoped_stats["payload"]["stats"], json!([]));
    second.close(None).await.unwrap();

    shutdown.cancel();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn global_subscription_receives_metadata_free_resource_invalidations() {
    let platform_id = Uuid::now_v7();
    let metrics = Arc::new(Metrics::default());
    let shutdown = CancellationToken::new();
    let service = RealtimeService::new(
        &config(),
        Arc::new(FakeReader {
            platform: platform(platform_id),
            allowed: Arc::new(AtomicBool::new(true)),
        }),
        metrics,
        shutdown.clone(),
    );
    let hub = service.hub();
    let (address, server) = start_server(service, shutdown.clone()).await;
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/phase0/realtime"))
            .await
            .unwrap();
    socket
        .send(Message::Text(
            json!({
                "protocolVersion": 1,
                "kind": "subscribe",
                "accessToken": TOKEN
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    assert_eq!(receive_json(&mut socket).await["kind"], "subscribed");

    hub.publish_resource_change("Registry", Uuid::nil(), "registryChanged");
    let event = receive_json(&mut socket).await;
    assert_eq!(event["resourceType"], "Registry");
    assert_eq!(event["resourceId"], Uuid::nil().to_string());
    assert_eq!(event["eventKind"], "registryChanged");
    assert_eq!(event["payload"], json!({}));

    for resource in [
        "Deployment",
        "Stack",
        "SwarmService",
        "Volume",
        "Network",
        "Build",
        "BackupPolicy",
        "AutomationAction",
        "Alert",
        "User",
        "Team",
        "Role",
        "ServiceAccount",
    ] {
        hub.publish_resource_change(resource, Uuid::now_v7(), "resourceChanged");
        let event = receive_json(&mut socket).await;
        assert_eq!(event["resourceType"], resource);
        assert_eq!(
            event["resourceId"],
            Uuid::nil().to_string(),
            "global invalidations must not disclose private IDs"
        );
        assert_eq!(event["payload"], json!({}));
    }

    socket.close(None).await.unwrap();
    shutdown.cancel();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn global_platform_updates_use_authorized_aggregates_and_stop_after_revocation() {
    let platform_id = Uuid::now_v7();
    let allowed = Arc::new(AtomicBool::new(true));
    let mut view = platform(platform_id);
    view.mem_total = 4096;
    view.stats = Some(vec![PlatformStatView {
        created: 123,
        cpu_usage: 12.5,
        memory_usage: 25.0,
        rx_bytes: 10.0,
        tx_bytes: 20.0,
        disk_used_bytes: None,
        disk_total_bytes: None,
        disk_usage: None,
    }]);
    let shutdown = CancellationToken::new();
    let service = RealtimeService::new(
        &config(),
        Arc::new(FakeReader {
            platform: view,
            allowed: allowed.clone(),
        }),
        Arc::new(Metrics::default()),
        shutdown.clone(),
    );
    let hub = service.hub();
    let (address, server) = start_server(service, shutdown.clone()).await;
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/phase0/realtime"))
            .await
            .unwrap();
    socket
        .send(Message::Text(
            json!({
                "protocolVersion": 1, "kind": "subscribe", "accessToken": TOKEN
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    assert_eq!(receive_json(&mut socket).await["kind"], "subscribed");

    hub.publish_container_stats(Uuid::now_v7(), &[]);
    hub.publish_container_stats(platform_id, &[]);
    let event = receive_json(&mut socket).await;
    assert_eq!(event["eventKind"], "platformStatsUpdated");
    assert_eq!(event["resourceId"], platform_id.to_string());
    assert_eq!(event["payload"]["stats"][0]["cpuUsage"], 12.5);
    assert_eq!(event["payload"]["stats"][0]["memoryUsage"], 25.0);
    assert_eq!(event["payload"]["memTotal"], 4096);
    assert!(event["payload"].get("runtimeResourceId").is_none());

    hub.publish_runtime_change(platform_id, "container", "start", "private-container-id");
    let event = receive_json(&mut socket).await;
    assert_eq!(event["eventKind"], "platformInventoryChanged");
    assert_eq!(event["payload"], json!({}));

    allowed.store(false, Ordering::Release);
    hub.publish_container_stats(platform_id, &[]);
    hub.publish_resource_change("Registry", Uuid::nil(), "registryChanged");
    // The sentinel proves the denied stats event was skipped, not merely delayed.
    assert_eq!(receive_json(&mut socket).await["resourceType"], "Registry");
    socket.close(None).await.unwrap();
    shutdown.cancel();
    server.await.unwrap().unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn global_subscription_streams_committed_platform_stats_after_store_recreation() {
    let database_url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let platform_id = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms (id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES ($1,$2,'Local',4,0,4096,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform_id).bind(format!("realtime-{platform_id}"))
        .execute(&pool).await.unwrap();
    for id in ["container-1", "container-2"] {
        sqlx::query("INSERT INTO containers (id,created,dockercontainerid,dockerimageid,name,platformid,ports,state,updated) VALUES ($1,1,$2,'image',$2,$3,'[]','running',1)")
            .bind(Uuid::now_v7()).bind(id).bind(platform_id).execute(&pool).await.unwrap();
    }
    let shutdown = CancellationToken::new();
    let reader = PersistedReader {
        authentication: FakeReader {
            platform: platform(platform_id),
            allowed: Arc::new(AtomicBool::new(true)),
        },
        store: PostgresPlatformReadStore::new(pool.clone()),
    };
    let service = RealtimeService::new(
        &config(),
        Arc::new(reader),
        Arc::new(Metrics::default()),
        shutdown.clone(),
    );
    let hub = service.hub();
    let (address, server) = start_server(service, shutdown.clone()).await;
    for revision in 1..=2 {
        // A new writer and connection still see persisted aggregates, not connection-local history.
        let (mut socket, _) =
            tokio_tungstenite::connect_async(format!("ws://{address}/phase0/realtime"))
                .await
                .unwrap();
        socket
            .send(Message::Text(
                json!({
                    "protocolVersion": 1, "kind": "subscribe", "accessToken": TOKEN
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
        assert_eq!(receive_json(&mut socket).await["kind"], "subscribed");
        let created = chrono::Utc::now().timestamp() + revision;
        let stats: Vec<_> = ["container-1", "container-2"]
            .into_iter()
            .map(|id| RuntimeContainerStat {
                docker_container_id: id.into(),
                created,
                cpu_usage: 25.0 * revision as f64,
                memory_active: 512.0,
                memory_cache: 0.0,
                memory_limit: 4096.0,
                rx_bytes: 10.0,
                tx_bytes: 20.0,
            })
            .collect();
        assert_eq!(
            PostgresContainerStatsStore::new(pool.clone())
                .persist(platform_id, &stats)
                .await
                .unwrap(),
            2
        );
        hub.publish_container_stats(platform_id, &stats);
        let event = receive_json(&mut socket).await;
        assert_eq!(event["eventKind"], "platformStatsUpdated");
        assert_eq!(event["payload"]["stats"][0]["created"], created);
        assert_eq!(
            event["payload"]["stats"][0]["cpuUsage"],
            12.5 * revision as f64
        );
        assert_eq!(event["payload"]["stats"][0]["memoryUsage"], 25.0);
        assert_eq!(event["payload"]["stats"].as_array().unwrap().len(), 1);
        socket.close(None).await.unwrap();
    }
    shutdown.cancel();
    server.await.unwrap().unwrap();
    sqlx::query("DELETE FROM platforms WHERE id = $1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
async fn invalid_token_is_rejected_before_any_snapshot() {
    let platform_id = Uuid::now_v7();
    let metrics = Arc::new(Metrics::default());
    let shutdown = CancellationToken::new();
    let service = RealtimeService::new(
        &config(),
        Arc::new(FakeReader {
            platform: platform(platform_id),
            allowed: Arc::new(AtomicBool::new(true)),
        }),
        Arc::clone(&metrics),
        shutdown.clone(),
    );
    let (address, server) = start_server(service, shutdown.clone()).await;
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/phase0/realtime"))
            .await
            .unwrap();
    socket
        .send(Message::Text(
            json!({
                "protocolVersion": 1,
                "kind": "subscribe",
                "accessToken": "wrong-token",
                "resourceType": "Platform",
                "resourceId": platform_id,
                "lastResourceRevision": 0
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    assert!(matches!(
        socket.next().await,
        Some(Ok(Message::Close(_))) | None
    ));
    tokio::time::timeout(Duration::from_secs(1), async {
        while metrics.realtime_active_connections() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    shutdown.cancel();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn bounded_hub_reports_lag_instead_of_retaining_every_event() {
    let metrics = Arc::new(Metrics::default());
    let platform_id = Uuid::now_v7();
    let hub = RealtimeHub::new(2, metrics);
    let mut receiver = hub.subscribe();
    hub.publish_runtime_change(platform_id, "container", "one", "1");
    hub.publish_runtime_change(platform_id, "container", "two", "2");
    hub.publish_runtime_change(platform_id, "container", "three", "3");

    assert!(matches!(
        receiver.recv().await,
        Err(broadcast::error::RecvError::Lagged(1))
    ));
}

#[tokio::test]
async fn connection_limit_rejects_excess_clients_before_allocating_a_subscription() {
    let platform_id = Uuid::now_v7();
    let mut realtime_config = config();
    realtime_config.max_connections = 1;
    let shutdown = CancellationToken::new();
    let service = RealtimeService::new(
        &realtime_config,
        Arc::new(FakeReader {
            platform: platform(platform_id),
            allowed: Arc::new(AtomicBool::new(true)),
        }),
        Arc::new(Metrics::default()),
        shutdown.clone(),
    );
    let (address, server) = start_server(service, shutdown.clone()).await;
    let (mut first, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/phase0/realtime"))
            .await
            .unwrap();

    let second = tokio_tungstenite::connect_async(format!("ws://{address}/phase0/realtime"))
        .await
        .unwrap_err();
    assert!(matches!(
        second,
        WebSocketError::Http(response)
            if response.status() == axum::http::StatusCode::SERVICE_UNAVAILABLE
    ));

    first.close(None).await.unwrap();
    shutdown.cancel();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn periodic_authorization_recheck_disconnects_a_revoked_actor() {
    let platform_id = Uuid::now_v7();
    let allowed = Arc::new(AtomicBool::new(true));
    let mut realtime_config = config();
    realtime_config.authorization_recheck_interval = Duration::from_millis(20);
    let shutdown = CancellationToken::new();
    let service = RealtimeService::new(
        &realtime_config,
        Arc::new(FakeReader {
            platform: platform(platform_id),
            allowed: Arc::clone(&allowed),
        }),
        Arc::new(Metrics::default()),
        shutdown.clone(),
    );
    let (address, server) = start_server(service, shutdown.clone()).await;
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/phase0/realtime"))
            .await
            .unwrap();
    subscribe(&mut socket, platform_id, 0).await;
    assert_eq!(receive_json(&mut socket).await["eventKind"], "snapshot");

    allowed.store(false, Ordering::Release);
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(1), socket.next())
            .await
            .unwrap(),
        Some(Ok(Message::Close(_))) | None
    ));

    shutdown.cancel();
    server.await.unwrap().unwrap();
}

async fn start_server(
    service: RealtimeService,
    shutdown: CancellationToken,
) -> (
    std::net::SocketAddr,
    tokio::task::JoinHandle<Result<(), std::io::Error>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, service.router())
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
    });
    (address, server)
}

async fn subscribe(
    socket: &mut tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    platform_id: Uuid,
    last_revision: u64,
) {
    socket
        .send(Message::Text(
            json!({
                "protocolVersion": 1,
                "kind": "subscribe",
                "accessToken": TOKEN,
                "resourceType": "Platform",
                "resourceId": platform_id,
                "lastResourceRevision": last_revision
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
}

async fn receive_json(
    socket: &mut tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
) -> Value {
    let message = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let Message::Text(text) = message else {
        panic!("expected text message, got {message:?}");
    };
    serde_json::from_str(text.as_str()).unwrap()
}

fn config() -> RealtimeConfig {
    RealtimeConfig {
        queue_capacity: 4,
        max_connections: 4,
        subscribe_timeout: Duration::from_secs(1),
        send_timeout: Duration::from_secs(1),
        authorization_recheck_interval: Duration::from_secs(60),
        snapshot_limit: 16,
    }
}

fn platform(id: Uuid) -> PlatformView {
    PlatformView {
        id,
        name: "fixture".to_owned(),
        description: None,
        address: "unix:///var/run/docker.sock".to_owned(),
        network_count: 0,
        volume_count: 0,
        image_count: 0,
        cpu_count: 0,
        mem_total: 0,
        agent_version: None,
        server_version: None,
        platform_type: "Docker".to_owned(),
        status: "Healthy".to_owned(),
        connector_type: "Local".to_owned(),
        deployment_count: 0,
        stack_count: 0,
        deployment_status_counts: WorkloadStatusCounts::default(),
        stack_status_counts: WorkloadStatusCounts::default(),
        swarm_service_status_counts: WorkloadStatusCounts::default(),
        stats: None,
        platform_descriptor: json!({"$type":"Docker"}),
        cluster_id: None,
        prune_historical_swarm_task_containers: true,
        capabilities: Some(PlatformCapabilitiesView::default()),
    }
}

struct FakeReader {
    platform: PlatformView,
    allowed: Arc<AtomicBool>,
}

struct PersistedReader {
    authentication: FakeReader,
    store: PostgresPlatformReadStore,
}

impl RealtimeReadPort for PersistedReader {
    fn authenticate<'a>(
        &'a self,
        token: &'a str,
    ) -> BoxFuture<'a, Result<ActorPrincipal, RealtimeReadError>> {
        self.authentication.authenticate(token)
    }

    fn authorize_platform<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        platform_id: Uuid,
    ) -> BoxFuture<'a, Result<PlatformView, RealtimeReadError>> {
        Box::pin(async move {
            self.authentication
                .authorize_platform(principal, platform_id)
                .await?;
            self.store
                .get_platform(platform_id)
                .await
                .map_err(|error| RealtimeReadError::Storage(error.to_string()))?
                .ok_or(RealtimeReadError::Authorization)
        })
    }

    fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerView>, RealtimeReadError>> {
        self.authentication.list_containers(platform_id)
    }
}

impl RealtimeReadPort for FakeReader {
    fn authenticate<'a>(
        &'a self,
        token: &'a str,
    ) -> BoxFuture<'a, Result<ActorPrincipal, RealtimeReadError>> {
        Box::pin(async move {
            if token == TOKEN {
                Ok(ActorPrincipal {
                    subject_id: Uuid::now_v7(),
                    actor_id: ActorId::new(Uuid::now_v7()),
                    name: "fixture".to_owned(),
                    principal_type: AuthenticatedPrincipalType::User,
                    credential_id: None,
                    roles: vec!["Admin".to_owned()],
                })
            } else {
                Err(RealtimeReadError::Authentication)
            }
        })
    }

    fn authorize_platform<'a>(
        &'a self,
        _principal: &'a ActorPrincipal,
        platform_id: Uuid,
    ) -> BoxFuture<'a, Result<PlatformView, RealtimeReadError>> {
        Box::pin(async move {
            if self.allowed.load(Ordering::Acquire) && platform_id == self.platform.id {
                Ok(self.platform.clone())
            } else {
                Err(RealtimeReadError::Authorization)
            }
        })
    }

    fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerView>, RealtimeReadError>> {
        Box::pin(async move {
            Ok(vec![ContainerView {
                image_view: None,
                deployment_view: None,
                id: Uuid::now_v7(),
                platform_id,
                container_id: "container-1".to_owned(),
                name: "fixture".to_owned(),
                docker_image_id: "sha256:fixture".to_owned(),
                created: 1,
                state: "running".to_owned(),
                control_state: "Idle".to_owned(),
                updated: 1,
                stack: None,
                is_system: false,
                system_role: None,
                has_citadel_ownership_labels: false,
                is_swarm_task: false,
                docker_node_id: None,
                node_hostname: None,
                projection_observed_at: None,
                projection_stale_since: None,
                projection_stale_reason: None,
                last_stats: None,
                ports: serde_json::Value::Null,
                deployment_id: None,
                stack_id: None,
                capabilities: None,
            }])
        })
    }
}
