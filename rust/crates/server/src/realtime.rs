use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::Router;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use citadel_domain::ActorId;
use citadel_platforms::{AuthorizedPlatformReader, PlatformRuntimePort};
use citadel_platforms::{PlatformSummary, RuntimeContainerSummary};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, broadcast};
use tokio::time::{Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config::RealtimeConfig;
use crate::metrics::Metrics;

pub const REALTIME_PROTOCOL_VERSION: u16 = 1;
const PAYLOAD_SCHEMA_VERSION: u16 = 1;
const PLATFORM_RESOURCE_TYPE: &str = "Platform";
const MAX_CLIENT_MESSAGE_BYTES: usize = 16 * 1024;
const WRITE_BUFFER_BYTES: usize = 16 * 1024;
const MAX_WRITE_BUFFER_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone)]
pub struct PublishedRuntimeEvent {
    resource_revision: u64,
    payload: Value,
}

#[derive(Clone)]
pub struct RealtimeHub {
    inner: Arc<RealtimeHubInner>,
}

struct RealtimeHubInner {
    platform_id: Uuid,
    revision: AtomicU64,
    sender: broadcast::Sender<Arc<PublishedRuntimeEvent>>,
    metrics: Arc<Metrics>,
}

impl RealtimeHub {
    #[must_use]
    pub fn new(platform_id: Uuid, capacity: usize, metrics: Arc<Metrics>) -> Self {
        let (sender, receiver) = broadcast::channel(capacity);
        drop(receiver);
        Self {
            inner: Arc::new(RealtimeHubInner {
                platform_id,
                revision: AtomicU64::new(0),
                sender,
                metrics,
            }),
        }
    }

    #[must_use]
    pub fn platform_id(&self) -> Uuid {
        self.inner.platform_id
    }

    #[must_use]
    pub fn current_revision(&self) -> u64 {
        self.inner.revision.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<PublishedRuntimeEvent>> {
        self.inner.sender.subscribe()
    }

    pub fn publish_runtime_change(
        &self,
        docker_resource_type: impl Into<String>,
        action: impl Into<String>,
        runtime_resource_id: impl Into<String>,
    ) -> u64 {
        let revision = self.inner.revision.fetch_add(1, Ordering::AcqRel) + 1;
        let event = Arc::new(PublishedRuntimeEvent {
            resource_revision: revision,
            payload: json!({
                "dockerResourceType": docker_resource_type.into(),
                "action": action.into(),
                "runtimeResourceId": runtime_resource_id.into(),
            }),
        });
        let _ = self.inner.sender.send(event);
        self.inner.metrics.realtime_event_published();
        revision
    }
}

#[derive(Clone)]
pub struct RealtimeService {
    inner: Arc<RealtimeServiceInner>,
}

struct RealtimeServiceInner {
    actor_id: ActorId,
    platform_id: Uuid,
    token_hash: [u8; 32],
    subscribe_timeout: Duration,
    send_timeout: Duration,
    authorization_recheck_interval: Duration,
    snapshot_limit: usize,
    connection_slots: Arc<Semaphore>,
    reader: Arc<dyn AuthorizedPlatformReader>,
    runtime: Arc<dyn PlatformRuntimePort>,
    hub: RealtimeHub,
    metrics: Arc<Metrics>,
    shutdown: CancellationToken,
}

impl RealtimeService {
    #[must_use]
    pub fn new(
        config: &RealtimeConfig,
        reader: Arc<dyn AuthorizedPlatformReader>,
        runtime: Arc<dyn PlatformRuntimePort>,
        metrics: Arc<Metrics>,
        shutdown: CancellationToken,
    ) -> Self {
        let hub = RealtimeHub::new(
            config.platform_id,
            config.queue_capacity,
            Arc::clone(&metrics),
        );
        Self {
            inner: Arc::new(RealtimeServiceInner {
                actor_id: ActorId::new(config.actor_id),
                platform_id: config.platform_id,
                token_hash: config.token_hash,
                subscribe_timeout: config.subscribe_timeout,
                send_timeout: config.send_timeout,
                authorization_recheck_interval: config.authorization_recheck_interval,
                snapshot_limit: config.snapshot_limit,
                connection_slots: Arc::new(Semaphore::new(config.max_connections)),
                reader,
                runtime,
                hub,
                metrics,
                shutdown,
            }),
        }
    }

    #[must_use]
    pub fn hub(&self) -> RealtimeHub {
        self.inner.hub.clone()
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/phase0/realtime", get(upgrade))
            .with_state(self)
    }
}

async fn upgrade(State(service): State<RealtimeService>, ws: WebSocketUpgrade) -> Response {
    let permit = match Arc::clone(&service.inner.connection_slots).try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Realtime connection capacity is exhausted.",
            )
                .into_response();
        }
    };
    ws.max_message_size(MAX_CLIENT_MESSAGE_BYTES)
        .write_buffer_size(WRITE_BUFFER_BYTES)
        .max_write_buffer_size(MAX_WRITE_BUFFER_BYTES)
        .on_upgrade(move |socket| handle_connection(socket, service, permit))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientMessage {
    protocol_version: u16,
    kind: String,
    access_token: Option<String>,
    resource_type: Option<String>,
    resource_id: Option<Uuid>,
    last_sequence: Option<u64>,
    last_resource_revision: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RealtimeEnvelope<'a> {
    protocol_version: u16,
    connection_id: Uuid,
    sequence: u64,
    resource_type: &'static str,
    resource_id: Uuid,
    resource_revision: u64,
    event_kind: &'static str,
    payload_schema_version: u16,
    payload: &'a Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformSnapshot {
    platform: PlatformSummary,
    containers: Vec<RuntimeContainerSummary>,
}

