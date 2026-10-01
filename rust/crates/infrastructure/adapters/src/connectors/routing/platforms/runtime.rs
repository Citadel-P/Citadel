//! Shared Platform and node transport selection. Authorization belongs to callers.
use crate::connectors::{
    agent::client::AgentClient,
    docker::DockerClient,
    edge::{EdgeRegistry, EdgeRuntime, EdgeTarget},
};
use crate::persistence::postgres::platforms::connection::{self, PlatformConnection};
use citadel_platforms::{PlatformInfoPort, RuntimeCapabilityError, RuntimeErrorKind};
use futures_util::future::BoxFuture;
use sqlx::PgPool;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Hosting supplies its existing connection owner; adapters never depend on Server.
pub trait PlatformAgentResolver: Send + Sync {
    fn resolve_agent<'a>(
        &'a self,
        platform: uuid::Uuid,
        address: &'a str,
    ) -> BoxFuture<'a, Result<Arc<AgentClient>, RuntimeCapabilityError>>;
}

#[derive(Clone)]
pub struct PlatformRuntimeRouter {
    pool: PgPool,
    docker: DockerClient,
    edge: EdgeRegistry,
    resolver: Arc<dyn PlatformAgentResolver>,
}

pub enum Runtime<'a> {
    Local(&'a DockerClient),
    Agent(AgentClient),
    Edge(EdgeRuntime),
}

