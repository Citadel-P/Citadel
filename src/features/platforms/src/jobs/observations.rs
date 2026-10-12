//! Atomic projection operations used by runtime observation workers.
use super::{
    ContainerDeltaResult, ContainerStateDelta, ProjectionChange, ResourceDelta, ResourceSnapshot,
    SnapshotGeneration,
};
use crate::{RuntimeCapabilityError, RuntimeContainerSummary};
use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct ContainerEvent<'a> {
    pub platform: Uuid,
    pub node: Option<&'a str>,
    pub docker_id: &'a str,
    pub state: Option<&'a str>,
    pub name: Option<&'a str>,
    pub observed: i64,
}

pub trait RuntimeProjectionStore: Send + Sync {
    fn mark_swarm_stale<'a>(
        &'a self,
        platform: Uuid,
        started_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn persist_resource_committed<'a>(
        &'a self,
        snapshot: &'a ResourceSnapshot,
        generation: Option<&'a SnapshotGeneration>,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>>;
    fn persist_delta<'a>(
        &'a self,
        platform: Uuid,
        delta: &'a ResourceDelta,
        generation: &'a SnapshotGeneration,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>>;
    fn container_state_deltas<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        deltas: &'a [ContainerStateDelta],
    ) -> BoxFuture<'a, Result<Vec<ContainerDeltaResult>, RuntimeCapabilityError>>;
    fn container_observation<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        container: &'a RuntimeContainerSummary,
        observed: i64,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>>;
    fn container_event<'a>(
        &'a self,
        event: ContainerEvent<'a>,
        observed_millis: i64,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>>;
}

pub trait PlatformLifecycleStore: Send + Sync {
    fn set_online<'a>(
        &'a self,
        platform: Uuid,
        online: bool,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn reconcile_deployments<'a>(
        &'a self,
        platform: Uuid,
    ) -> BoxFuture<'a, Result<usize, RuntimeCapabilityError>>;
}
