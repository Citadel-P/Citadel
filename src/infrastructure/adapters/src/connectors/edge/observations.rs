//! Persistence required by the Edge transport's fenced observation sessions.
use super::EdgeSession;
use crate::connectors::agent::client::AgentDaemonEvent;
use chrono::{DateTime, Utc};
use citadel_platforms::jobs::{
    ContainerDeltaResult, ContainerStateDelta, ProjectionChange, ResourceSnapshot,
    SnapshotGeneration,
};
use citadel_platforms::{PlatformKind, RuntimeCapabilityError};
use futures_util::future::BoxFuture;
use uuid::Uuid;
pub trait EdgeObservationPersistence: Send + Sync {
    fn session_platform_kind<'a>(
        &'a self,
        platform: Uuid,
        agent: Uuid,
        connected_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<PlatformKind, RuntimeCapabilityError>>;
    fn complete_resource_recovery<'a>(
        &'a self,
        session: &'a EdgeSession,
        started: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>>;
    fn persist_resource_committed<'a>(
        &'a self,
        session: &'a EdgeSession,
        snapshot: &'a ResourceSnapshot,
        generation: Option<&'a SnapshotGeneration>,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>>;
    fn persist_resource_event<'a>(
        &'a self,
        session: &'a EdgeSession,
        event: &'a AgentDaemonEvent,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>>;
    fn persist_container_state_deltas<'a>(
        &'a self,
        session: &'a EdgeSession,
        deltas: &'a [ContainerStateDelta],
    ) -> BoxFuture<'a, Result<Vec<ContainerDeltaResult>, RuntimeCapabilityError>>;
    fn persist_container_event_at<'a>(
        &'a self,
        session: &'a EdgeSession,
        event: &'a AgentDaemonEvent,
        observed_millis: i64,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>>;
}
