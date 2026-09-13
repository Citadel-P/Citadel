use super::connection::{ConnectionIo, receive_initial_subscription, send_text};
use super::groups::run_group_connection;
use super::{
    PAYLOAD_SCHEMA_VERSION, PLATFORM_RESOURCE_TYPE, PublishedRuntimeEvent,
    REALTIME_PROTOCOL_VERSION, RealtimeError, RealtimeService,
    authorization::{authorize_platform, map_realtime_read_error, validate_subscription},
    protocol::{PlatformSnapshot, RealtimeEnvelope, RealtimeEventRef, parse_client_message},
};
use axum::extract::ws::Message;
use citadel_identity::ActorPrincipal;
use serde_json::json;
use tokio::{
    sync::broadcast,
    time::{Instant, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct ConnectionState {
    id: Uuid,
    platform_id: Option<Uuid>,
    next_sequence: u64,
    last_resource_revision: u64,
}

impl ConnectionState {
    fn new(id: Uuid, platform_id: Option<Uuid>) -> Self {
        Self {
            id,
            platform_id,
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

pub(super) async fn run_connection(
    socket: &mut ConnectionIo,
    service: &RealtimeService,
    cancellation: &CancellationToken,
) -> Result<(), RealtimeError> {
    let subscribe = receive_initial_subscription(socket, service.inner.subscribe_timeout).await?;
    let groups_mode = subscribe.client_mode.as_deref() == Some("groups");
    let mut subscription = validate_subscription(service, subscribe).await?;

    if groups_mode {
        return run_group_connection(socket, service, subscription, cancellation).await;
    }

    let mut connection = ConnectionState::new(socket.id, subscription.platform_id);
    let mut receiver = service.inner.hub.subscribe();
    send_snapshot(socket, service, &subscription.principal, &mut connection).await?;

    let mut authorization_recheck = tokio::time::interval_at(
        Instant::now() + service.inner.authorization_recheck_interval,
        service.inner.authorization_recheck_interval,
    );
    authorization_recheck.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
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
                        send_snapshot(socket, service, &subscription.principal, &mut connection).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => return Ok(()),
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Binary(_))) => return Err(RealtimeError::InvalidMessage("binary client messages are unsupported".to_owned())),
                    Some(Err(error)) => return Err(RealtimeError::Socket(error.to_string())),
                }
            }
            _ = authorization_recheck.tick() => {
                let principal = service.inner.reader.authenticate(&subscription.access_token).await
                    .map_err(map_realtime_read_error)?;
                if let Some(platform_id) = subscription.platform_id {
                    authorize_platform(service, &principal, platform_id).await?;
                }
                subscription.principal = principal;
            }
            event = receiver.recv() => {
                match event {
                    Ok(event) if event_matches(&event, &connection)
                        && event.resource_revision > connection.last_resource_revision => {
                        // The global UI subscription can see only authorized Platform summaries,
                        // never the container statistics carried by a scoped runtime event.
                        let mut event_kind = event.event_kind;
                        let mut payload = &event.payload;
                        let platform_payload;
                        if connection.platform_id.is_none()
                            && let Some(platform_id) = event.platform_id
                        {
                            let platform = match authorize_platform(service, &subscription.principal, platform_id).await {
                                Ok(platform) => platform,
                                Err(RealtimeError::Authorization) => continue,
                                Err(error) => return Err(error),
                            };
                            if event.payload["dockerResourceType"] == "containerStats" {
                                event_kind = "platformStatsUpdated";
                                platform_payload = json!({
                                    "platformId": platform.id,
                                    "stats": platform.stats,
                                    "memTotal": platform.mem_total,
                                });
                            } else {
                                event_kind = "platformInventoryChanged";
                                platform_payload = json!({});
                            }
                            payload = &platform_payload;
                        }
                        let resource_id = if event.platform_id.is_none() { Uuid::nil() } else { event.resource_id };
                        send_envelope(
                            socket,
                            service,
                            &mut connection,
                            RealtimeEventRef {
                                resource_revision: event.resource_revision,
                                resource_type: event.resource_type,
                                resource_id,
                                event_kind,
                                payload,
                            },
                        ).await?;
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        service.inner.metrics.realtime_overflowed();
                        let payload = json!({ "reason": "queueOverflow", "missedEvents": missed });
                        let resource_type = connection_resource_type(&connection);
                        let resource_id = connection.platform_id.unwrap_or(Uuid::nil());
                        send_envelope(
                            socket,
                            service,
                            &mut connection,
                            RealtimeEventRef {
                                resource_revision: service.inner.hub.current_revision(),
                                resource_type,
                                resource_id,
                                event_kind: "resyncRequired",
                                payload: &payload,
                            },
                        ).await?;
                        receiver = service.inner.hub.subscribe();
                        send_snapshot(socket, service, &subscription.principal, &mut connection).await?;
                    }
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                }
            }
        }
    }
}

async fn send_snapshot(
    socket: &mut ConnectionIo,
    service: &RealtimeService,
    principal: &ActorPrincipal,
    connection: &mut ConnectionState,
) -> Result<(), RealtimeError> {
    let Some(platform_id) = connection.platform_id else {
        let message = json!({
            "protocolVersion": REALTIME_PROTOCOL_VERSION,
            "kind": "subscribed"
        });
        return send_text(socket, service, message.to_string()).await;
    };
    let snapshot_revision = service.inner.hub.current_revision();
    let platform = authorize_platform(service, principal, platform_id).await?;
    let containers = service
        .inner
        .reader
        .list_containers(platform_id)
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
        RealtimeEventRef {
            resource_revision: snapshot_revision,
            resource_type: PLATFORM_RESOURCE_TYPE,
            resource_id: platform_id,
            event_kind: "snapshot",
            payload: &payload,
        },
    )
    .await?;
    service.inner.metrics.realtime_snapshot_resynced();
    Ok(())
}

async fn send_envelope(
    socket: &mut ConnectionIo,
    service: &RealtimeService,
    connection: &mut ConnectionState,
    event: RealtimeEventRef<'_>,
) -> Result<(), RealtimeError> {
    let envelope = RealtimeEnvelope {
        protocol_version: REALTIME_PROTOCOL_VERSION,
        connection_id: connection.id,
        sequence: connection.take_sequence(),
        resource_type: event.resource_type,
        resource_id: event.resource_id,
        resource_revision: event.resource_revision,
        event_kind: event.event_kind,
        payload_schema_version: PAYLOAD_SCHEMA_VERSION,
        payload: event.payload,
    };
    let text = serde_json::to_string(&envelope)
        .map_err(|error| RealtimeError::Serialization(error.to_string()))?;
    send_text(socket, service, text).await?;
    connection.last_resource_revision = event.resource_revision;
    Ok(())
}

fn event_matches(event: &PublishedRuntimeEvent, connection: &ConnectionState) -> bool {
    match connection.platform_id {
        Some(platform_id) => event.platform_id == Some(platform_id),
        None => event.platform_id.is_none() || event.resource_type == PLATFORM_RESOURCE_TYPE,
    }
}

fn connection_resource_type(connection: &ConnectionState) -> &'static str {
    if connection.platform_id.is_some() {
        PLATFORM_RESOURCE_TYPE
    } else {
        "Global"
    }
}
