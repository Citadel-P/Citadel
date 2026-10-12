//! Persistence required by the Edge transport's fenced observation sessions.
use super::store::PostgresEdgeStore;
use crate::connectors::agent::client::AgentDaemonEvent;
use crate::connectors::edge::{EdgeSession, observations::EdgeObservationPersistence};
use chrono::{DateTime, Utc};
use citadel_platforms::jobs::{
    ContainerDeltaResult, ContainerStateDelta, ProjectionChange, ResourceSnapshot,
    SnapshotGeneration,
};
use citadel_platforms::{PlatformKind, RuntimeCapabilityError};
use futures_util::future::BoxFuture;
use uuid::Uuid;
impl EdgeObservationPersistence for PostgresEdgeStore {
    fn session_platform_kind<'a>(
        &'a self,
        platform: Uuid,
        agent: Uuid,
        connected_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<PlatformKind, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.session_platform_kind(platform, agent, connected_at)
                .await
                .map_err(|e| {
                    RuntimeCapabilityError::new(
                        citadel_platforms::RuntimeErrorKind::Remote,
                        e.to_string(),
                        true,
                    )
                })
        })
    }
    fn complete_resource_recovery<'a>(
        &'a self,
        session: &'a EdgeSession,
        started: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.complete_resource_recovery(session, started)
                .await
                .map_err(|e| {
                    RuntimeCapabilityError::new(
                        citadel_platforms::RuntimeErrorKind::Remote,
                        e.to_string(),
                        true,
                    )
                })
        })
    }
    fn persist_resource_committed<'a>(
        &'a self,
        session: &'a EdgeSession,
        snapshot: &'a ResourceSnapshot,
        generation: Option<&'a SnapshotGeneration>,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.persist_resource_committed(session, snapshot, generation)
                .await
                .map_err(|e| {
                    RuntimeCapabilityError::new(
                        citadel_platforms::RuntimeErrorKind::Remote,
                        e.to_string(),
                        true,
                    )
                })
        })
    }
    fn persist_resource_event<'a>(
        &'a self,
        session: &'a EdgeSession,
        event: &'a AgentDaemonEvent,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.persist_resource_event(session, event)
                .await
                .map_err(|e| {
                    RuntimeCapabilityError::new(
                        citadel_platforms::RuntimeErrorKind::Remote,
                        e.to_string(),
                        true,
                    )
                })
        })
    }
    fn persist_container_state_deltas<'a>(
        &'a self,
        session: &'a EdgeSession,
        deltas: &'a [ContainerStateDelta],
    ) -> BoxFuture<'a, Result<Vec<ContainerDeltaResult>, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.persist_container_state_deltas(session, deltas)
                .await
                .map_err(|e| {
                    RuntimeCapabilityError::new(
                        citadel_platforms::RuntimeErrorKind::Remote,
                        e.to_string(),
                        true,
                    )
                })
        })
    }
    fn persist_container_event_at<'a>(
        &'a self,
        session: &'a EdgeSession,
        event: &'a AgentDaemonEvent,
        observed_millis: i64,
    ) -> BoxFuture<'a, Result<ProjectionChange, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.persist_container_event_at(session, event, observed_millis)
                .await
                .map_err(|e| {
                    RuntimeCapabilityError::new(
                        citadel_platforms::RuntimeErrorKind::Remote,
                        e.to_string(),
                        true,
                    )
                })
        })
    }
}
