//! Minimal networking presentation of the existing normalized inspection.
use crate::api::resources::schema_models::network_usage::NetworkContainerSchema;
use citadel_platforms::network_usage::NetworkContainer;
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkContainersView {
    #[schema(value_type=Vec<NetworkContainerSchema>)]
    pub containers: Vec<NetworkContainer>,
    pub truncated: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerNetworkingView {
    #[schema(value_type=NetworkContainerSchema)]
    pub container: NetworkContainer,
    pub mode: String,
    pub endpoints: Vec<NetworkEndpointView>,
    pub published_ports: Vec<PublishedPortView>,
    pub observed_at: i64,
    pub truncated: bool,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkEndpointView {
    /// None means the runtime did not provide endpoint evidence.
    pub attached: Option<bool>,
    pub network_id: String,
    pub name: String,
    pub addresses: Vec<String>,
    pub aliases: Vec<String>,
}
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublishedPortView {
    pub container_port: String,
    pub host_ip: String,
    pub host_port: String,
}
impl ContainerNetworkingView {
    pub fn from_inspection(value: &Value, container: NetworkContainer) -> Self {
        let mut truncated = false;
        let mode = match value
            .pointer("/hostConfig/networkMode")
            .and_then(Value::as_str)
        {
            Some("host") => "host",
            Some("none") => "none",
            Some(mode) if mode.starts_with("container:") => "shared",
            Some(mode) if !mode.is_empty() => "networks",
            _ => "unknown",
        }
        .to_owned();
        let networks = value
            .pointer("/networkSettings/networks")
            .and_then(Value::as_object);
        truncated |= networks.is_some_and(|v| v.len() > 50);
        let endpoints = networks
            .into_iter()
            .flatten()
            .take(50)
            .map(|(name, endpoint)| {
                let aliases = endpoint.get("aliases").and_then(Value::as_array);
                truncated |= aliases.is_some_and(|v| v.len() > 50);
                NetworkEndpointView {
                    attached: endpoint
                        .get("endpointID")
                        .and_then(Value::as_str)
                        .map(|id| !id.is_empty()),
                    network_id: endpoint
                        .get("networkID")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    name: name.clone(),
                    addresses: ["ipAddress", "globalIPv6Address"]
                        .into_iter()
                        .filter_map(|k| endpoint.get(k)?.as_str())
                        .filter(|v| !v.is_empty())
                        .map(str::to_owned)
                        .collect(),
                    aliases: aliases
                        .into_iter()
                        .flatten()
                        .take(50)
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect(),
                }
            })
            .collect();
        let mut published_ports = Vec::new();
        for (port, bindings) in value
            .pointer("/networkSettings/ports")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            for binding in bindings.as_array().into_iter().flatten() {
                if published_ports.len() == 100 {
                    truncated = true;
                    break;
                }
                if let Some(host_port) = binding.get("hostPort").and_then(Value::as_str) {
                    published_ports.push(PublishedPortView {
                        container_port: port.clone(),
                        host_port: host_port.into(),
                        host_ip: binding
                            .get("hostIP")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .into(),
                    });
                }
            }
        }
        Self {
            container,
            mode,
            endpoints,
            published_ports,
            observed_at: chrono::Utc::now().timestamp(),
            truncated,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn project(value: &Value) -> ContainerNetworkingView {
        ContainerNetworkingView::from_inspection(
            value,
            NetworkContainer {
                container_id: uuid::Uuid::nil(),
                docker_container_id: "docker-id".into(),
                docker_node_id: None,
                container_name: "app".into(),
                container_state: Some("Running".into()),
                is_stale: false,
                workload_kind: None,
                workload_id: None,
                workload_name: None,
                workload_status: None,
                workload_processing: false,
            },
        )
    }
    #[test]
    fn projects_networking_without_configuration_or_namespace_target_leaks() {
        let value = json!({"config":{"env":["SECRET=private"]},"hostConfig":{"networkMode":"container:hidden-id"},"networkSettings":{"networks":{"backend":{"networkID":"full-id","ipAddress":"172.20.0.2","globalIPv6Address":"fd00::2","aliases":["db"]}},"ports":{"80/tcp":[{"hostIP":"0.0.0.0","hostPort":"8080"}]}}});
        let result = serde_json::to_value(project(&value)).unwrap();
        assert_eq!(result["mode"], "shared");
        assert_eq!(result["container"]["containerState"], "Running");
        assert_eq!(result["container"]["isStale"], false);
        assert_eq!(result["endpoints"][0]["aliases"], json!(["db"]));
        assert_eq!(
            result["endpoints"][0]["addresses"],
            json!(["172.20.0.2", "fd00::2"])
        );
        assert_eq!(result["publishedPorts"][0]["hostPort"], "8080");
        assert!(!result.to_string().contains("hidden-id"));
        assert!(!result.to_string().contains("SECRET"));
    }
    #[test]
    fn distinguishes_modes_and_bounds_endpoints() {
        let stopped = project(
            &json!({"networkSettings":{"networks":{"bridge":{"networkID":"retained","endpointID":""}}}}),
        );
        assert_eq!(stopped.endpoints[0].attached, Some(false));
        let missing =
            project(&json!({"networkSettings":{"networks":{"bridge":{"networkID":"retained"}}}}));
        assert_eq!(missing.endpoints[0].attached, None);
        for mode in ["host", "none"] {
            assert_eq!(
                project(&json!({"hostConfig":{"networkMode":mode}})).mode,
                mode
            );
        }
        assert_eq!(project(&json!({})).mode, "unknown");
        let networks: serde_json::Map<_, _> = (0..200)
            .map(|i| (i.to_string(), json!({"networkID":i.to_string()})))
            .collect();
        let result = project(&json!({"networkSettings":{"networks":networks}}));
        assert!(result.truncated);
        assert_eq!(result.endpoints.len(), 50);
    }
}
