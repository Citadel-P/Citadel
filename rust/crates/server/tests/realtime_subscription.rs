use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use citadel_domain::{ActorId, AuthenticatedPrincipalType};
use citadel_identity::ActorPrincipal;
use citadel_platforms::{
    ContainerView, PlatformCapabilitiesView, PlatformView, WorkloadStatusCounts,
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
    second.close(None).await.unwrap();

    shutdown.cancel();
    server.await.unwrap().unwrap();
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
