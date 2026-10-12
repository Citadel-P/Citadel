//! Typed committed notifications. A reconnect is explicit so consumers can resync.
use crate::RuntimeSignal;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionResource {
    Platform,
    BuildAgentPool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Online,
    Offline,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionEvent {
    pub resource: ConnectionResource,
    pub resource_id: Uuid,
    pub state: ConnectionState,
}

#[derive(Clone, Debug, Deserialize, Hash, Eq, PartialEq)]
pub struct ContainerCreated {
    pub platform: Uuid,
    pub container: String,
    pub node: Option<String>,
}

#[derive(Clone, Debug)]
pub enum RuntimeNotification {
    Signal(RuntimeSignal),
    Connection(ConnectionEvent),
    /// Invalid connection payload; resync connection subscribers without treating it as a transport reconnect.
    ConnectionResync,
    ContainerCreated(ContainerCreated),
    StackDrift(Uuid),
    SwarmPrune,
    JobAlerts,
    Resync,
}

#[derive(Debug, thiserror::Error)]
#[error("Notification transport failed: {0}")]
pub struct NotificationError(pub String);

/// Receiving must be cancellation safe. Notifications only wake work; durable
/// state and periodic reconciliation remain authoritative after missed events.
pub trait NotificationSource: Send {
    fn recv(&mut self) -> BoxFuture<'_, Result<RuntimeNotification, NotificationError>>;
}
