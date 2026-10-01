use super::{runtime_mapping, views::*};
use citadel_platforms::{RuntimeNetworkSummary, RuntimeVolumeSummary};

pub(crate) fn map_network(
    network: RuntimeNetworkSummary,
    capabilities: NetworkCapabilitiesView,
) -> Result<NetworkView, serde_json::Error> {
    let is_system = runtime_mapping::system_network(&network.name, network.ingress);
    Ok(NetworkView {
        name: network.name,
        id: network.id,
        created: network.created,
        driver: network.driver,
        scope: network.scope,
        enable_ipv4: network.enable_ipv4,
        enable_ipv6: network.enable_ipv6,
        internal: network.internal,
        attachable: network.attachable,
        ingress: network.ingress,
        config_only: network.config_only,
        in_use: network.container_count > 0,
        config_from: network.config_from,
        ipam: network
            .ipam
            .map(runtime_mapping::ipam)
            .transpose()?
            .flatten(),
        options: network.options,
        labels: network.labels,
        containers: network
            .containers
            .into_iter()
            .map(|(id, c)| runtime_mapping::network_container(c).map(|c| (id, c)))
            .collect::<Result<_, _>>()?,
        peers: network
            .peers
            .into_iter()
            .map(runtime_mapping::peer)
            .collect::<Result<_, _>>()?,
        is_system,
        docker_node_id: None,
        node_hostname: None,
        is_stale: false,
        stale_reason: None,
        capabilities: Some(capabilities),
    })
}

pub(crate) fn map_node_network(
    node: citadel_platforms::NodeResourceProjection<RuntimeNetworkSummary>,
    capabilities: NetworkCapabilitiesView,
) -> Result<NetworkView, serde_json::Error> {
    let mut view = map_network(node.resource, capabilities)?;
    view.docker_node_id = Some(node.docker_node_id);
    view.node_hostname = node.node_hostname;
    view.is_stale = node.is_stale;
    view.stale_reason = node
        .is_stale
        .then(|| "Node Agent is disconnected or unavailable.".into());
    Ok(view)
}

pub(crate) fn map_node_volume(
    node: citadel_platforms::NodeResourceProjection<RuntimeVolumeSummary>,
    capabilities: VolumeCapabilitiesView,
) -> Result<VolumeView, serde_json::Error> {
    let mut view = map_volume(node.resource, capabilities)?;
    view.docker_node_id = Some(node.docker_node_id);
    view.node_hostname = node.node_hostname;
    view.is_stale = node.is_stale;
    view.stale_reason = node
        .is_stale
        .then(|| "Node Agent is disconnected or unavailable.".into());
    Ok(view)
}

pub(crate) fn map_volume(
    volume: RuntimeVolumeSummary,
    capabilities: VolumeCapabilitiesView,
) -> Result<VolumeView, serde_json::Error> {
    Ok(VolumeView {
        backup_coverage: None,
        id: volume.name.clone(),
        name: volume.name,
        in_use: volume.in_use,
        scope: volume.scope,
        driver: volume.driver,
        mountpoint: volume.mountpoint,
        created_at: volume.created_at,
        cluster_volume: volume.cluster_volume.map(runtime_mapping::cluster_volume),
        usage_data: volume
            .usage_data
            .and_then(|usage| serde_json::from_value(usage).ok()),
        containers: volume
            .containers
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<_, _>>()?,
        status: volume
            .status
            .into_iter()
            .map(|(key, value)| {
                let value = if value.is_null() {
                    String::new()
                } else {
                    value
                        .as_str()
                        .map_or_else(|| value.to_string(), str::to_owned)
                };
                (key, value)
            })
            .collect(),
        labels: volume.labels,
        options: volume.options,
        docker_node_id: None,
        node_hostname: None,
        is_stale: false,
        stale_reason: None,
        capabilities: Some(capabilities),
    })
}

pub(crate) fn image_capabilities(platform: PlatformCapabilitiesView) -> ImageCapabilitiesView {
    ImageCapabilitiesView {
        can_read: platform.can_read,
        can_write: platform.can_write,
        can_execute: platform.can_execute,
        can_inspect: platform.can_inspect,
        can_pull: platform.can_pull,
    }
}

pub(crate) fn network_capabilities(platform: PlatformCapabilitiesView) -> NetworkCapabilitiesView {
    NetworkCapabilitiesView {
        can_read: platform.can_read,
        can_write: platform.can_write,
        can_execute: platform.can_execute,
        can_inspect: platform.can_inspect,
    }
}
