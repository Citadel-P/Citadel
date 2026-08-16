use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use citadel_application::{
    AuthorizedPlatformReader, AuthorizedReadError, PlatformRuntimePort, RuntimeCapabilityError,
    RuntimeStatsStream,
};
use citadel_domain::{ActorId, PlatformSummary, RuntimeContainerSummary, RuntimePlatformInfo};
use citadel_server::config::RealtimeConfig;
use citadel_server::metrics::Metrics;
use citadel_server::realtime::{RealtimeHub, RealtimeService};
use futures_util::future::BoxFuture;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::broadcast;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const TOKEN: &str = "phase0c-test-token-with-at-least-32-characters";

#[tokio::test]
async fn subscription_sequences_events_and_resynchronizes_after_reconnect() {
    let platform_id = Uuid::now_v7();
    let actor_id = Uuid::now_v7();
    let allowed = Arc::new(AtomicBool::new(true));
    let reader = Arc::new(FakeReader {
        platform: platform(platform_id),
        allowed,
    });
    let runtime = Arc::new(FakeRuntime);
    let metrics = Arc::new(Metrics::default());
    let shutdown = CancellationToken::new();
    let service = RealtimeService::new(
        &config(actor_id, platform_id),
        reader,
        runtime,
        metrics,
        shutdown.clone(),
    );
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

    let revision = hub.publish_runtime_change("container", "start", "container-1");
    assert_eq!(revision, 1);
    let event = receive_json(&mut first).await;
    assert_eq!(event["connectionId"], first_connection);
    assert_eq!(event["sequence"], 2);
    assert_eq!(event["resourceRevision"], 1);
    assert_eq!(event["eventKind"], "runtimeChanged");

    first
        .send(Message::Text(
            json!({
                "protocolVersion": 1,
                "kind": "resync",
                "lastSequence": 2,
                "lastResourceRevision": 1
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    let resync = receive_json(&mut first).await;
    assert_eq!(resync["sequence"], 3);
    assert_eq!(resync["resourceRevision"], 1);
    assert_eq!(resync["eventKind"], "snapshot");
    first.close(None).await.unwrap();

    let (mut second, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/phase0/realtime"))
            .await
            .unwrap();
    subscribe(&mut second, platform_id, 1).await;
    let reconnect = receive_json(&mut second).await;
    assert_ne!(reconnect["connectionId"], first_connection);
    assert_eq!(reconnect["sequence"], 1);
    assert_eq!(reconnect["resourceRevision"], 1);
    assert_eq!(reconnect["eventKind"], "snapshot");
    second.close(None).await.unwrap();

    shutdown.cancel();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn invalid_token_is_rejected_before_any_snapshot() {
    let platform_id = Uuid::now_v7();
    let actor_id = Uuid::now_v7();
    let metrics = Arc::new(Metrics::default());
    let shutdown = CancellationToken::new();
    let service = RealtimeService::new(
        &config(actor_id, platform_id),
        Arc::new(FakeReader {
            platform: platform(platform_id),
            allowed: Arc::new(AtomicBool::new(true)),
        }),
        Arc::new(FakeRuntime),
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
    let hub = RealtimeHub::new(Uuid::now_v7(), 2, metrics);
    let mut receiver = hub.subscribe();
    hub.publish_runtime_change("container", "one", "1");
    hub.publish_runtime_change("container", "two", "2");
    hub.publish_runtime_change("container", "three", "3");

    assert!(matches!(
        receiver.recv().await,
        Err(broadcast::error::RecvError::Lagged(1))
    ));
}

#[tokio::test]
async fn connection_limit_rejects_excess_clients_before_allocating_a_subscription() {
    let platform_id = Uuid::now_v7();
    let actor_id = Uuid::now_v7();
    let mut realtime_config = config(actor_id, platform_id);
    realtime_config.max_connections = 1;
    let shutdown = CancellationToken::new();
    let service = RealtimeService::new(
        &realtime_config,
        Arc::new(FakeReader {
            platform: platform(platform_id),
            allowed: Arc::new(AtomicBool::new(true)),
        }),
        Arc::new(FakeRuntime),
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
    let actor_id = Uuid::now_v7();
    let allowed = Arc::new(AtomicBool::new(true));
    let mut realtime_config = config(actor_id, platform_id);
    realtime_config.authorization_recheck_interval = Duration::from_millis(20);
    let shutdown = CancellationToken::new();
    let service = RealtimeService::new(
        &realtime_config,
        Arc::new(FakeReader {
            platform: platform(platform_id),
            allowed: Arc::clone(&allowed),
        }),
        Arc::new(FakeRuntime),
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

fn config(actor_id: Uuid, platform_id: Uuid) -> RealtimeConfig {
    RealtimeConfig {
        actor_id,
        platform_id,
        token_hash: Sha256::digest(TOKEN.as_bytes()).into(),
        queue_capacity: 4,
        max_connections: 4,
        subscribe_timeout: Duration::from_secs(1),
        send_timeout: Duration::from_secs(1),
        authorization_recheck_interval: Duration::from_secs(60),
        snapshot_limit: 16,
    }
}

fn platform(id: Uuid) -> PlatformSummary {
    PlatformSummary {
        id,
        name: "fixture".to_owned(),
        address: "unix:///var/run/docker.sock".to_owned(),
        status: "Healthy".to_owned(),
        connector_type: "Local".to_owned(),
    }
}

struct FakeReader {
    platform: PlatformSummary,
    allowed: Arc<AtomicBool>,
}

impl AuthorizedPlatformReader for FakeReader {
    fn list_authorized<'a>(
        &'a self,
        _actor_id: ActorId,
    ) -> BoxFuture<'a, Result<Vec<PlatformSummary>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(if self.allowed.load(Ordering::Acquire) {
                vec![self.platform.clone()]
            } else {
                Vec::new()
            })
        })
    }
}

struct FakeRuntime;

impl PlatformRuntimePort for FakeRuntime {
    fn get_info<'a>(
        &'a self,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        unreachable!()
    }

    fn list_containers<'a>(
        &'a self,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
        Box::pin(async {
            Ok(vec![RuntimeContainerSummary {
                id: "container-1".to_owned(),
                name: "fixture".to_owned(),
                image: "fixture:latest".to_owned(),
                state: "running".to_owned(),
            }])
        })
    }

    fn stream_stats<'a>(
        &'a self,
        _fetch_interval: Duration,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeStatsStream, RuntimeCapabilityError>> {
        unreachable!()
    }
}
