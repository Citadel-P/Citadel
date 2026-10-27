//! Resolve the run's original resource, never the project's newly selected builder.
use super::{client::AgentClient, execution::AgentExecutionClient};
use crate::connectors::edge::{EdgeRegistry, EdgeTarget};
use citadel_builds::{
    BuildCleanupAttempt, BuildCleanupChecker, BuildError, BuildExecutorCapacity,
    BuildPlatformSnapshot, BuildPoolTarget,
};
use citadel_contracts::citadel::images::v1::{BuildCleanupState, ReconcileBuildRequest};
use futures_util::future::BoxFuture;
use sqlx::PgPool;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct AgentBuildCleanupChecker {
    pub pool: PgPool,
    pub local: Option<(citadel_docker::client::DockerClient, uuid::Uuid)>,
    pub agent: Option<AgentClient>,
    pub edge: EdgeRegistry,
}

impl AgentBuildCleanupChecker {
    async fn client(
        &self,
        target: &BuildPlatformSnapshot,
        cancellation: &CancellationToken,
    ) -> Result<AgentExecutionClient, BuildError> {
        let (id, connection) = match target.builder_kind.as_deref() {
            Some("BuildAgentPool") => {
                let id = target.build_agent_pool_id.ok_or(BuildError::NotFound)?;
                let spec =
                    crate::persistence::postgres::builds::execution_target::pool_cleanup_spec(
                        &self.pool, id,
                    )
                    .await
                    .map_err(|_| BuildError::Storage("Could not load cleanup target.".into()))?
                    .ok_or(BuildError::NotFound)?;
                (id, citadel_builds::pool_target(&spec)?)
            }
            Some("Platform") => {
                let id = target.id.ok_or(BuildError::NotFound)?;
                let (connector, address, _) =
                    crate::persistence::postgres::platforms::connection::execution_target(
                        &self.pool, id,
                    )
                    .await
                    .map_err(|_| BuildError::Storage("Could not load cleanup target.".into()))?
                    .ok_or(BuildError::NotFound)?;
                (
                    id,
                    match connector.as_str() {
                        "Agent" => BuildPoolTarget::Inbound(address),
                        "EdgeAgent" => BuildPoolTarget::Edge,
                        _ => {
                            return Err(BuildError::Validation(
                                "Original remote build target is unavailable.".into(),
                            ));
                        }
                    },
                )
            }
            _ => {
                return Err(BuildError::Validation(
                    "Original build target is unknown.".into(),
                ));
            }
        };
        match connection {
            BuildPoolTarget::Inbound(address) => {
                let agent = self.agent.as_ref().ok_or(BuildError::NotFound)?;
                let client = agent
                    .for_address(&address, id, cancellation)
                    .await
                    .map_err(|_| {
                        BuildError::Validation("Build cleanup Agent is unavailable.".into())
                    })?;
                Ok(AgentExecutionClient::Direct(Arc::new(client)))
            }
            BuildPoolTarget::Edge => {
                let target = if target.builder_kind.as_deref() == Some("BuildAgentPool") {
                    EdgeTarget::build_pool(id)
                } else {
                    EdgeTarget::platform(id)
                };
                self.edge
                    .get(&target)
                    .map(AgentExecutionClient::Edge)
                    .map_err(|_| {
                        BuildError::Validation("Build cleanup Agent is unavailable.".into())
                    })
            }
        }
    }
}

impl BuildCleanupChecker for AgentBuildCleanupChecker {
    fn recovery_host<'a>(
        &'a self,
        target: &'a BuildPlatformSnapshot,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<BuildExecutorCapacity, BuildError>> {
        Box::pin(async move {
            if target.builder_kind.as_deref() == Some("Platform")
                && let Some(id) = target.id
            {
                let target = crate::persistence::postgres::platforms::connection::execution_target(
                    &self.pool, id,
                )
                .await
                .map_err(|_| BuildError::Storage("Could not load cleanup target.".into()))?;
                if target.is_some_and(|(connector, _, _)| connector == "Local") {
                    let (docker, authority) = self.local.as_ref().ok_or(BuildError::NotFound)?;
                    let info = docker
                        .info()
                        .await
                        .map_err(|_| BuildError::Conflict("Local Docker is unavailable.".into()))?;
                    return Ok(BuildExecutorCapacity {
                        daemon_id: info.id,
                        authority_id: *authority,
                        admission_generation: 1,
                        build_slots: 2,
                        heavy_slots: 2,
                    });
                }
            }
            let client = self.client(target, cancellation).await?;
            let host = client.check_build_host(cancellation).await.map_err(|_| BuildError::Conflict("The original Agent is unavailable. Restore its connection before resolving cleanup.".into()))?;
            if !host.available || host.docker_daemon_id.is_empty() {
                return Err(BuildError::Conflict(
                    "Docker is unavailable on the original Agent.".into(),
                ));
            }
            Ok(BuildExecutorCapacity {
                daemon_id: host.docker_daemon_id,
                authority_id: uuid::Uuid::parse_str(&host.admission_authority_id).map_err(
                    |_| {
                        BuildError::Conflict(
                            "The Agent did not provide a valid admission identity.".into(),
                        )
                    },
                )?,
                admission_generation: host.admission_generation,
                build_slots: host.build_capacity,
                heavy_slots: host.heavy_capacity,
            })
        })
    }

    fn confirmed<'a>(
        &'a self,
        attempt: &'a BuildCleanupAttempt,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        Box::pin(async move {
            let client = self.client(&attempt.target, cancellation).await?;
            let response = client
                .reconcile_build(
                    ReconcileBuildRequest {
                        run_id: attempt.run_id.to_string(),
                        attempt_id: attempt.attempt_id.to_string(),
                        docker_daemon_id: attempt.daemon_id.clone(),
                        admission_authority_id: attempt.authority_id.to_string(),
                        admission_generation: attempt.admission_generation,
                    },
                    cancellation,
                )
                .await
                .map_err(|_| {
                    BuildError::Validation("Build cleanup could not be verified.".into())
                })?;
            Ok(response.state == BuildCleanupState::BuildCleanupConfirmed as i32)
        })
    }
}
