//! Actor-specific relationships shared by HTTP and daemon snapshots.
use super::views::NetworkView;
use citadel_identity::ActorPrincipal;
use citadel_platforms::{
    AuthorizedReadError, PlatformReadService,
    network_usage::{NetworkRelationship, NetworkUsageQuery},
};
use uuid::Uuid;

pub(crate) async fn enrich_networks(
    reader: &PlatformReadService,
    principal: &ActorPrincipal,
    platform: Uuid,
    networks: &mut [NetworkView],
) -> Result<(), AuthorizedReadError> {
    let queries: Vec<_> = networks
        .iter()
        .map(|network| NetworkUsageQuery {
            id: network.id.clone(),
            name: network.name.clone(),
            scope: network.scope.clone(),
            docker_node_id: network.docker_node_id.clone(),
            container_ids: network.containers.keys().cloned().collect(),
            stack_project: network
                .labels
                .get("com.docker.stack.namespace")
                .or_else(|| network.labels.get("com.docker.compose.project"))
                .filter(|project| !project.is_empty())
                .cloned(),
        })
        .collect();
    let usage = reader
        .network_usage(
            principal.actor_id,
            principal.is_administrator(),
            platform,
            &queries,
        )
        .await?;
    for (network, usage) in networks.iter_mut().zip(usage) {
        // Counts, endpoint names and addresses must follow the same permissions
        // as workload links. Missing/unresolved ownership is not a public owner.
        network.containers.retain(|id, _| {
            usage.entries.iter().any(|entry| {
                entry.relationship == NetworkRelationship::Attachment
                    && entry.docker_container_id.as_ref() == Some(id)
            })
        });
        network.usage = Some(usage);
    }
    Ok(())
}
