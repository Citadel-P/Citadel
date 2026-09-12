//! Native Swarm inventory inputs. These operations do not create Citadel workloads.
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// Require the currently connected daemon to remain the pinned Swarm manager.
pub fn manager_matches(
    info: &crate::RuntimePlatformInfo,
    cluster_id: Option<&str>,
    descriptor: &serde_json::Value,
) -> bool {
    let manager = descriptor
        .get("nodeID")
        .or_else(|| descriptor.get("NodeID"))
        .and_then(serde_json::Value::as_str);
    let daemon = descriptor
        .get("daemonId")
        .or_else(|| descriptor.get("DaemonId"))
        .and_then(serde_json::Value::as_str);
    info.swarm.as_ref().is_some_and(|swarm| {
        swarm.control_available
            && swarm.local_node_state.eq_ignore_ascii_case("active")
            && cluster_id
                .is_some_and(|id| !id.is_empty() && Some(id) == swarm.cluster_id.as_deref())
            && manager.is_some_and(|id| !id.is_empty() && id == swarm.node_id)
            && daemon.is_some_and(|id| !id.is_empty() && id == info.daemon_id)
    })
}

#[derive(Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSwarmNodeInput {
    pub version_index: i64,
    pub availability: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}
#[derive(Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmNodeAvailabilityTarget {
    pub node_id: String,
    pub version_index: i64,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSwarmNodesAvailabilityInput {
    #[serde(default)]
    pub nodes: Vec<SwarmNodeAvailabilityTarget>,
    pub availability: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSwarmMaterialInput {
    pub name: String,
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSwarmResourceLabelsInput {
    pub version_index: i64,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}
#[derive(Deserialize, utoipa::ToSchema)]
pub struct DeleteSwarmResourcesInput {
    #[serde(default)]
    pub ids: Vec<String>,
}
pub fn resource_id(id: &str) -> Result<(), &'static str> {
    if id.trim().is_empty() || id.len() > 255 || id.chars().any(char::is_control) {
        Err("A resource id of at most 255 characters is required.")
    } else {
        Ok(())
    }
}
pub fn availability(value: &str) -> Result<(), &'static str> {
    if ["Active", "Pause", "Drain"]
        .iter()
        .any(|v| v.eq_ignore_ascii_case(value))
    {
        Ok(())
    } else {
        Err("Availability must be Active, Pause, or Drain.")
    }
}
pub fn labels(values: &BTreeMap<String, String>) -> Result<(), &'static str> {
    if values.len() > 1000
        || values
            .iter()
            .any(|(key, value)| key.trim().is_empty() || key.len() > 255 || value.len() > 4096)
    {
        Err(
            "Labels must have nonempty keys of at most 255 characters and values of at most 4096 characters.",
        )
    } else {
        Ok(())
    }
}
pub fn version(value: i64) -> Result<(), &'static str> {
    if value < 0 {
        Err("Version index must not be negative.")
    } else {
        Ok(())
    }
}
impl UpdateSwarmNodeInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        version(self.version_index)?;
        availability(&self.availability)?;
        labels(&self.labels)
    }
}
impl UpdateSwarmNodesAvailabilityInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        availability(&self.availability)?;
        if self.nodes.is_empty() || self.nodes.len() > 100 {
            return Err("Select between 1 and 100 nodes.");
        }
        let mut ids = BTreeSet::new();
        for node in &self.nodes {
            resource_id(&node.node_id)?;
            version(node.version_index)?;
            if !ids.insert(&node.node_id) {
                return Err("Each node can only be selected once.");
            }
        }
        Ok(())
    }
}
impl CreateSwarmMaterialInput {
    pub fn validate(&self, secret: bool) -> Result<(), &'static str> {
        resource_id(&self.name)?;
        labels(&self.labels)?;
        if secret && self.data.is_empty() {
            return Err("Secret data is required.");
        }
        if self.data.len() > if secret { 500 * 1024 } else { 1000 * 1024 } {
            return Err("Resource data exceeds the permitted size.");
        }
        Ok(())
    }
}
impl UpdateSwarmResourceLabelsInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        version(self.version_index)?;
        labels(&self.labels)
    }
}
impl DeleteSwarmResourcesInput {
    pub fn validate(&mut self) -> Result<(), &'static str> {
        if self.ids.is_empty() || self.ids.len() > 100 {
            return Err("Select between 1 and 100 resources.");
        }
        for id in &self.ids {
            resource_id(id)?;
        }
        let mut seen = BTreeSet::new();
        self.ids.retain(|id| seen.insert(id.clone()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_utf8_payload_bytes_and_immutable_update_contract() {
        let input = CreateSwarmMaterialInput {
            name: "token".into(),
            data: "é".repeat(256001),
            labels: BTreeMap::new(),
        };
        assert!(input.validate(true).is_err());
        assert!(input.validate(false).is_ok());
        let labels: UpdateSwarmResourceLabelsInput =
            serde_json::from_str(r#"{"versionIndex":2,"labels":{},"data":"ignored"}"#).unwrap();
        assert!(labels.validate().is_ok());
    }
    #[test]
    fn validates_entire_selection_and_versions() {
        let input: UpdateSwarmNodesAvailabilityInput = serde_json::from_str(r#"{"availability":"Drain","nodes":[{"nodeId":"n","versionIndex":1},{"nodeId":"n","versionIndex":2}]}"#).unwrap();
        assert!(input.validate().is_err());
        assert!(version(-1).is_err());
        assert!(availability("pause").is_ok());
        assert!(availability("stop").is_err());
        let mut input = DeleteSwarmResourcesInput {
            ids: vec!["s".into(), "s".into()],
        };
        input.validate().unwrap();
        assert_eq!(input.ids, ["s"]);
    }
}