struct ConnectionState {
    id: Uuid,
    next_sequence: u64,
    last_resource_revision: u64,
}

impl ConnectionState {
    fn new() -> Self {
        Self {
            id: Uuid::now_v7(),
            next_sequence: 1,
            last_resource_revision: 0,
        }
    }

    fn take_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        sequence
    }
}

#[derive(Debug, thiserror::Error)]
enum RealtimeError {
    #[error("the client did not subscribe before the deadline")]
    SubscribeTimeout,
    #[error("the client closed before subscribing")]
    Closed,
    #[error("invalid realtime message: {0}")]
    InvalidMessage(String),
    #[error("realtime protocol version {0} is unsupported")]
    UnsupportedProtocol(u16),
    #[error("realtime authentication failed")]
    Authentication,
    #[error("the actor cannot read the requested Platform")]
    Authorization,
    #[error("authorization lookup failed: {0}")]
    AuthorizationStorage(String),
    #[error("runtime snapshot failed: {0}")]
    Snapshot(String),
    #[error("runtime snapshot contains {actual} containers; limit is {limit}")]
    SnapshotLimit { actual: usize, limit: usize },
    #[error("realtime serialization failed: {0}")]
    Serialization(String),
    #[error("realtime client write timed out")]
    SendTimeout,
    #[error("realtime socket failed: {0}")]
    Socket(String),
}

async fn handle_connection(
    mut socket: WebSocket,
    service: RealtimeService,
    _permit: OwnedSemaphorePermit,
) {
    let _connection = service.inner.metrics.realtime_connection_guard();
    let cancellation = service.inner.shutdown.child_token();
    let result = run_connection(&mut socket, &service, &cancellation).await;
    cancellation.cancel();
    if let Err(error) = result {
        tracing::debug!(%error, "realtime connection closed");
        let _ = tokio::time::timeout(
            service.inner.send_timeout,
            socket.send(Message::Close(None)),
        )
        .await;
    }
}

async fn run_connection(
    socket: &mut WebSocket,
    service: &RealtimeService,
    cancellation: &CancellationToken,
) -> Result<(), RealtimeError> {
    let subscribe = receive_initial_subscription(socket, service.inner.subscribe_timeout).await?;
    validate_subscription(service, subscribe)?;

    let mut connection = ConnectionState::new();
    let mut receiver = service.inner.hub.subscribe();
    send_snapshot(socket, service, cancellation, &mut connection).await?;

    let mut authorization_recheck = tokio::time::interval_at(
        Instant::now() + service.inner.authorization_recheck_interval,
        service.inner.authorization_recheck_interval,
    );
    authorization_recheck.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            message = socket.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        let message = parse_client_message(text.as_str())?;
                        if message.protocol_version != REALTIME_PROTOCOL_VERSION {
                            return Err(RealtimeError::UnsupportedProtocol(message.protocol_version));
                        }
                        if message.kind != "resync" {
                            return Err(RealtimeError::InvalidMessage("only resync is accepted after subscription".to_owned()));
                        }
                        let last_sequence = message.last_sequence.ok_or_else(|| RealtimeError::InvalidMessage("lastSequence is required for resync".to_owned()))?;
                        let last_revision = message.last_resource_revision.ok_or_else(|| RealtimeError::InvalidMessage("lastResourceRevision is required for resync".to_owned()))?;
                        if last_sequence >= connection.next_sequence || last_revision > service.inner.hub.current_revision() {
                            return Err(RealtimeError::InvalidMessage("resync cursor is ahead of the server".to_owned()));
                        }
                        receiver = service.inner.hub.subscribe();
                        send_snapshot(socket, service, cancellation, &mut connection).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => return Ok(()),
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Binary(_))) => return Err(RealtimeError::InvalidMessage("binary client messages are unsupported".to_owned())),
                    Some(Err(error)) => return Err(RealtimeError::Socket(error.to_string())),
                }
            }
            _ = authorization_recheck.tick() => {
                authorize_platform(service).await?;
            }
            event = receiver.recv() => {
                match event {
                    Ok(event) if event.resource_revision > connection.last_resource_revision => {
                        send_envelope(
                            socket,
                            service,
                            &mut connection,
                            event.resource_revision,
                            "runtimeChanged",
                            &event.payload,
                        ).await?;
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        service.inner.metrics.realtime_overflowed();
                        let payload = json!({ "reason": "queueOverflow", "missedEvents": missed });
                        send_envelope(
                            socket,
                            service,
                            &mut connection,
                            service.inner.hub.current_revision(),
                            "resyncRequired",
                            &payload,
                        ).await?;
                        receiver = service.inner.hub.subscribe();
                        send_snapshot(socket, service, cancellation, &mut connection).await?;
                    }
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                }
            }
        }
    }
}

