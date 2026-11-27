use super::*;
use citadel_server::realtime::RealtimeHub;
pub(super) async fn join(socket: &mut Socket, group: &str, count: usize) {
    let completion = invoke(socket, "JoinGroup", group).await;
    assert!(completion["error"].is_null(), "{group}: {completion}");
    for _ in 0..count {
        assert_eq!(receive(socket).await["kind"], "event");
    }
}

pub(super) async fn connect(
    f: &Fixture,
    principal: &ActorPrincipal,
    address: std::net::SocketAddr,
) -> Socket {
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(f,principal).await}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut socket).await["kind"], "subscribed");
    socket
}

// A broadcast delivered on both sockets bounds this assertion without a sleep.
pub(super) fn barrier(hub: &RealtimeHub) {
    hub.publish_resource_change("License", Uuid::nil(), "resourceChanged");
}

pub(super) async fn read_until_barrier(socket: &mut Socket) -> Vec<Value> {
    let mut events = Vec::new();
    loop {
        let event = receive(socket).await;
        if event["target"] == "LicenseStateChanged" {
            return events;
        }
        events.push(event);
    }
}

use citadel_server::realtime::{AuthorizationLease, PublishedRuntimeEvent, RealtimeReadError};
use citadel_server::realtime_groups::GroupSnapshot;
use futures_util::future::BoxFuture;

#[derive(Clone, Default)]
pub(super) struct GroupReads(Arc<std::sync::Mutex<Vec<String>>>);

impl GroupReads {
    pub(super) fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

struct ObservedReader {
    inner: ApplicationGroupReader,
    reads: GroupReads,
}
impl GroupReadPort for ObservedReader {
    fn recheck<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        g: &'a Group,
        lease: &'a AuthorizationLease,
    ) -> BoxFuture<'a, Result<(), RealtimeReadError>> {
        self.inner.recheck(p, g, lease)
    }
    fn read_with_lease<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        g: &'a Group,
        e: Option<&'a PublishedRuntimeEvent>,
        lease: &'a AuthorizationLease,
    ) -> BoxFuture<'a, Result<GroupSnapshot, RealtimeReadError>> {
        use citadel_server::realtime::topic::Topic;
        if e.is_some()
            || matches!(
                g.topic(),
                Topic::Dashboard | Topic::DeploymentStats(_) | Topic::ServiceStats(_)
            )
        {
            self.reads.0.lock().unwrap().push(g.name.clone());
        }
        self.inner.read_with_lease(p, g, e, lease)
    }
    fn read<'a>(
        &'a self,
        p: &'a ActorPrincipal,
        g: &'a Group,
        e: Option<&'a PublishedRuntimeEvent>,
    ) -> BoxFuture<'a, Result<GroupSnapshot, RealtimeReadError>> {
        self.inner.read(p, g, e)
    }
}
pub(super) async fn serve(
    f: &Fixture,
) -> (
    RealtimeHub,
    GroupReads,
    std::net::SocketAddr,
    CancellationToken,
    tokio::task::JoinHandle<()>,
) {
    let cancellation = CancellationToken::new();
    let reads = GroupReads::default();
    let service = RealtimeService::new(
        &RealtimeConfig {
            queue_capacity: 32,
            max_connections: 4,
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
    .with_groups(Arc::new(ObservedReader {
        inner: reader(f),
        reads: reads.clone(),
    }));
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

    (hub, reads, address, cancellation, server)
}
