use super::{Runtime, docker_error, filters, strings, text};
use citadel_adapters::connectors::docker::projection::{DockerNetwork, NetworkCreateRequest};
use citadel_contracts::citadel::{networks::v1::*, shared_models::v1::*};
use serde_json::{Value, json};
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl network_service_server::NetworkService for Runtime {
    async fn list(
        &self,
        request: Request<ListNetworksRequest>,
    ) -> Result<Response<ListNetworksResponse>, Status> {
        let request = request.into_inner();
        let filter = filters([
            ("dangling", request.dangling.map(|v| v.to_string())),
            ("driver", request.driver),
            ("id", request.id),
            ("name", request.name),
        ]);
        let networks = self
            .docker
            .list_networks_filtered(filter.as_deref())
            .await
            .map_err(docker_error)?;
        // Docker does not always include attached containers in list results.
        let containers = self
            .docker
            .list_container_models(Some(true), None, None, None)
            .await
            .map_err(docker_error)?;
        let used: std::collections::HashSet<_> = containers
            .iter()
            .flat_map(|c| {
                c.network_settings
                    .as_ref()
                    .and_then(|n| n.networks.as_ref())
                    .into_iter()
                    .flatten()
                    .filter_map(|(_, n)| n.network_id.clone())
            })
            .collect();
        Ok(Response::new(ListNetworksResponse {
            networks: networks
                .into_iter()
                .map(|value| {
                    let in_use = !value.containers.is_empty() || used.contains(&value.id);
                    network(value, in_use)
                })
                .collect(),
        }))
    }
    async fn create(
        &self,
        request: Request<CreateNetworkRequest>,
    ) -> Result<Response<CreateNetworkResponse>, Status> {
        let r = request.into_inner();
        let input = NetworkCreateRequest {
            name: r.name, driver: r.driver, scope: r.scope, internal: r.internal, attachable: r.attachable, ingress: r.ingress,
            config_only: r.config_only, enable_ipv4: r.enable_i_pv4, enable_ipv6: r.enable_i_pv6,
            config_from: r.config_from.filter(|v| !v.network.is_empty()).map(|v| json!({"Network":v.network})),
            ipam: r.ipam.map(|v| json!({"Driver":v.driver,"Options":v.options,"Config":v.config.into_iter().map(|c| {
                let mut value = serde_json::Map::new();
                for (key, field) in [("Subnet", c.subnet), ("IPRange", c.ip_range), ("Gateway", c.gateway)] {
                    if let Some(field) = field { value.insert(key.into(), Value::String(field)); }
                }
                Value::Object(value)
            }).collect::<Vec<_>>()})),
            options: r.options, labels: r.labels,
        };
        Ok(Response::new(CreateNetworkResponse {
            id: self
                .docker
                .create_network(&input)
                .await
                .map_err(docker_error)?
                .id,
        }))
    }
    async fn delete(
        &self,
        request: Request<DeleteNetworkRequest>,
    ) -> Result<Response<DeleteNetworkResponse>, Status> {
        let request = request.into_inner();
        if request.ids.iter().any(|id| id.trim().is_empty()) {
            return Err(Status::invalid_argument("Network ids must not be empty"));
        }
        for id in request.ids {
            self.docker
                .delete_network(&id)
                .await
                .map_err(docker_error)?;
        }
        Ok(Response::new(DeleteNetworkResponse {}))
    }
    async fn inspect(
        &self,
        request: Request<InspectNetworkRequest>,
    ) -> Result<Response<InspectNetworkResponse>, Status> {
        let v = self
            .docker
            .inspect_network(&request.into_inner().id)
            .await
            .map_err(docker_error)?;
        Ok(Response::new(InspectNetworkResponse {
            name: v.name,
            id: v.id,
            created: v.created,
            driver: v.driver,
            scope: v.scope,
            enable_i_pv4: v.enable_ipv4,
            enable_i_pv6: v.enable_ipv6,
            internal: v.internal,
            attachable: v.attachable,
            ingress: v.ingress,
            config_only: v.config_only,
            config_from: v
                .config_from
                .as_ref()
                .and_then(|v| v["Network"].as_str().map(str::to_owned)),
            ipam: v.ipam.as_ref().map(ipam),
            options: v.options,
            labels: v.labels,
            containers: v
                .containers
                .into_iter()
                .map(|(key, c)| {
                    (
                        key,
                        NetworkContainerMessage {
                            name: text(&c, "Name"),
                            endpoint_id: text(&c, "EndpointID"),
                            mac_address: text(&c, "MacAddress"),
                            ip_pv4_address: text(&c, "IPv4Address"),
                            ipv6_address: text(&c, "IPv6Address"),
                        },
                    )
                })
                .collect(),
            peers: v
                .peers
                .into_iter()
                .map(|v| PeerInfoMessage {
                    name: text(&v, "Name"),
                    ip: text(&v, "IP"),
                })
                .collect(),
        }))
    }
}

pub(super) fn network(v: DockerNetwork, in_use: bool) -> Network {
    Network {
        name: v.name,
        id: v.id,
        created: v.created,
        driver: v.driver,
        scope: v.scope,
        enable_i_pv4: v.enable_ipv4,
        enable_i_pv6: v.enable_ipv6,
        internal: v.internal,
        attachable: v.attachable,
        ingress: v.ingress,
        config_only: v.config_only,
        in_use,
        config_from: v
            .config_from
            .as_ref()
            .and_then(|v| v["Network"].as_str().map(str::to_owned)),
        ipam: v.ipam.as_ref().map(ipam),
        options: v.options,
        labels: v.labels,
    }
}
fn ipam(v: &Value) -> IpamMessage {
    IpamMessage {
        driver: v["Driver"].as_str().map(str::to_owned),
        options: strings(&v["Options"]),
        config: v["Config"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| IpamConfigMessage {
                subnet: c["Subnet"].as_str().map(str::to_owned),
                ip_range: c["IPRange"].as_str().map(str::to_owned),
                gateway: c["Gateway"].as_str().map(str::to_owned),
            })
            .collect(),
    }
}
