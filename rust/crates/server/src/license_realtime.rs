use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::Router;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use citadel_application::{
    LicenseStateNotifier, LicenseTransitionMonitor, LicenseValidationPersistence,
    license_transition_delay,
};
use citadel_identity::IdentityService;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::{Semaphore, broadcast};
use tokio::time::{Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::realtime::RealtimeHub;

const PROTOCOL_VERSION: u16 = 1;
const PAYLOAD_SCHEMA_VERSION: u16 = 1;
const EVENT_CAPACITY: usize = 16;
const MAX_CONNECTIONS: usize = 128;
const AUTHENTICATION_TIMEOUT: Duration = Duration::from_secs(10);
const SEND_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const AUTHORIZATION_RECHECK_INTERVAL: Duration = Duration::from_secs(5 * 60);
const MAX_CLIENT_MESSAGE_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy)]
struct LicenseStateChanged {
    instance_id: Uuid,
    revision: u64,
}

#[derive(Clone)]
pub struct LicenseRealtimeHub {
    revision: Arc<AtomicU64>,
    sender: broadcast::Sender<LicenseStateChanged>,
    realtime: Option<RealtimeHub>,
}

impl Default for LicenseRealtimeHub {
    fn default() -> Self {
        let (sender, receiver) = broadcast::channel(EVENT_CAPACITY);
        drop(receiver);
        Self {
            revision: Arc::new(AtomicU64::new(0)),
            sender,
            realtime: None,
        }
    }
}

impl LicenseRealtimeHub {
    #[must_use]
    pub fn with_realtime(mut self, realtime: Option<RealtimeHub>) -> Self {
        self.realtime = realtime;
        self
    }

    pub fn publish(&self, instance_id: Uuid) -> u64 {
        let revision = self.revision.fetch_add(1, Ordering::AcqRel) + 1;
        let _ = self.sender.send(LicenseStateChanged {
            instance_id,
            revision,
        });
        if let Some(realtime) = &self.realtime {
            realtime.publish_resource_change("License", instance_id, "licenseStateChanged");
        }
        revision
    }

    fn subscribe(&self) -> broadcast::Receiver<LicenseStateChanged> {
        self.sender.subscribe()
    }
}

impl LicenseStateNotifier for LicenseRealtimeHub {
    fn license_state_changed(&self, instance_id: Uuid) {
        self.publish(instance_id);
    }
}

#[derive(Clone)]
pub struct LicenseRealtimeService {
    identity: Arc<IdentityService>,
    hub: LicenseRealtimeHub,
    connections: Arc<Semaphore>,
    shutdown: CancellationToken,
}

impl LicenseRealtimeService {
    #[must_use]
    pub fn new(
        identity: Arc<IdentityService>,
        hub: LicenseRealtimeHub,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            identity,
            hub,
            connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
            shutdown,
        }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/api/v1/realtime", get(upgrade))
            .with_state(self)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SubscribeMessage {
    protocol_version: u16,
    kind: String,
    access_token: String,
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

async fn upgrade(State(service): State<LicenseRealtimeService>, ws: WebSocketUpgrade) -> Response {
    let Ok(permit) = Arc::clone(&service.connections).try_acquire_owned() else {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    ws.max_message_size(MAX_CLIENT_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            handle(socket, service).await;
        })
}

async fn handle(mut socket: WebSocket, service: LicenseRealtimeService) {
    let access_token = match authenticate(&mut socket, &service).await {
        Ok(access_token) => access_token,
        Err(()) => {
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    };
    let connection_id = Uuid::now_v7();
    let mut sequence = 1_u64;
    let mut receiver = service.hub.subscribe();
    if !matches!(
        tokio::time::timeout(
            SEND_TIMEOUT,
            socket.send(Message::Text(
                json!({ "protocolVersion": PROTOCOL_VERSION, "kind": "subscribed" })
                    .to_string()
                    .into(),
            )),
        )
        .await,
        Ok(Ok(()))
    ) {
        return;
    }
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);
    heartbeat.tick().await;
    let mut authorization_recheck = tokio::time::interval_at(
        Instant::now() + AUTHORIZATION_RECHECK_INTERVAL,
        AUTHORIZATION_RECHECK_INTERVAL,
    );
    authorization_recheck.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        let event = tokio::select! {
            () = service.shutdown.cancelled() => return,
            incoming = socket.next() => match incoming {
                Some(Ok(Message::Close(_))) | None => return,
                Some(Err(_)) => return,
                Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                Some(Ok(Message::Text(_))) | Some(Ok(Message::Binary(_))) => return,
            },
            _ = heartbeat.tick() => {
                if !matches!(
                    tokio::time::timeout(SEND_TIMEOUT, socket.send(Message::Ping(Vec::new().into()))).await,
                    Ok(Ok(()))
                ) {
                    return;
                }
                continue;
            },
            _ = authorization_recheck.tick() => {
                if service.identity.authenticate_bearer(&access_token).await.is_err() {
                    return;
                }
                continue;
            },
            event = receiver.recv() => match event {
                Ok(event) => event,
                Err(broadcast::error::RecvError::Lagged(_)) | Err(broadcast::error::RecvError::Closed) => return,
            },
        };
        let payload = json!({});
        let envelope = RealtimeEnvelope {
            protocol_version: PROTOCOL_VERSION,
            connection_id,
            sequence,
            resource_type: "License",
            resource_id: event.instance_id,
            resource_revision: event.revision,
            event_kind: "licenseStateChanged",
            payload_schema_version: PAYLOAD_SCHEMA_VERSION,
            payload: &payload,
        };
        sequence = sequence.saturating_add(1);
        let Ok(message) = serde_json::to_string(&envelope) else {
            return;
        };
        if !matches!(
            tokio::time::timeout(SEND_TIMEOUT, socket.send(Message::Text(message.into()))).await,
            Ok(Ok(()))
        ) {
            return;
        }
    }
}

async fn authenticate(
    socket: &mut WebSocket,
    service: &LicenseRealtimeService,
) -> Result<String, ()> {
    let message = tokio::time::timeout(AUTHENTICATION_TIMEOUT, socket.next())
        .await
        .map_err(|_| ())?
        .ok_or(())?
        .map_err(|_| ())?;
    let Message::Text(message) = message else {
        return Err(());
    };
    let subscribe: SubscribeMessage = serde_json::from_str(message.as_str()).map_err(|_| ())?;
    if subscribe.protocol_version != PROTOCOL_VERSION || subscribe.kind != "subscribe" {
        return Err(());
    }
    service
        .identity
        .authenticate_bearer(&subscribe.access_token)
        .await
        .map_err(|_| ())?;
    Ok(subscribe.access_token)
}

pub async fn run_license_transition_monitor(
    cancellation: CancellationToken,
    monitor: LicenseTransitionMonitor,
    hub: LicenseRealtimeHub,
) -> Result<(), Infallible> {
    let mut wake = hub.subscribe();
    loop {
        let now = chrono::Utc::now();
        let delay = match monitor.check_once().await {
            Ok(check) => {
                if check.persistence == LicenseValidationPersistence::Transitioned {
                    // Persistence commits before check_once returns, so the notification can
                    // never advertise a state that was rolled back.
                    if let Some(instance_id) = check.instance_id {
                        hub.publish(instance_id);
                    }
                }
                license_transition_delay(now, check.next_boundary)
            }
            Err(error) => {
                tracing::error!(%error, "license transition check failed");
                license_transition_delay(now, None)
            }
        };
        tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            () = tokio::time::sleep(delay) => {}
            _ = wake.recv() => {}
        }
    }
}
