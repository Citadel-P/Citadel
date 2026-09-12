use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::Router;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::{ActorPrincipal, IdentityService};
use citadel_platforms::{ContainerView, PlatformReadService, PlatformView};
use futures_util::StreamExt;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, broadcast};
use tokio::time::{Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::config::RealtimeConfig;
use crate::metrics::Metrics;

/// Coarse invalidations for resource routers whose writes finish inside the
/// handler. Jobs publish their claim/completion separately; no payload or IDs
/// are broadcast here, and every follow-up read applies its normal ACL.
pub fn notify_mutations(router: Router, resource_type: &'static str) -> Router {
    router.layer(axum::middleware::from_fn_with_state(
        resource_type,
        mutation_notification,
    ))
}

async fn mutation_notification(
    State(resource_type): State<&'static str>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let hub = (!request.method().is_safe())
        .then(|| request.extensions().get::<RealtimeHub>().cloned())
        .flatten();
    let response = next.run(request).await;
    if response.status().is_success()
        && let Some(hub) = hub
    {
        hub.publish_resource_change(resource_type, Uuid::nil(), "resourceChanged");
    }
    response
}

pub fn change_callback(
    hub: Option<RealtimeHub>,
    resource_type: &'static str,
) -> Arc<dyn Fn() + Send + Sync> {
    Arc::new(move || {
        if let Some(hub) = &hub {
            hub.publish_resource_change(resource_type, Uuid::nil(), "resourceChanged");
        }
    })
}

pub fn build_log_callback(hub: Option<RealtimeHub>) -> citadel_builds::BuildLogNotifier {
    Arc::new(move |run_id, entry| {
        if let Some(hub) = &hub {
            hub.publish_build_log(run_id, entry);
        }
    })
}

pub trait RealtimeReadPort: Send + Sync {
    fn authenticate<'a>(
        &'a self,
        token: &'a str,
    ) -> BoxFuture<'a, Result<ActorPrincipal, RealtimeReadError>>;

    fn authorize_platform<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        platform_id: Uuid,
    ) -> BoxFuture<'a, Result<PlatformView, RealtimeReadError>>;

    fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerView>, RealtimeReadError>>;
}

#[derive(Debug, thiserror::Error)]
pub enum RealtimeReadError {
    #[error("authentication failed")]
    Authentication,
    #[error("authorization failed")]
    Authorization,
    #[error("realtime read failed: {0}")]
    Storage(String),
}

pub struct IdentityRealtimeReader {
    identity: Arc<IdentityService>,
    platforms: Arc<PlatformReadService>,
}

impl IdentityRealtimeReader {
    #[must_use]
    pub fn new(identity: Arc<IdentityService>, platforms: Arc<PlatformReadService>) -> Self {
        Self {
            identity,
            platforms,
        }
    }
}

impl RealtimeReadPort for IdentityRealtimeReader {
    fn authenticate<'a>(
        &'a self,
        token: &'a str,
    ) -> BoxFuture<'a, Result<ActorPrincipal, RealtimeReadError>> {
        Box::pin(async move {
            crate::token_safety::authenticate_realtime(&self.identity, token)
                .await
                .map_err(|error| match error {
                    citadel_identity::IdentityError::Forbidden => RealtimeReadError::Authorization,
                    _ => RealtimeReadError::Authentication,
                })
        })
    }

    fn authorize_platform<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        platform_id: Uuid,
    ) -> BoxFuture<'a, Result<PlatformView, RealtimeReadError>> {
        Box::pin(async move {
            if !principal.is_administrator() {
                let permission = self
                    .identity
                    .permission_for_resource(principal, ResourceType::Platform, platform_id)
                    .await
                    .map_err(|error| RealtimeReadError::Storage(error.to_string()))?;
                if !permission
                    .is_some_and(|permission| permission.level.grants(PermissionLevel::Read))
                {
                    return Err(RealtimeReadError::Authorization);
                }
            }
            self.platforms
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
        Box::pin(async move {
            self.platforms
                .list_containers(platform_id)
                .await
                .map_err(|error| RealtimeReadError::Storage(error.to_string()))
        })
    }
}

