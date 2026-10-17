use std::collections::HashMap;

use citadel_docker_api::models::ContainerSummary;
use serde_json::json;

use super::projection::DockerNetwork;

/// Carry actual endpoint evidence from the container list already fetched for
/// usage counts. A configured network without an endpoint is not an attachment.
pub fn populate_network_attachments(
    networks: &mut [DockerNetwork],
    containers: &[ContainerSummary],
) {
    let indices: HashMap<_, _> = networks
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();
    for container in containers {
        let Some(id) = container.id.as_deref().filter(|id| !id.is_empty()) else {
            continue;
        };
        for endpoint in container
            .network_settings
            .as_ref()
            .and_then(|n| n.networks.as_ref())
            .into_iter()
            .flat_map(|n| n.values())
        {
            let Some(network_id) = endpoint.network_id.as_deref().filter(|id| !id.is_empty())
            else {
                continue;
            };
            let Some(endpoint_id) = endpoint.endpoint_id.as_deref().filter(|id| !id.is_empty())
            else {
                continue;
            };
            let Some(index) = indices.get(network_id) else {
                continue;
            };
            networks[*index].containers.entry(id.to_owned()).or_insert_with(|| json!({
                "Name": container.names.as_ref().and_then(|names| names.first()).map(|name| name.trim_start_matches('/')).unwrap_or(id),
                "EndpointID": endpoint_id,
                "MacAddress": endpoint.mac_address.as_deref().unwrap_or_default(),
                "IPv4Address": address(endpoint.ip_address.as_deref(), endpoint.ip_prefix_len.map(i64::from)),
                "IPv6Address": address(endpoint.global_ipv6_address.as_deref(), endpoint.global_ipv6_prefix_len),
            }));
        }
    }
}

fn address(ip: Option<&str>, prefix: Option<i64>) -> String {
    match (ip.filter(|ip| !ip.is_empty()), prefix) {
        (Some(ip), Some(prefix)) => format!("{ip}/{prefix}"),
        (Some(ip), None) => ip.to_owned(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_attachments_require_endpoint_and_network_identity() {
        let mut networks: Vec<DockerNetwork> = serde_json::from_value(json!([
            {"Id":"network-id", "Name":"backend"},
            {"Id":"other-id", "Name":"backend"}
        ]))
        .unwrap();
        let containers: Vec<ContainerSummary> = serde_json::from_value(json!([
            {"Id":"running", "Names":["/web"], "NetworkSettings":{"Networks":{"backend":{
                "NetworkID":"network-id", "EndpointID":"endpoint", "IPAddress":"172.20.0.2", "IPPrefixLen":16,
                "GlobalIPv6Address":"fd00::2", "GlobalIPv6PrefixLen":64
            }}}},
            {"Id":"stopped", "NetworkSettings":{"Networks":{"backend":{"NetworkID":"network-id","EndpointID":""}}}},
            {"Id":"missing-id", "NetworkSettings":{"Networks":{"backend":{"EndpointID":"endpoint"}}}}
        ])).unwrap();
        populate_network_attachments(&mut networks, &containers);
        assert_eq!(networks[0].containers.len(), 1);
        assert!(networks[1].containers.is_empty());
        let endpoint = &networks[0].containers["running"];
        assert_eq!(endpoint["Name"], "web");
        assert_eq!(endpoint["IPv4Address"], "172.20.0.2/16");
        assert_eq!(endpoint["IPv6Address"], "fd00::2/64");
        networks[0].containers.get_mut("running").unwrap()["Name"] = json!("inspected-name");
        populate_network_attachments(&mut networks, &containers);
        assert_eq!(networks[0].containers["running"]["Name"], "inspected-name");
    }
}
