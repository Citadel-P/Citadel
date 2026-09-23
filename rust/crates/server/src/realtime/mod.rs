use crate::{config::RealtimeConfig, metrics::Metrics};
use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

mod authorization;
mod connection;
mod groups;
mod hub;
mod invocation;
mod protocol;
mod subscription;
pub mod topic;

pub use authorization::{IdentityRealtimeReader, RealtimeReadError, RealtimeReadPort};
use connection::handle_connection;
pub use hub::{
    PublishedRuntimeEvent, RealtimeHub, build_log_callback, change_callback, notify_mutations,
};

pub const REALTIME_PROTOCOL_VERSION: u16 = 1;
const PAYLOAD_SCHEMA_VERSION: u16 = 1;
const PLATFORM_RESOURCE_TYPE: &str = "Platform";
const MAX_CLIENT_MESSAGE_BYTES: usize = 16 * 1024;
const WRITE_BUFFER_BYTES: usize = 16 * 1024;
const MAX_WRITE_BUFFER_BYTES: usize = 128 * 1024;

#[derive(Clone)]
pub struct RealtimeService {
    inner: Arc<RealtimeServiceInner>,
}

struct RealtimeServiceInner {
    groups: Option<Arc<dyn crate::realtime_groups::GroupReadPort>>,
    subscribe_timeout: Duration,
    send_timeout: Duration,
    authorization_recheck_interval: Duration,
    snapshot_limit: usize,
    outbound_capacity: usize,
    connection_slots: Arc<Semaphore>,
    reader: Arc<dyn RealtimeReadPort>,
    hub: RealtimeHub,
    metrics: Arc<Metrics>,
    shutdown: CancellationToken,
}

impl RealtimeService {
    #[must_use]
    pub fn new(
        config: &RealtimeConfig,
        reader: Arc<dyn RealtimeReadPort>,
        metrics: Arc<Metrics>,
        shutdown: CancellationToken,
    ) -> Self {
        let hub = RealtimeHub::new(config.queue_capacity, Arc::clone(&metrics));
        Self::with_hub(config, reader, metrics, shutdown, hub)
    }

    #[must_use]
    pub fn with_hub(
        config: &RealtimeConfig,
        reader: Arc<dyn RealtimeReadPort>,
        metrics: Arc<Metrics>,
        shutdown: CancellationToken,
        hub: RealtimeHub,
    ) -> Self {
        Self {
            inner: Arc::new(RealtimeServiceInner {
                groups: None,
                subscribe_timeout: config.subscribe_timeout,
                send_timeout: config.send_timeout,
                authorization_recheck_interval: config.authorization_recheck_interval,
                snapshot_limit: config.snapshot_limit,
                outbound_capacity: config.queue_capacity,
                connection_slots: Arc::new(Semaphore::new(config.max_connections)),
                reader,
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

    pub fn with_groups(mut self, reader: Arc<dyn crate::realtime_groups::GroupReadPort>) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("configure groups before sharing the realtime service")
            .groups = Some(reader);
        self
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/api/v1/realtime", get(upgrade))
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

#[derive(Debug, thiserror::Error)]
enum RealtimeError {
    #[error("the client did not subscribe before the deadline")]
    SubscribeTimeout,
    #[error("the client closed before subscribing")]
    Closed,
    #[error("realtime outbound queue capacity exceeded")]
    Overloaded,
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
    #[error("realtime socket failed: {0}")]
    Socket(String),
}

pub mod notifiers;