pub const REALTIME_PROTOCOL_VERSION: u16 = 1;
const PAYLOAD_SCHEMA_VERSION: u16 = 1;
const PLATFORM_RESOURCE_TYPE: &str = "Platform";
const MAX_CLIENT_MESSAGE_BYTES: usize = 16 * 1024;
const WRITE_BUFFER_BYTES: usize = 16 * 1024;
const MAX_WRITE_BUFFER_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone)]
pub struct PublishedRuntimeEvent {
    pub(crate) platform_id: Option<Uuid>,
    pub(crate) resource_type: &'static str,
    pub(crate) resource_id: Uuid,
    pub(crate) event_kind: &'static str,
    pub(crate) resource_revision: u64,
    pub(crate) payload: Value,
}

impl PublishedRuntimeEvent {
    #[must_use]
    pub fn resource_type(&self) -> &'static str {
        self.resource_type
    }
}

#[derive(Clone)]
pub struct RealtimeHub {
    inner: Arc<RealtimeHubInner>,
}

struct RealtimeHubInner {
    revision: AtomicU64,
    sender: broadcast::Sender<Arc<PublishedRuntimeEvent>>,
    metrics: Arc<Metrics>,
}

impl RealtimeHub {
    #[must_use]
    pub fn new(capacity: usize, metrics: Arc<Metrics>) -> Self {
        let (sender, receiver) = broadcast::channel(capacity);
        drop(receiver);
        Self {
            inner: Arc::new(RealtimeHubInner {
                revision: AtomicU64::new(0),
                sender,
                metrics,
            }),
        }
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
        platform_id: Uuid,
        docker_resource_type: impl Into<String>,
        action: impl Into<String>,
        runtime_resource_id: impl Into<String>,
    ) -> u64 {
        if self.inner.sender.receiver_count() == 0 {
            return self.current_revision();
        }
        self.publish(
            Some(platform_id),
            PLATFORM_RESOURCE_TYPE,
            platform_id,
            "runtimeChanged",
            json!({
                "dockerResourceType": docker_resource_type.into(),
                "action": action.into(),
                "runtimeResourceId": runtime_resource_id.into(),
            }),
        )
    }

    pub fn publish_container_stats(
        &self,
        platform_id: Uuid,
        stats: &[citadel_platforms::RuntimeContainerStat],
    ) -> u64 {
        self.publish_scoped_container_stats(platform_id, None, stats)
    }

    pub fn publish_scoped_container_stats(
        &self,
        platform_id: Uuid,
        node_id: Option<&str>,
        stats: &[citadel_platforms::RuntimeContainerStat],
    ) -> u64 {
        if self.inner.sender.receiver_count() == 0 {
            return self.current_revision();
        }
        self.publish(
            Some(platform_id),
            PLATFORM_RESOURCE_TYPE,
            platform_id,
            "runtimeChanged",
            json!({
                "dockerResourceType": "containerStats",
                "dockerNodeId": node_id,
                "action": "sample",
                "runtimeResourceId": platform_id,
                "stats": stats,
            }),
        )
    }

    pub fn publish_resource_change(
        &self,
        resource_type: &'static str,
        resource_id: Uuid,
        event_kind: &'static str,
    ) -> u64 {
        if self.inner.sender.receiver_count() == 0 {
            return self.current_revision();
        }
        self.publish(None, resource_type, resource_id, event_kind, json!({}))
    }

    pub fn publish_build_log(&self, run_id: Uuid, entry: citadel_builds::BuildLogEntry) -> u64 {
        if self.inner.sender.receiver_count() == 0 {
            return self.current_revision();
        }
        self.publish(
            None,
            "Build",
            run_id,
            "buildLogs",
            json!({"entries":[entry]}),
        )
    }

