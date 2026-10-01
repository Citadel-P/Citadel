use crate::AuthorizedReadError;
use crate::*;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use std::collections::BTreeMap;
use uuid::Uuid;

pub trait PlatformReader: Send + Sync {
    fn platform_kind(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<PlatformKind>, AuthorizedReadError>>;
    fn has_swarm_nodes(&self, platform: Uuid) -> BoxFuture<'_, Result<bool, AuthorizedReadError>>;
    /// At most two matches: callers must reject ambiguous deployment identity.
    fn deployment_container_ids(
        &self,
        deployment: Uuid,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, AuthorizedReadError>>;
    fn stack_container_id<'a>(
        &'a self,
        stack: Uuid,
        reference: &'a str,
    ) -> BoxFuture<'a, Result<Option<Uuid>, AuthorizedReadError>>;
    fn image_identity(
        &self,
        platform: Uuid,
        image: Uuid,
    ) -> BoxFuture<'_, Result<Option<ImageIdentity>, AuthorizedReadError>>;
    fn image_registry_id<'a>(
        &'a self,
        platform: Uuid,
        docker_image: &'a str,
    ) -> BoxFuture<'a, Result<Option<Uuid>, AuthorizedReadError>>;

    fn container_identities(
        &self,
        platform: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerIdentity>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(self
                .list_containers(platform)
                .await?
                .into_iter()
                .map(|c| ContainerIdentity {
                    id: c.id,
                    platform_id: c.platform_id,
                    deployment_id: c.deployment_id,
                    stack_id: c.stack_id,
                    container_id: c.container_id,
                    docker_node_id: c.docker_node_id,
                })
                .collect())
        })
    }
    fn platform_telemetry(
        &self,
        platform: Uuid,
    ) -> BoxFuture<'_, Result<Option<PlatformTelemetryContext>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(self
                .get_platform(platform)
                .await?
                .map(|p| PlatformTelemetryContext {
                    cpu_count: p.cpu_count,
                    mem_total: p.mem_total,
                    network_count: p.network_count,
                    volume_count: p.volume_count,
                    image_count: p.image_count,
                    descriptor: p.platform_descriptor.metadata,
                }))
        })
    }

    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        is_administrator: bool,
        tags: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<PlatformDetails>, AuthorizedReadError>>;

    fn get_platform(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<PlatformDetails>, AuthorizedReadError>>;

    fn swarm_summary(
        &self,
        platform: Uuid,
    ) -> BoxFuture<'_, Result<crate::swarm_overview::SwarmSummary, AuthorizedReadError>>;

    fn permissions_for_platforms<'a>(
        &'a self,
        actor_id: ActorId,
        platform_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<BTreeMap<Uuid, EffectivePlatformPermission>, AuthorizedReadError>>;

    fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerDetails>, AuthorizedReadError>>;

    fn get_container(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<ContainerDetails>, AuthorizedReadError>>;

    fn containers_by_runtime_id<'a>(
        &'a self,
        platform_id: Uuid,
        docker_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<ContainerDetails>, AuthorizedReadError>> {
        Box::pin(async move {
            self.containers_by_runtime_ids(platform_id, &[docker_id.to_owned()])
                .await
        })
    }

    fn containers_by_runtime_ids<'a>(
        &'a self,
        platform_id: Uuid,
        docker_ids: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<ContainerDetails>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(self
                .list_containers(platform_id)
                .await?
                .into_iter()
                .filter(|container| docker_ids.contains(&container.container_id))
                .collect())
        })
    }

    fn list_stack_containers(
        &self,
        stack_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerDetails>, AuthorizedReadError>>;

    fn resolve_container_reference<'a>(
        &'a self,
        reference: &'a str,
    ) -> BoxFuture<'a, Result<Option<Uuid>, AuthorizedReadError>> {
        Box::pin(async move { Ok(Uuid::parse_str(reference).ok()) })
    }

    fn list_images(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ImageDetails>, AuthorizedReadError>>;

    fn list_node_volumes(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<
        '_,
        Result<Vec<NodeResourceProjection<crate::RuntimeVolumeSummary>>, AuthorizedReadError>,
    >;

    fn list_node_networks(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<
        '_,
        Result<Vec<NodeResourceProjection<crate::RuntimeNetworkSummary>>, AuthorizedReadError>,
    >;

    fn list_swarm_nodes(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmNodeSummary>, AuthorizedReadError>>;

    fn get_swarm_node<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmNodeSummary>, AuthorizedReadError>>;

    fn list_swarm_services(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmServiceSummary>, AuthorizedReadError>>;

    fn get_swarm_service<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmServiceSummary>, AuthorizedReadError>>;

    fn list_swarm_tasks<'a>(
        &'a self,
        platform_id: Uuid,
        service_id: Option<&'a str>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<SwarmTaskSummary>, AuthorizedReadError>>;

    fn get_swarm_task<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmTaskSummary>, AuthorizedReadError>>;

    fn list_swarm_configs(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmConfigSummary>, AuthorizedReadError>>;

    fn get_swarm_config<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmConfigSummary>, AuthorizedReadError>>;

    fn list_swarm_networks(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmNetworkSummary>, AuthorizedReadError>>;

    fn get_swarm_network<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmNetworkSummary>, AuthorizedReadError>>;

    fn list_swarm_secrets(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmSecretSummary>, AuthorizedReadError>>;

    fn get_swarm_secret<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmSecretSummary>, AuthorizedReadError>>;
}
