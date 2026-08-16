#![forbid(unsafe_code)]

mod supervisor;

use std::pin::Pin;
use std::time::Duration;

use citadel_domain::{
    ActorId, PlatformSummary, RuntimeContainerSummary, RuntimePlatformInfo, RuntimePlatformStats,
};
use futures_util::future::BoxFuture;
use futures_util::stream::Stream;
use tokio_util::sync::CancellationToken;

pub use supervisor::{SupervisedTaskError, TaskSupervisor};

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
