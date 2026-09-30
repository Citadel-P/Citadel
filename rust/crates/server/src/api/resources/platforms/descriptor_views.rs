//! Native platform metadata and the redacted Docker inspection envelope.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
pub enum PlatformDescriptor {
    Docker(DockerPlatformDescriptor),
    DockerSwarm(DockerSwarmPlatformDescriptor),
    Kubernetes(KubernetesPlatformDescriptor),
}

impl PlatformDescriptor {
    pub fn node_id(&self) -> Option<&str> {
        match self {
            Self::DockerSwarm(descriptor) => descriptor.node_id.as_deref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum PlatformConnectorType {
    Unknown,
    Local,
    Agent,
    EdgeAgent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DockerPlatformDescriptor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers_paused: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers_running: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers_stopped: Option<i64>,
    #[serde(default)]
    pub daemon_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_used_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_api_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_used_bytes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DockerSwarmPlatformDescriptor {
    #[serde(flatten)]
    pub docker: DockerPlatformDescriptor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_node_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managers: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_addr: Option<String>,
    #[serde(rename = "nodeID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Vec<crate::api::resources::schema_models::platforms::RuntimeSwarmPeerSchema>>)]
    pub remote_managers: Option<Vec<citadel_platforms::RuntimeSwarmPeer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running_task_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KubernetesPlatformDescriptor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_server_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerInspectionView {
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_armor_profile: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<BTreeMap<String, Value>>,
    pub created: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(rename = "execIDs")]
    #[serde(default)]
    pub exec_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_driver: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_config: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hosts_path: Option<String>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_label: Option<String>,
    #[serde(default)]
    pub mounts: Vec<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_settings: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolv_conf_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_root_fs: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_rw: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<BTreeMap<String, Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pending_edge_descriptors_do_not_require_a_first_inventory() {
        for kind in ["Docker", "DockerSwarm"] {
            let pending = json!({"$type":kind,"daemonId":""});
            let descriptor: PlatformDescriptor = serde_json::from_value(pending.clone()).unwrap();
            assert_eq!(serde_json::to_value(descriptor).unwrap(), pending);
        }
        assert!(serde_json::from_value::<PlatformDescriptor>(json!({"$type":"invalid"})).is_err());
        assert!(
            serde_json::from_value::<PlatformDescriptor>(
                json!({"$type":"Docker","containerCount":"bad"})
            )
            .is_err()
        );
    }

    #[test]
    fn swarm_descriptor_preserves_identity_counts_and_peers() {
        let wire = json!({"$type":"DockerSwarm","daemonId":"daemon","nodeID":"node",
            "nodeAddr":"10.0.0.2","localNodeState":"active","controlAvailable":true,
            "containerCount":7,"containersRunning":4,"containersStopped":2,"containersPaused":1,
            "nodes":3,"managers":3,"serviceCount":8,"runningTaskCount":9,
            "imageUsedBytes":9007199254740993_i64,"remoteManagers":[{"nodeId":"manager","address":"10.0.0.1:2377"}]});
        let descriptor: PlatformDescriptor = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(descriptor.node_id(), Some("node"));
        assert_eq!(serde_json::to_value(descriptor).unwrap(), wire);
    }

    #[test]
    fn inspection_envelope_preserves_redaction_and_docker_extension_fields() {
        let value = citadel_adapters::connectors::containers::inspection::map_inspection(json!({
            "Id":"container", "Created":"2026-09-30T10:00:00Z", "Name":"/web",
            "Config":{"Env":["API_TOKEN=secret","LOG_LEVEL=info"],"Labels":{"Mixed.Key":"value"}},
            "NetworkSettings":{"Networks":{"Mixed.Network":{"NetworkID":"network"}}},
            "FutureDiagnostic":{"Count":9007199254740993_i64}
        }), "container").unwrap();
        let inspection: ContainerInspectionView = serde_json::from_value(value).unwrap();
        let wire = serde_json::to_value(inspection).unwrap();
        assert_eq!(wire["config"]["env"][0], "API_TOKEN=********");
        assert_eq!(wire["config"]["labels"]["Mixed.Key"], "value");
        assert_eq!(
            wire["networkSettings"]["networks"]["Mixed.Network"]["networkID"],
            "network"
        );
        assert_eq!(wire["futureDiagnostic"]["count"], 9007199254740993_i64);
        assert_eq!(wire["execIDs"], json!([]));
        assert!(!wire.to_string().contains("secret"));
    }

    #[test]
    fn platform_and_inspection_schemas_are_native() {
        let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
        for path in [
            "/api/v1/containers/{id}/inspect",
            "/api/v1/deployments/{id}/inspect",
            "/api/v1/stacks/{stackId}/containers/{containerId}/inspect",
            "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/inspect",
        ] {
            assert_eq!(
                doc["paths"][path]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
                    ["$ref"],
                "#/components/schemas/ContainerInspectionView"
            );
        }
    }
}