    fn publish(
        &self,
        platform_id: Option<Uuid>,
        resource_type: &'static str,
        resource_id: Uuid,
        event_kind: &'static str,
        payload: Value,
    ) -> u64 {
        let revision = self.inner.revision.fetch_add(1, Ordering::AcqRel) + 1;
        let event = Arc::new(PublishedRuntimeEvent {
            platform_id,
            resource_type,
            resource_id,
            event_kind,
            resource_revision: revision,
            payload,
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
    groups: Option<Arc<dyn crate::realtime_groups::GroupReadPort>>,
    subscribe_timeout: Duration,
    send_timeout: Duration,
    authorization_recheck_interval: Duration,
    snapshot_limit: usize,
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
    client_mode: Option<String>,
    invocation_id: Option<String>,
    target: Option<String>,
    arguments: Option<Vec<Value>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RealtimeEnvelope<'a> {
    protocol_version: u16,
    connection_id: Uuid,
    sequence: u64,
    resource_type: &'a str,
    resource_id: Uuid,
    resource_revision: u64,
    event_kind: &'a str,
    payload_schema_version: u16,
    payload: &'a Value,
}

struct RealtimeEventRef<'a> {
    resource_revision: u64,
    resource_type: &'a str,
    resource_id: Uuid,
    event_kind: &'a str,
    payload: &'a Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformSnapshot {
    platform: PlatformView,
    containers: Vec<ContainerView>,
}

struct ConnectionState {
    id: Uuid,
    platform_id: Option<Uuid>,
    next_sequence: u64,
    last_resource_revision: u64,
}

impl ConnectionState {
    fn new(platform_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::now_v7(),
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
    let groups_mode = subscribe.client_mode.as_deref() == Some("groups");
    let mut subscription = validate_subscription(service, subscribe).await?;

    if groups_mode {
        return run_group_connection(socket, service, subscription, cancellation).await;
    }

    let mut connection = ConnectionState::new(subscription.platform_id);
    let mut receiver = service.inner.hub.subscribe();
    send_snapshot(socket, service, &subscription.principal, &mut connection).await?;

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

async fn run_group_connection(
    socket: &mut WebSocket,
    service: &RealtimeService,
    mut subscription: Subscription,
    cancellation: &CancellationToken,
) -> Result<(), RealtimeError> {
    use crate::realtime_groups::{Group, GroupSubscription, MAX_GROUPS};
    use std::collections::BTreeMap;
    let reader = service
        .inner
        .groups
        .as_ref()
        .ok_or(RealtimeError::Authorization)?;
    let mut groups: BTreeMap<String, GroupSubscription> = BTreeMap::new();
    let mut streams = tokio_stream::StreamMap::<String, crate::realtime_groups::GroupStream>::new();
    let mut stream_guards = BTreeMap::new();
    let mut terminal_inputs: BTreeMap<String, citadel_platforms::terminal::TerminalInputSender> =
        BTreeMap::new();
    let mut receiver = service.inner.hub.subscribe();
    send_group_message(
        socket,
        service,
        &json!({"protocolVersion":1,"kind":"subscribed"}),
    )
    .await?;
    let mut recheck = tokio::time::interval_at(
        Instant::now() + service.inner.authorization_recheck_interval,
        service.inner.authorization_recheck_interval,
    );
    recheck.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            ()=cancellation.cancelled()=>return Ok(()),
            message=socket.next()=>{
                let text=match message {
                    Some(Ok(Message::Text(text)))=>text,
                    Some(Ok(Message::Close(_)))|None=>return Ok(()),
                    Some(Ok(Message::Ping(_)|Message::Pong(_)))=>continue,
                    _=>return Err(RealtimeError::InvalidMessage("Expected a text invocation".into())),
                };
                let request=parse_client_message(&text)?;
                if request.protocol_version!=REALTIME_PROTOCOL_VERSION || request.kind!="invoke" {return Err(RealtimeError::InvalidMessage("Expected a versioned invocation".into()));}
                let invocation=request.invocation_id.filter(|id|!id.is_empty()&&id.len()<=64).ok_or_else(||RealtimeError::InvalidMessage("Invalid invocation ID".into()))?;
                subscription.principal=service.inner.reader.authenticate(&subscription.access_token).await.map_err(map_realtime_read_error)?;
                let name=request.arguments.as_ref().and_then(|args|(args.len()==1).then(||args[0].as_str()).flatten());
                let mut updates=vec![];
                let error=match (request.target.as_deref(),name) {
                    (Some("LeaveGroup"),Some(name))=>{groups.remove(name);streams.remove(name);stream_guards.remove(name);terminal_inputs.remove(name);None},
                    (Some("JoinGroup"),Some(name))=>{
                        if !groups.contains_key(name) && groups.len()>=MAX_GROUPS {Some("Realtime group limit exceeded")}
                        else if let Some(group)=Group::parse(name) {
                            match tokio::time::timeout(service.inner.subscribe_timeout,reader.read(&subscription.principal,&group,None)).await {
                                Ok(Ok(snapshot))=>{
                                    if groups.contains_key(name) {
                                        // Revalidate access even on an idempotent join.
                                        send_group_message(socket,service,&json!({"protocolVersion":1,"kind":"completion","invocationId":invocation,"error":null})).await?;
                                        continue;
                                    }
                                    let mut joined=GroupSubscription::new(group);
                                    updates=joined.apply(snapshot,service.inner.snapshot_limit).map_err(map_realtime_read_error)?;
                                    groups.insert(name.into(),joined);
                                    None
                                },
                                Ok(Err(_))=>Some("Not authorized to join this group, or the resource is unavailable."),
                                Err(_)=>Some("Realtime group initialization timed out"),
                            }
                        } else {Some("Not authorized to join this group.")}
                    },
                    (Some(target),_)=>{
                        let args=request.arguments.as_deref().unwrap_or_default();
                        let terminal=tokio::time::timeout(service.inner.subscribe_timeout,reader.terminal_invocation(&subscription.principal,target,args)).await;
                        let terminal_error=match terminal {
                            Ok(Ok(Some(command)))=>{
                                use crate::realtime_groups::TerminalAction;
                                let error=if !groups.contains_key(&command.group.name) {Some("Join the terminal group before starting or sending input.")}
                                else {match command.action {
                                    TerminalAction::Input(input)=>match terminal_inputs.get(&command.group.name) {
                                        Some(sender)=>sender.try_send(input).err().map(|_|"Terminal input is closed, invalid or overloaded."),
                                        None=>Some("This connection does not own an active terminal session."),
                                    },
                                    TerminalAction::Start(shell)=>{
                                        if terminal_inputs.contains_key(&command.group.name) {None}
                                        // The legacy SendContainerExec event has no session discriminator.
                                        else if !terminal_inputs.is_empty() || stream_guards.len()>=4 {Some("An active terminal already exists on this connection.")}
                                        else {
                                            let token=cancellation.child_token(); let guard=token.clone().drop_guard();
                                            match tokio::time::timeout(service.inner.subscribe_timeout,reader.terminal(&subscription.principal,&command.group,shell,&token)).await {
                                                Ok(Ok(session))=>{
                                                    terminal_inputs.insert(command.group.name.clone(),session.input);
                                                    streams.insert(command.group.name.clone(),session.output);
                                                    stream_guards.insert(command.group.name,guard);None
                                                },
                                                _=>Some("Terminal target is unauthorized, unavailable or timed out."),
                                            }
                                        }
                                    }
                                }};
                                Some(error)
                            },
                            Ok(Ok(None))=>None,
                            _=>Some(Some("Terminal request is invalid, unauthorized or unavailable.")),
                        };
                        if let Some(error)=terminal_error {
                            send_group_message(socket,service,&json!({"protocolVersion":1,"kind":"completion","invocationId":invocation,"error":error})).await?;
                            continue;
                        }
                        match tokio::time::timeout(service.inner.subscribe_timeout,reader.invocation_group(&subscription.principal,target,args)).await {
                            Ok(Ok(Some(group))) if groups.contains_key(&group.name)=>{
                                if stream_guards.contains_key(&group.name) { None }
                                else if stream_guards.len()>=4 {Some("Realtime stream limit exceeded")}
                                else {
                                    let token=cancellation.child_token();
                                    let guard=token.clone().drop_guard();
                                    match tokio::time::timeout(service.inner.subscribe_timeout,reader.stream(&subscription.principal,&group,&token)).await {
                                        Ok(Ok(Some(stream)))=>{streams.insert(group.name.clone(),stream);stream_guards.insert(group.name,guard);None},
                                        Ok(Ok(None))=>Some("This resource does not support streaming."),
                                        Ok(Err(_))=>Some("Not authorized to stream this resource, or the resource is unavailable."),
                                        Err(_)=>Some("Realtime stream initialization timed out"),
                                    }
                                }
                            },
                            _=>Some("Not authorized to invoke this method, or its resource group has not been joined."),
                        }
                    },
                    _=>Some("This realtime method is not available in the Rust backend."),
                };
                send_group_message(socket,service,&json!({"protocolVersion":1,"kind":"completion","invocationId":invocation,"error":error})).await?;
                for update in updates {send_group_message(socket,service,&update).await?;}
            },
            _=recheck.tick()=>{
                subscription.principal=service.inner.reader.authenticate(&subscription.access_token).await.map_err(map_realtime_read_error)?;
                for name in stream_guards.keys() {
                    if let Some(joined)=groups.get(name) {
                        tokio::time::timeout(service.inner.subscribe_timeout,reader.read(&subscription.principal,&joined.group,None)).await
                            .map_err(|_|RealtimeError::SubscribeTimeout)?.map_err(map_realtime_read_error)?;
                    }
                }
            },
            Some((_,item))=streams.next(),if !streams.is_empty()=>{
                let update=item.map_err(map_realtime_read_error)?;
                send_group_message(socket,service,&update).await?;
            },
            event=receiver.recv()=>{
                let event=match event {
                    Ok(event)=>event,
                    Err(broadcast::error::RecvError::Lagged(_))=>{
                        // Close rather than dropping changes. The existing reconnect
                        // lifecycle rejoins groups and refreshes authoritative reads.
                        service.inner.metrics.realtime_overflowed();
                        return Err(RealtimeError::InvalidMessage("Realtime resynchronization required".into()));
                    },
                    Err(broadcast::error::RecvError::Closed)=>return Ok(()),
                };
                subscription.principal=service.inner.reader.authenticate(&subscription.access_token).await.map_err(map_realtime_read_error)?;
                if event.resource_type=="License" {
                    send_group_message(socket,service,&crate::realtime_groups::ClientEvent::new("LicenseStateChanged",vec![])).await?;
                    continue;
                }
                for joined in groups.values_mut().filter(|g|g.group.affected_by(&event)) {
                    // Inventory I/O must not pause this connection's active streams.
                    let snapshot={
                        let read=tokio::time::timeout(service.inner.subscribe_timeout,reader.read(&subscription.principal,&joined.group,Some(&event)));
                        tokio::pin!(read);
                        loop {
                            tokio::select! {
                                biased;
                                ()=cancellation.cancelled()=>return Ok(()),
                                result=&mut read=>break result.map_err(|_|RealtimeError::SubscribeTimeout)?.map_err(map_realtime_read_error)?,
                                Some((_,item))=streams.next(),if !streams.is_empty()=>{
                                    send_group_message(socket,service,&item.map_err(map_realtime_read_error)?).await?;
                                }
                            }
                        }
                    };
                    for update in joined.apply(snapshot,service.inner.snapshot_limit).map_err(map_realtime_read_error)? {
                        send_group_message(socket,service,&update).await?;
                    }
                }
            }
        }
    }
}

async fn send_group_message(
    socket: &mut WebSocket,
    service: &RealtimeService,
    message: &impl Serialize,
) -> Result<(), RealtimeError> {
    let text = serde_json::to_string(message)
        .map_err(|error| RealtimeError::Serialization(error.to_string()))?;
    if text.len() > 4 * 1024 * 1024 {
        return Err(RealtimeError::InvalidMessage(
            "Realtime payload limit exceeded".into(),
        ));
    }
    tokio::time::timeout(
        service.inner.send_timeout,
        socket.send(Message::Text(text.into())),
    )
    .await
    .map_err(|_| RealtimeError::SendTimeout)?
    .map_err(|error| RealtimeError::Socket(error.to_string()))?;
    service.inner.metrics.realtime_message_sent();
    Ok(())
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

struct Subscription {
    principal: ActorPrincipal,
    platform_id: Option<Uuid>,
    access_token: Zeroizing<String>,
}

async fn validate_subscription(
    service: &RealtimeService,
    subscribe: ClientMessage,
) -> Result<Subscription, RealtimeError> {
    if subscribe.protocol_version != REALTIME_PROTOCOL_VERSION {
        return Err(RealtimeError::UnsupportedProtocol(
            subscribe.protocol_version,
        ));
    }
    if subscribe.kind != "subscribe" {
        return Err(RealtimeError::InvalidMessage(
            "a realtime subscription is required".to_owned(),
        ));
    }
    let Some(access_token) = subscribe.access_token.map(Zeroizing::new) else {
        service.inner.metrics.realtime_authorization_failed();
        return Err(RealtimeError::Authentication);
    };
    let principal = service
        .inner
        .reader
        .authenticate(access_token.as_str())
        .await
        .map_err(map_realtime_read_error)?;
    let platform_id = match (subscribe.resource_type.as_deref(), subscribe.resource_id) {
        (None, None) => None,
        (Some(PLATFORM_RESOURCE_TYPE), Some(platform_id)) => {
            authorize_platform(service, &principal, platform_id).await?;
            Some(platform_id)
        }
        _ => {
            return Err(RealtimeError::InvalidMessage(
                "resourceType and resourceId must identify a Platform, or both be omitted"
                    .to_owned(),
            ));
        }
    };
    Ok(Subscription {
        principal,
        platform_id,
        access_token,
    })
}

async fn authorize_platform(
    service: &RealtimeService,
    principal: &ActorPrincipal,
    platform_id: Uuid,
) -> Result<PlatformView, RealtimeError> {
    let result = service
        .inner
        .reader
        .authorize_platform(principal, platform_id)
        .await
        .map_err(map_realtime_read_error);
    if result.is_err() {
        service.inner.metrics.realtime_authorization_failed();
    }
    result
}

fn map_realtime_read_error(error: RealtimeReadError) -> RealtimeError {
    match error {
        RealtimeReadError::Authentication => RealtimeError::Authentication,
        RealtimeReadError::Authorization => RealtimeError::Authorization,
        RealtimeReadError::Storage(message) => RealtimeError::AuthorizationStorage(message),
    }
}

async fn send_snapshot(
    socket: &mut WebSocket,
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
    socket: &mut WebSocket,
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

async fn send_text(
    socket: &mut WebSocket,
    service: &RealtimeService,
    text: String,
) -> Result<(), RealtimeError> {
    match tokio::time::timeout(
        service.inner.send_timeout,
        socket.send(Message::Text(text.into())),
    )
    .await
    {
        Ok(Ok(())) => {
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