impl PlatformRuntimeRouter {
    pub async fn swarm<'a>(
        &'a self,
        platform: uuid::Uuid,
        cancel: &CancellationToken,
    ) -> Result<
        impl citadel_platforms::swarm_mutations::SwarmControlPort
        + citadel_platforms::PlatformInfoPort
        + citadel_platforms::NetworkInventoryPort
        + citadel_platforms::SwarmInventoryPort
        + 'a
        + use<'a>,
        RuntimeCapabilityError,
    > {
        self.resolve(platform, None, false, cancel).await
    }

    pub async fn image_mutations(
        &self,
        platform: uuid::Uuid,
        cancel: &CancellationToken,
    ) -> Result<
        impl citadel_platforms::image_pull::ImagePullPort
        + citadel_platforms::images::ImageDeletionPort
        + citadel_platforms::ImageInventoryPort
        + '_,
        RuntimeCapabilityError,
    > {
        self.resolve(platform, None, false, cancel).await
    }

    pub async fn pruning(
        &self,
        platform: uuid::Uuid,
        cancel: &CancellationToken,
    ) -> Result<impl citadel_platforms::prune::PlatformPrunePort + '_, RuntimeCapabilityError> {
        self.resolve(platform, None, false, cancel).await
    }

    pub async fn networks(
        &self,
        platform: uuid::Uuid,
        node: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<
        impl citadel_platforms::NetworkInventoryPort
        + citadel_platforms::NetworkObservationPort
        + citadel_platforms::NetworkMutationPort
        + '_,
        RuntimeCapabilityError,
    > {
        self.resolve(platform, node, false, cancel).await
    }
    pub async fn volumes(
        &self,
        platform: uuid::Uuid,
        node: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<
        impl citadel_platforms::VolumeInventoryPort
        + citadel_platforms::VolumeObservationPort
        + citadel_platforms::VolumeMutationPort
        + '_,
        RuntimeCapabilityError,
    > {
        self.resolve(platform, node, false, cancel).await
    }
    pub async fn container_inspection(
        &self,
        platform: uuid::Uuid,
        node: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<
        impl citadel_platforms::containers::ContainerInspectionPort + '_,
        RuntimeCapabilityError,
    > {
        self.resolve(platform, node, false, cancel).await
    }

    pub async fn images(
        &self,
        platform: uuid::Uuid,
        node: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<impl citadel_platforms::images::ImageInspectionPort + '_, RuntimeCapabilityError>
    {
        self.resolve(platform, node, false, cancel).await
    }

    pub async fn logs(
        &self,
        platform: uuid::Uuid,
        node: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<impl citadel_platforms::logs::LogReadPort + '_, RuntimeCapabilityError> {
        self.resolve(platform, node, false, cancel).await
    }

    pub async fn inventory(
        &self,
        platform: uuid::Uuid,
        cancel: &CancellationToken,
    ) -> Result<
        impl citadel_platforms::NetworkInventoryPort + citadel_platforms::VolumeInventoryPort + '_,
        RuntimeCapabilityError,
    > {
        self.resolve(platform, None, false, cancel).await
    }

    pub async fn tasks(
        &self,
        platform: uuid::Uuid,
        cancel: &CancellationToken,
    ) -> Result<impl citadel_platforms::SwarmTaskRuntimePort + '_, RuntimeCapabilityError> {
        self.resolve(platform, None, false, cancel).await
    }

    pub fn streams(
        &self,
        platform: uuid::Uuid,
        node: Option<&str>,
    ) -> impl citadel_platforms::logs::ContainerLogPort
    + citadel_platforms::terminal::ContainerTerminalPort
    + use<> {
        super::streams::RoutedContainerStreams {
            router: self.clone(),
            platform,
            node: node.map(str::to_owned),
        }
    }

    pub fn new(
        pool: PgPool,
        docker: DockerClient,
        agent: Option<AgentClient>,
        edge: EdgeRegistry,
    ) -> Self {
        let resolver = super::registry::PlatformRuntimeRegistry::new(pool.clone(), agent);
        Self {
            pool,
            docker,
            edge,
            resolver,
        }
    }

    pub fn with_agent_resolver(mut self, resolver: Arc<dyn PlatformAgentResolver>) -> Self {
        self.resolver = resolver;
        self
    }

    pub fn with_edge(mut self, edge: EdgeRegistry) -> Self {
        self.edge = edge;
        self
    }

    pub(crate) async fn execution_agent(
        &self,
        platform_id: uuid::Uuid,
        cancellation: &CancellationToken,
    ) -> Result<crate::connectors::agent::execution::AgentExecutionClient, RuntimeCapabilityError>
    {
        use crate::connectors::agent::execution::AgentExecutionClient;
        match self.resolve(platform_id, None, false, cancellation).await? {
            Runtime::Agent(agent) => Ok(AgentExecutionClient::Direct(Arc::new(agent))),
            Runtime::Edge(runtime) => Ok(AgentExecutionClient::Edge(runtime.session)),
            Runtime::Local(_) => Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Unavailable,
                "The configured Agent transport is unavailable.",
                false,
            )),
        }
    }

    pub(crate) async fn execution_agent_for(
        &self,
        connection: &PlatformConnection,
        cancellation: &CancellationToken,
    ) -> Result<crate::connectors::agent::execution::AgentExecutionClient, RuntimeCapabilityError>
    {
        use crate::connectors::agent::execution::AgentExecutionClient;
        match self
            .resolve_connection(connection, None, false, cancellation)
            .await?
        {
            Runtime::Agent(agent) => Ok(AgentExecutionClient::Direct(Arc::new(agent))),
            Runtime::Edge(runtime) => Ok(AgentExecutionClient::Edge(runtime.session)),
            Runtime::Local(_) => Err(unavailable()),
        }
    }

    pub async fn resolve(
        &self,
        platform_id: uuid::Uuid,
        node_id: Option<&str>,
        require_online: bool,
        cancellation: &CancellationToken,
    ) -> Result<Runtime<'_>, RuntimeCapabilityError> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled, "Platform operation cancelled.", false)),
            result = self.resolve_inner(platform_id, node_id, require_online, cancellation) => result,
        }
    }

    async fn resolve_inner(
        &self,
        platform_id: uuid::Uuid,
        node_id: Option<&str>,
        require_online: bool,
        cancellation: &CancellationToken,
    ) -> Result<Runtime<'_>, RuntimeCapabilityError> {
        let connection = connection::load(&self.pool, platform_id)
            .await
            .map_err(storage)?
            .ok_or_else(|| {
                if require_online {
                    unavailable()
                } else {
                    RuntimeCapabilityError::new(
                        RuntimeErrorKind::NotFound,
                        "Platform not found",
                        false,
                    )
                }
            })?;
        if require_online && connection.status != citadel_primitives::PlatformStatus::Online {
            return Err(unavailable());
        }
        self.resolve_connection(&connection, node_id, require_online, cancellation)
            .await
    }

    async fn resolve_connection<'a>(
        &'a self,
        connection: &PlatformConnection,
        node_id: Option<&str>,
        require_online: bool,
        cancellation: &CancellationToken,
    ) -> Result<Runtime<'a>, RuntimeCapabilityError> {
        let platform_id = connection.id;
        if let Some(node) = node_id {
            let stale = connection::node_stale(&self.pool, platform_id, node)
                .await
                .map_err(storage)?;
            if stale.is_none() && !require_online {
                return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::NotFound,
                    "Swarm Node not found.",
                    false,
                ));
            }
            if stale != Some(false) {
                return Err(unavailable());
            }
            if let Ok(session) = self
                .edge
                .get(&EdgeTarget::node(platform_id, node.to_owned()))
            {
                return Ok(Runtime::Edge(EdgeRuntime { session }));
            }
            if connection.node_id.as_deref() != Some(node) {
                return Err(unavailable());
            }
        }
        let address = &connection.address;
        let runtime = match connection.connector {
            citadel_platforms::ConnectorKind::Local => Runtime::Local(&self.docker),
            citadel_platforms::ConnectorKind::Agent => Runtime::Agent(
                tokio::select! {
                    () = cancellation.cancelled() => return Err(RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled, "Platform operation cancelled.", false)),
                    value = self.resolver.resolve_agent(platform_id, address) => value?,
                }
                .as_ref()
                .clone(),
            ),
            citadel_platforms::ConnectorKind::EdgeAgent => Runtime::Edge(EdgeRuntime {
                session: self
                    .edge
                    .get(&EdgeTarget::platform(platform_id))
                    .map_err(|_| unavailable())?,
            }),
        };
        if let Some(node) = node_id {
            let info = match &runtime {
                Runtime::Local(r) => r.get_info(cancellation).await,
                Runtime::Agent(r) => r.get_info(cancellation).await,
                Runtime::Edge(r) => r.get_info(cancellation).await,
            }?;
            if !info.swarm.is_some_and(|s| {
                s.node_id == node && s.local_node_state.eq_ignore_ascii_case("active")
            }) {
                return Err(unavailable());
            }
        }
        Ok(runtime)
    }
}

fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), true)
}
fn unavailable() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Unavailable,
        "The owning Platform or Node Agent is disconnected or unavailable.",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelled_resolution_does_not_query_or_open_a_channel() {
        let pool = PgPool::connect_lazy("postgres://unused:unused@127.0.0.1:1/unused").unwrap();
        let docker = DockerClient::new(
            std::path::Path::new("/tmp/citadel-unused-routing.sock"),
            std::time::Duration::from_secs(1),
        )
        .unwrap();
        let router = PlatformRuntimeRouter::new(pool, docker, None, EdgeRegistry::default());
        let cancel = CancellationToken::new();
        cancel.cancel();
        let result = router
            .resolve(uuid::Uuid::now_v7(), None, true, &cancel)
            .await;
        assert!(matches!(result, Err(error) if error.kind == RuntimeErrorKind::Cancelled));
        let id = uuid::Uuid::now_v7();
        assert!(
            matches!(router.image_mutations(id, &cancel).await, Err(error) if error.kind == RuntimeErrorKind::Cancelled)
        );
        assert!(
            matches!(router.pruning(id, &cancel).await, Err(error) if error.kind == RuntimeErrorKind::Cancelled)
        );
        assert!(
            matches!(router.networks(id, Some("node"), &cancel).await, Err(error) if error.kind == RuntimeErrorKind::Cancelled)
        );
        assert!(
            matches!(router.volumes(id, Some("node"), &cancel).await, Err(error) if error.kind == RuntimeErrorKind::Cancelled)
        );

        assert!(
            matches!(router.container_inspection(id, Some("node"), &cancel).await, Err(error) if error.kind == RuntimeErrorKind::Cancelled)
        );
        assert!(
            matches!(router.images(id, Some("node"), &cancel).await, Err(error) if error.kind == RuntimeErrorKind::Cancelled)
        );
        assert!(
            matches!(router.logs(id, Some("node"), &cancel).await, Err(error) if error.kind == RuntimeErrorKind::Cancelled)
        );
        assert!(
            matches!(router.tasks(id, &cancel).await, Err(error) if error.kind == RuntimeErrorKind::Cancelled)
        );
    }
}
