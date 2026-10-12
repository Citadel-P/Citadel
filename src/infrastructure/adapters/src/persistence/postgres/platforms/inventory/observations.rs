use super::store::{PostgresInventoryProjectionStore, storage};
use chrono::{DateTime, Utc};
use citadel_platforms::jobs::observations::*;
use citadel_platforms::jobs::{
    ContainerDeltaResult, ContainerStateDelta, ProjectionChange, ResourceDelta, ResourceSnapshot,
    SnapshotGeneration,
};
use citadel_platforms::{RuntimeCapabilityError, RuntimeContainerSummary};
use futures_util::future::BoxFuture;
use uuid::Uuid;

use futures_util::FutureExt;
impl RuntimeProjectionStore for PostgresInventoryProjectionStore {
    fn mark_swarm_stale<'a>(
        &'a self,
        platform: Uuid,
        started_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        async move { self.mark_swarm_stale(platform, started_at).await }.boxed()
    }
    fn persist_resource_committed<'a>(
        &'a self,
        snapshot: &'a ResourceSnapshot,
        generation: Option<&'a SnapshotGeneration>,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>> {
        async move { self.persist_resource_committed(snapshot, generation).await }.boxed()
    }
    fn persist_delta<'a>(
        &'a self,
        platform: Uuid,
        delta: &'a ResourceDelta,
        generation: &'a SnapshotGeneration,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>> {
        async move { self.persist_delta(platform, delta, generation).await }.boxed()
    }
    fn container_state_deltas<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        deltas: &'a [ContainerStateDelta],
    ) -> BoxFuture<'a, Result<Vec<ContainerDeltaResult>, RuntimeCapabilityError>> {
        async move {
            super::super::status::container_state_deltas_committed(
                &self.pool, platform, node, deltas,
            )
            .await
            .map_err(storage)
        }
        .boxed()
    }
    fn container_observation<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        container: &'a RuntimeContainerSummary,
        observed: i64,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>> {
        async move {
            super::super::status::container_observation_committed(
                &self.pool, platform, node, container, observed,
            )
            .await
            .map_err(storage)
        }
        .boxed()
    }
    fn container_event<'a>(
        &'a self,
        event: ContainerEvent<'a>,
        observed_millis: i64,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>> {
        async move {
            super::super::status::container_event_committed_at(&self.pool, event, observed_millis)
                .await
                .map_err(storage)
        }
        .boxed()
    }
}
impl PlatformLifecycleStore for PostgresInventoryProjectionStore {
    fn set_online<'a>(
        &'a self,
        platform: Uuid,
        online: bool,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        async move {
            if online {
                super::super::status::platform_online(&self.pool, platform).await
            } else {
                super::super::status::platform_offline(&self.pool, platform).await
            }
            .map_err(storage)
        }
        .boxed()
    }
    fn reconcile_deployments<'a>(
        &'a self,
        platform: Uuid,
    ) -> BoxFuture<'a, Result<usize, RuntimeCapabilityError>> {
        async move {
            super::super::status::reconcile_deployments(&self.pool, platform, None, false)
                .await
                .map_err(storage)
        }
        .boxed()
    }
}
