//! Container summary and live runtime contracts shared by HTTP and realtime.
use super::{
    inventory_views::{ContainerStateStatus, HostPortBinding},
    views::{
        ContainerDeploymentView, ContainerStatView, ContainerView, ImageView,
        PlatformCapabilitiesView,
    },
};
use citadel_primitives::ResourceControlState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum ContainerSystemRole {
    Core,
    Database,
    Agent,
    EdgeAgent,
}

impl ContainerSystemRole {
    fn from_label(role: &str) -> Option<Self> {
        // Docker labels are optional external metadata, not serialized enum values.
        match role.trim().to_ascii_lowercase().as_str() {
            "core" => Some(Self::Core),
            "database" => Some(Self::Database),
            "agent" => Some(Self::Agent),
            "edgeagent" | "edge-agent" => Some(Self::EdgeAgent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRuntimeView {
    /// Docker container ID; the statistics containerId is Citadel's database UUID.
    pub id: String,
    pub name: String,
    pub platform_id: Uuid,
    pub image: String,
    pub image_id: String,
    pub state: ContainerStateStatus,
    #[schema(value_type = crate::api::resources::schema_models::primitives::ResourceControlStateSchema)]
    pub control_state: ResourceControlState,
    pub created: i64,
    #[schema(required = true)]
    pub stack: Option<String>,
    #[schema(required = true)]
    pub ports: Option<BTreeMap<String, Option<Vec<HostPortBinding>>>>,
    #[schema(required = true)]
    pub container_stat: Option<ContainerStatView>,
    pub is_system: bool,
    #[schema(required = true)]
    pub system_role: Option<ContainerSystemRole>,
    pub has_citadel_ownership_labels: bool,
    pub is_swarm_task: bool,
    #[schema(required = true)]
    pub docker_node_id: Option<String>,
    #[schema(required = true)]
    pub deployment_id: Option<Uuid>,
    #[schema(required = true)]
    pub stack_id: Option<Uuid>,
    #[schema(required = true)]
    pub capabilities: Option<PlatformCapabilitiesView>,
}

impl ContainerView {
    pub fn runtime_data(&self) -> Result<ContainerRuntimeView, serde_json::Error> {
        Ok(ContainerRuntimeView {
            id: self.container_id.clone(),
            name: self.name.clone(),
            platform_id: self.platform_id,
            image: self
                .image_view
                .as_ref()
                .map_or_else(String::new, |image| image.name.clone()),
            image_id: self.docker_image_id.clone(),
            state: serde_json::from_value(Value::String(self.state.clone()))?,
            control_state: self.control_state,
            created: self.created,
            stack: self.stack.clone(),
            ports: serde_json::from_value(self.ports.clone())?,
            container_stat: self.last_stats.clone(),
            is_system: self.is_system,
            system_role: self
                .system_role
                .as_deref()
                .filter(|_| self.is_system)
                .and_then(ContainerSystemRole::from_label),
            has_citadel_ownership_labels: self.has_citadel_ownership_labels,
            is_swarm_task: self.is_swarm_task,
            docker_node_id: self.docker_node_id.clone(),
            deployment_id: self.deployment_id,
            stack_id: self.stack_id,
            capabilities: self.capabilities,
        })
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRuntimeListView {
    pub containers: Vec<ContainerRuntimeView>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerSummaryView {
    pub name: String,
    pub container_id: String,
    pub platform_id: Uuid,
    pub platform_name: String,
    pub started_at: String,
    pub finished_at: String,
    pub volumes: Vec<String>,
    pub networks: BTreeMap<String, String>,
    pub ports: BTreeMap<String, Option<Vec<HostPortBinding>>>,
    pub state: ContainerStateStatus,
    #[schema(required = true)]
    pub image_view: Option<ImageView>,
    #[schema(required = true)]
    pub deployment_view: Option<ContainerDeploymentView>,
    #[schema(required = true)]
    pub capabilities: Option<PlatformCapabilitiesView>,
}

impl ContainerSummaryView {
    pub fn from_inspection(
        container: &ContainerView,
        platform_name: &str,
        inspection: &Value,
        capabilities: Option<PlatformCapabilitiesView>,
    ) -> Result<Self, serde_json::Error> {
        let volumes = inspection["mounts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|mount| mount["name"].as_str().map(str::to_owned))
            .collect();
        let networks = inspection
            .pointer("/networkSettings/networks")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .map(|(name, settings)| {
                let id = settings["networkID"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .unwrap_or(name);
                (name.clone(), id.to_owned())
            })
            .collect();
        Ok(Self {
            name: inspection["name"].as_str().unwrap_or_default().into(),
            container_id: container.container_id.clone(),
            platform_id: container.platform_id,
            // Deployment readers do not automatically have permission to read the parent Platform.
            platform_name: if capabilities.is_some() {
                platform_name.to_owned()
            } else {
                String::new()
            },
            started_at: inspection
                .pointer("/state/startedAt")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            finished_at: inspection
                .pointer("/state/finishedAt")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            volumes,
            networks,
            ports: serde_json::from_value(
                inspection
                    .pointer("/hostConfig/portBindings")
                    .filter(|v| v.is_object())
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
            )?,
            state: serde_json::from_value(Value::String(
                inspection
                    .pointer("/state/status")
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown")
                    .into(),
            ))?,
            image_view: container.image_view.clone(),
            deployment_view: container.deployment_view.clone(),
            capabilities,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn container() -> ContainerView {
        ContainerView {
            id: Uuid::now_v7(),
            platform_id: Uuid::now_v7(),
            container_id: "docker-container-id".into(),
            name: "web".into(),
            docker_image_id: "sha256:image".into(),
            created: 1,
            updated: 2,
            state: "Running".into(),
            control_state: citadel_primitives::ResourceControlState::Queued,
            stack: None,
            is_system: true,
            system_role: Some("EdgeAgent".into()),
            has_citadel_ownership_labels: false,
            is_swarm_task: false,
            docker_node_id: None,
            node_hostname: None,
            projection_observed_at: None,
            projection_stale_since: None,
            projection_stale_reason: None,
            last_stats: None,
            ports: json!({"80/tcp":[{"hostIP":"::","hostPort":"8080"}],"53/udp":null}),
            deployment_id: None,
            stack_id: None,
            image_view: None,
            deployment_view: None,
            capabilities: None,
        }
    }

    #[test]
    fn runtime_projection_keeps_identity_ports_and_explicit_nulls() {
        let container = container();
        let value = serde_json::to_value(container.runtime_data().unwrap()).unwrap();
        assert_eq!(value["id"], "docker-container-id");
        assert_eq!(value["platformId"], container.platform_id.to_string());
        assert_eq!(value["ports"]["80/tcp"][0]["hostIP"], "::");
        assert!(value["ports"]["53/udp"].is_null());
        assert_eq!(value["controlState"], "Queued");
        assert_eq!(value["systemRole"], "EdgeAgent");
        for key in [
            "containerStat",
            "capabilities",
            "deploymentId",
            "stackId",
            "dockerNodeId",
            "stack",
        ] {
            assert!(value.as_object().unwrap().contains_key(key));
            assert!(value[key].is_null());
        }
    }

    #[test]
    fn invalid_persisted_runtime_fields_fail_projection() {
        let mut container = container();
        container.state = "invalid".into();
        assert!(container.runtime_data().is_err());
        container.state = "Running".into();
        container.ports = json!({"80/tcp":42});
        assert!(container.runtime_data().is_err());
    }

    #[test]
    fn runtime_projection_tolerates_missing_and_unknown_system_roles() {
        let mut container = container();
        for role in [
            None,
            Some(""),
            Some("  "),
            Some("unknown-role"),
            Some("swarm-node-agent"),
        ] {
            container.system_role = role.map(str::to_owned);
            let value = serde_json::to_value(container.runtime_data().unwrap()).unwrap();
            assert_eq!(value["isSystem"], true);
            assert_eq!(value["systemRole"], Value::Null, "role: {role:?}");
        }
    }

    #[test]
    fn runtime_projection_maps_system_role_labels_case_insensitively() {
        let mut container = container();
        for (label, expected) in [
            ("core", ContainerSystemRole::Core),
            ("Database", ContainerSystemRole::Database),
            ("AGENT", ContainerSystemRole::Agent),
            ("EdgeAgent", ContainerSystemRole::EdgeAgent),
            ("edge-agent", ContainerSystemRole::EdgeAgent),
            (" EDGE-AGENT ", ContainerSystemRole::EdgeAgent),
        ] {
            container.system_role = Some(label.into());
            assert_eq!(
                container.runtime_data().unwrap().system_role,
                Some(expected)
            );
        }
        container.is_system = false;
        assert_eq!(container.runtime_data().unwrap().system_role, None);
    }

    #[test]
    fn summary_preserves_network_names_and_does_not_disclose_parent_platform() {
        let container = container();
        let inspection = json!({"name":"/web", "state":{"status":"Running","startedAt":"now"},
            "mounts":[{"name":"data"},{"source":"/host","destination":"/bind"}],
            "hostConfig":{"portBindings":{"53/udp":null}},
            "networkSettings":{"networks":{"Mixed.Network":{"networkID":"network-id"},"fallback":{"networkID":""}}},
            "config":{"env":["SECRET=must-not-appear"]}});
        let view = ContainerSummaryView::from_inspection(
            &container,
            "private-platform",
            &inspection,
            None,
        )
        .unwrap();
        assert!(view.platform_name.is_empty());
        assert_eq!(view.networks["Mixed.Network"], "network-id");
        assert_eq!(view.networks["fallback"], "fallback");
        assert_eq!(view.volumes, vec!["data"]);
        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.contains("must-not-appear"));
        let view = ContainerSummaryView::from_inspection(
            &container,
            "visible-platform",
            &inspection,
            Some(PlatformCapabilitiesView::default()),
        )
        .unwrap();
        assert_eq!(view.platform_name, "visible-platform");
    }

    #[test]
    fn summary_and_runtime_routes_derive_native_schemas() {
        let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
        for (path, name) in [
            ("/api/v1/containers/{id}/info", "ContainerSummaryView"),
            ("/api/v1/deployments/{id}/info", "ContainerSummaryView"),
            ("/api/v1/containers/{id}/data", "ContainerRuntimeView"),
            ("/api/v1/stacks/{stackId}/data", "ContainerRuntimeListView"),
        ] {
            assert_eq!(
                doc["paths"][path]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
                    ["$ref"],
                format!("#/components/schemas/{name}")
            );
        }
    }
}
