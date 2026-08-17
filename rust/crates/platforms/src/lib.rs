#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::pin::Pin;
use std::time::Duration;

use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use futures_util::stream::Stream;
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod inventory;
mod read;

pub use inventory::*;
pub use read::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSummary {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub status: String,
    pub connector_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePlatformInfo {
    pub daemon_id: String,
    pub server_version: String,
    pub operating_system: String,
    pub os_type: String,
    pub architecture: String,
    pub cpu_count: i64,
    pub memory_total: i64,
    pub container_count: i64,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
    pub api_version: String,
    pub minimum_api_version: String,
    pub agent_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub image_id: String,
    pub created: i64,
    pub state: String,
    pub status: String,
    pub labels: BTreeMap<String, String>,
    pub ports: serde_json::Value,
    pub stack: Option<String>,
    pub is_system: bool,
    pub system_role: Option<String>,
    pub has_citadel_ownership_labels: bool,
    pub is_swarm_task: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePlatformStats {
    pub memory_usage: f64,
    pub cpu_usage: f64,
    pub receive_bytes: f64,
    pub transmit_bytes: f64,
    pub container_count: i64,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthorizedReadError {
    #[error("authorized read failed: {0}")]
    Storage(String),
}

pub trait AuthorizedPlatformReader: Send + Sync {
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
    ) -> BoxFuture<'a, Result<Vec<PlatformSummary>, AuthorizedReadError>>;
}

pub type RuntimeStatsStream =
    Pin<Box<dyn Stream<Item = Result<RuntimePlatformStats, RuntimeCapabilityError>> + Send>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeErrorKind {
    Cancelled,
    Timeout,
    Unavailable,
    Authentication,
    PermissionDenied,
    InvalidRequest,
    NotFound,
    Conflict,
    ResourceExhausted,
    Remote,
}

#[derive(Debug, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct RuntimeCapabilityError {
    pub kind: RuntimeErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl RuntimeCapabilityError {
    #[must_use]
    pub fn new(kind: RuntimeErrorKind, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
        }
    }
}

pub trait PlatformRuntimePort: Send + Sync {
    fn get_info<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>>;

    fn list_containers<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>>;

    fn stream_stats<'a>(
        &'a self,
        fetch_interval: Duration,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeStatsStream, RuntimeCapabilityError>>;
}
