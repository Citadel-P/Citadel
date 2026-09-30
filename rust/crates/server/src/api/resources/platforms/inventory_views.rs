//! Native public inventory fields shared by HTTP and realtime projections.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum ContainerStateStatus {
    Unknown,
    Created,
    Running,
    Paused,
    Restarting,
    Exited,
    Removing,
    Dead,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostPortBinding {
    #[serde(rename = "hostIP", skip_serializing_if = "Option::is_none")]
    pub host_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_port: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAttachmentView {
    #[schema(required = true)]
    pub name: Option<String>,
    #[schema(required = true)]
    pub endpoint_id: Option<String>,
    #[schema(required = true)]
    pub mac_address: Option<String>,
    #[serde(rename = "ipV4Address")]
    #[schema(required = true)]
    pub ipv4_address: Option<String>,
    #[schema(required = true)]
    pub ipv6_address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPeerView {
    #[schema(required = true)]
    pub name: Option<String>,
    #[schema(required = true)]
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VolumeContainerView {
    pub id: String,
    pub name: String,
    pub image: String,
    pub image_id: String,
    pub state: ContainerStateStatus,
    pub networks: BTreeMap<String, String>,
    pub ports: BTreeMap<String, Vec<HostPortBinding>>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreatedNetworkView {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkIpamView {
    #[schema(required = true)]
    pub driver: Option<String>,
    pub config: Vec<super::requests::RuntimeIpamConfig>,
    pub options: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::resources::platforms::{runtime_mapping, views::VolumeView};
    use serde_json::json;

    #[test]
    fn network_projection_preserves_absent_fields_and_dictionary_keys() {
        let ipam = runtime_mapping::ipam(json!({"Options":{"Mixed.Key":"value"}}))
            .unwrap()
            .unwrap();
        assert!(ipam.driver.is_none());
        assert!(ipam.config.is_empty());
        assert_eq!(ipam.options["Mixed.Key"], "value");
        assert!(
            runtime_mapping::ipam(serde_json::Value::Null)
                .unwrap()
                .is_none()
        );
        let attachment =
            runtime_mapping::network_container(json!({"Name":"web", "EndpointID":"endpoint"}))
                .unwrap();
        let wire = serde_json::to_value(attachment).unwrap();
        assert_eq!(wire["name"], "web");
        assert_eq!(wire["endpointId"], "endpoint");
        assert!(wire["ipV4Address"].is_null());
        assert!(wire["ipv6Address"].is_null());
        assert!(runtime_mapping::network_container(json!({"Name":42})).is_err());
    }

    #[test]
    fn volume_container_contract_preserves_port_protocols_and_rejects_invalid_state() {
        let wire = json!({"id":"container", "name":"web", "image":"nginx", "imageId":"sha256:image", "state":"Running", "networks":{"Mixed.Network":"network-id"}, "ports":{"80/tcp":[{"hostIP":"::", "hostPort":"8080"}], "53/udp":[]}});
        let view: VolumeContainerView = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(view).unwrap(), wire);
        let mut invalid = wire;
        invalid["state"] = json!("UnknownNewState");
        assert!(serde_json::from_value::<VolumeContainerView>(invalid).is_err());
        // Invalid attachment data fails the whole projection rather than losing an attachment.
        let details = citadel_platforms::VolumeDetails {
            containers: vec![json!({"id":"incomplete"})],
            ..Default::default()
        };
        assert!(VolumeView::try_from(details).is_err());
    }

    #[test]
    fn inventory_routes_publish_native_network_and_swarm_schemas() {
        let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
        let response = |path: &str, method: &str| {
            doc["paths"][path][method]["responses"]["200"]["content"]["application/json"]["schema"]
                ["$ref"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        assert_eq!(
            response("/api/v1/networks/{platformId}/{networkId}", "get"),
            "#/components/schemas/NetworkView"
        );
        assert_eq!(
            response("/api/v1/networks", "post"),
            "#/components/schemas/CreatedNetworkView"
        );
        for kind in [
            "nodes", "services", "tasks", "networks", "configs", "secrets",
        ] {
            assert!(
                response(
                    &format!("/api/v1/platforms/{{platformId}}/swarm/{kind}"),
                    "get"
                )
                .contains("SwarmItemsResponse")
            );
        }
        let schemas = &doc["components"]["schemas"];
        assert_eq!(
            schemas["VolumeView"]["properties"]["containers"]["items"]["$ref"],
            "#/components/schemas/VolumeContainerView"
        );
        assert_eq!(
            schemas["NetworkView"]["properties"]["containers"]["additionalProperties"]["$ref"],
            "#/components/schemas/NetworkAttachmentView"
        );
    }
}
