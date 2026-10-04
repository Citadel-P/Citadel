//! Central connection-event fanout. Consumers subscribe before producers start.
//! Database state is authoritative; reconnects and slow subscribers trigger resync.
use citadel_adapters::persistence::postgres::connection_events::{
    ConnectionEvent, ConnectionResource,
};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug)]
pub(super) enum ConnectionNotification {
    Changed(ConnectionEvent),
    Resync,
}

#[derive(Clone)]
pub(super) struct ConnectionEventHub {
    sender: broadcast::Sender<ConnectionNotification>,
}

impl ConnectionEventHub {
    pub fn new(capacity: usize) -> Self {
        Self {
            sender: broadcast::channel(capacity.max(1)).0,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ConnectionNotification> {
        self.sender.subscribe()
    }

    pub fn publish(&self, notification: ConnectionNotification) {
        let _ = self.sender.send(notification);
    }
}

pub(super) async fn realtime_updates(
    mut events: broadcast::Receiver<ConnectionNotification>,
    realtime: Option<crate::realtime::RealtimeHub>,
    cancellation: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    loop {
        let event = tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            event = events.recv() => event,
        };
        let notification = match event {
            Ok(notification) => notification,
            Err(broadcast::error::RecvError::Lagged(_)) => ConnectionNotification::Resync,
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        };
        let Some(realtime) = &realtime else { continue };
        match notification {
            ConnectionNotification::Changed(event) => {
                let resource = match event.resource {
                    ConnectionResource::Platform => "Platform",
                    ConnectionResource::BuildAgentPool => "BuildAgentPool",
                };
                realtime.publish_resource_change(resource, event.resource_id, "connectionChanged");
            }
            ConnectionNotification::Resync => {
                // Nil selects all subscribed resources; each read still checks ACLs.
                for resource in ["Platform", "BuildAgentPool"] {
                    realtime.publish_resource_change(
                        resource,
                        uuid::Uuid::nil(),
                        "connectionChanged",
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_adapters::persistence::postgres::connection_events::ConnectionState;
    use std::sync::Arc;

    #[tokio::test]
    async fn fans_out_to_independent_subscribers_and_realtime() {
        let hub = ConnectionEventHub::new(8);
        let mut other = hub.subscribe();
        let realtime =
            crate::realtime::RealtimeHub::new(8, Arc::new(crate::metrics::Metrics::default()));
        let mut updates = realtime.subscribe();
        let token = CancellationToken::new();
        let worker = tokio::spawn(realtime_updates(
            hub.subscribe(),
            Some(realtime),
            token.clone(),
        ));
        for resource in [
            ConnectionResource::Platform,
            ConnectionResource::BuildAgentPool,
        ] {
            for state in [
                ConnectionState::Online,
                ConnectionState::Offline,
                ConnectionState::Revoked,
            ] {
                let event = ConnectionEvent {
                    resource,
                    resource_id: uuid::Uuid::now_v7(),
                    state,
                };
                hub.publish(ConnectionNotification::Changed(event));
                assert!(
                    matches!(other.recv().await.unwrap(), ConnectionNotification::Changed(received) if received == event)
                );
                let update =
                    tokio::time::timeout(std::time::Duration::from_secs(1), updates.recv())
                        .await
                        .unwrap()
                        .unwrap();
                assert_eq!(update.resource_id, event.resource_id);
                assert_eq!(update.event_kind, "connectionChanged");
                let group = if resource == ConnectionResource::Platform {
                    "docker-daemon"
                } else {
                    "build-agent-pool"
                };
                assert!(
                    crate::realtime_groups::Group::parse(&format!("{group}:{}", event.resource_id))
                        .unwrap()
                        .affected_by(&update)
                );
                assert_eq!(
                    update.resource_type,
                    if resource == ConnectionResource::Platform {
                        "Platform"
                    } else {
                        "BuildAgentPool"
                    }
                );
            }
        }
        token.cancel();
        worker.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn lag_and_database_reconnect_refresh_both_resource_types() {
        let hub = ConnectionEventHub::new(1);
        let events = hub.subscribe();
        hub.publish(ConnectionNotification::Resync);
        hub.publish(ConnectionNotification::Resync);
        let realtime =
            crate::realtime::RealtimeHub::new(8, Arc::new(crate::metrics::Metrics::default()));
        let mut updates = realtime.subscribe();
        let token = CancellationToken::new();
        let worker = tokio::spawn(realtime_updates(events, Some(realtime), token.clone()));
        // One pair for receiver lag, one for the explicit database resync.
        for expected in ["Platform", "BuildAgentPool", "Platform", "BuildAgentPool"] {
            let update = tokio::time::timeout(std::time::Duration::from_secs(1), updates.recv())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(update.resource_type, expected);
            assert!(update.resource_id.is_nil());
            let group = if expected == "Platform" {
                "docker-daemon"
            } else {
                "build-agent-pool"
            };
            assert!(
                crate::realtime_groups::Group::parse(&format!("{group}:{}", uuid::Uuid::now_v7()))
                    .unwrap()
                    .affected_by(&update)
            );
        }
        token.cancel();
        worker.await.unwrap().unwrap();
    }
}