async fn receive_initial_subscription(
    socket: &mut WebSocket,
    timeout: Duration,
) -> Result<ClientMessage, RealtimeError> {
    let message = tokio::time::timeout(timeout, socket.next())
        .await
        .map_err(|_| RealtimeError::SubscribeTimeout)?
        .ok_or(RealtimeError::Closed)?
        .map_err(|error| RealtimeError::Socket(error.to_string()))?;
    match message {
        Message::Text(text) => parse_client_message(text.as_str()),
        Message::Close(_) => Err(RealtimeError::Closed),
        _ => Err(RealtimeError::InvalidMessage(
            "the first message must be a text subscription".to_owned(),
        )),
    }
}

fn parse_client_message(text: &str) -> Result<ClientMessage, RealtimeError> {
    serde_json::from_str(text).map_err(|error| RealtimeError::InvalidMessage(error.to_string()))
}

fn validate_subscription(
    service: &RealtimeService,
    subscribe: ClientMessage,
) -> Result<(), RealtimeError> {
    if subscribe.protocol_version != REALTIME_PROTOCOL_VERSION {
        return Err(RealtimeError::UnsupportedProtocol(
            subscribe.protocol_version,
        ));
    }
    if subscribe.kind != "subscribe"
        || subscribe.resource_type.as_deref() != Some(PLATFORM_RESOURCE_TYPE)
        || subscribe.resource_id != Some(service.inner.platform_id)
    {
        return Err(RealtimeError::InvalidMessage(
            "only the configured Platform subscription is supported".to_owned(),
        ));
    }
    let Some(token) = subscribe.access_token.as_deref() else {
        service.inner.metrics.realtime_authorization_failed();
        return Err(RealtimeError::Authentication);
    };
    let actual_hash: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    if !bool::from(service.inner.token_hash.ct_eq(&actual_hash)) {
        service.inner.metrics.realtime_authorization_failed();
        return Err(RealtimeError::Authentication);
    }
    Ok(())
}

async fn authorize_platform(service: &RealtimeService) -> Result<PlatformSummary, RealtimeError> {
    let platforms = service
        .inner
        .reader
        .list_authorized(service.inner.actor_id)
        .await
        .map_err(|error| RealtimeError::AuthorizationStorage(error.to_string()))?;
    platforms
        .into_iter()
        .find(|platform| platform.id == service.inner.platform_id)
        .ok_or_else(|| {
            service.inner.metrics.realtime_authorization_failed();
            RealtimeError::Authorization
        })
}

async fn send_snapshot(
    socket: &mut WebSocket,
    service: &RealtimeService,
    cancellation: &CancellationToken,
    connection: &mut ConnectionState,
) -> Result<(), RealtimeError> {
    let snapshot_revision = service.inner.hub.current_revision();
    let platform = authorize_platform(service).await?;
    let containers = service
        .inner
        .runtime
        .list_containers(cancellation)
        .await
        .map_err(|error| RealtimeError::Snapshot(error.to_string()))?;
    if containers.len() > service.inner.snapshot_limit {
        return Err(RealtimeError::SnapshotLimit {
            actual: containers.len(),
            limit: service.inner.snapshot_limit,
        });
    }
    let payload = serde_json::to_value(PlatformSnapshot {
        platform,
        containers,
    })
    .map_err(|error| RealtimeError::Serialization(error.to_string()))?;
    send_envelope(
        socket,
        service,
        connection,
        snapshot_revision,
        "snapshot",
        &payload,
    )
    .await?;
    service.inner.metrics.realtime_snapshot_resynced();
    Ok(())
}

async fn send_envelope(
    socket: &mut WebSocket,
    service: &RealtimeService,
    connection: &mut ConnectionState,
    resource_revision: u64,
    event_kind: &'static str,
    payload: &Value,
) -> Result<(), RealtimeError> {
    let envelope = RealtimeEnvelope {
        protocol_version: REALTIME_PROTOCOL_VERSION,
        connection_id: connection.id,
        sequence: connection.take_sequence(),
        resource_type: PLATFORM_RESOURCE_TYPE,
        resource_id: service.inner.platform_id,
        resource_revision,
        event_kind,
        payload_schema_version: PAYLOAD_SCHEMA_VERSION,
        payload,
    };
    let text = serde_json::to_string(&envelope)
        .map_err(|error| RealtimeError::Serialization(error.to_string()))?;
    match tokio::time::timeout(
        service.inner.send_timeout,
        socket.send(Message::Text(text.into())),
    )
    .await
    {
        Ok(Ok(())) => {
            connection.last_resource_revision = resource_revision;
            service.inner.metrics.realtime_message_sent();
            Ok(())
        }
        Ok(Err(error)) => Err(RealtimeError::Socket(error.to_string())),
        Err(_) => {
            service.inner.metrics.realtime_send_timed_out();
            Err(RealtimeError::SendTimeout)
        }
    }
}
