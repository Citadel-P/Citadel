//! Inventory projections omit container configuration and config contents during decoding.
use super::{DockerClient, DockerError};
use crate::client::mapping::version;
use crate::client::projection::{SwarmConfig, SwarmTask};
use citadel_docker_api::models::{ObjectVersion, TaskState, TaskStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Task {
    #[serde(rename = "ID")]
    id: Option<String>,
    version: Option<Box<ObjectVersion>>,
    name: Option<String>,
    #[serde(rename = "NodeID")]
    node_id: Option<String>,
    #[serde(rename = "ServiceID")]
    service_id: Option<String>,
    desired_state: Option<TaskState>,
    slot: Option<i32>,
    created_at: Option<String>,
    updated_at: Option<String>,
    status: Option<TaskStatus>,
    spec: Option<TaskSpec>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct TaskSpec {
    container_spec: Option<ContainerSpec>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct ContainerSpec {
    image: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Config {
    #[serde(rename = "ID")]
    id: Option<String>,
    version: Option<Box<ObjectVersion>>,
    created_at: Option<String>,
    updated_at: Option<String>,
    spec: Option<ConfigSpec>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct ConfigSpec {
    name: Option<String>,
    labels: Option<BTreeMap<String, String>>,
    templating: Option<Driver>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Driver {
    name: Option<String>,
}

impl DockerClient {
    pub(in crate::client) async fn task_inventory(
        &self,
        filters: Option<&str>,
    ) -> Result<Vec<SwarmTask>, DockerError> {
        let bytes = self.filtered_inventory("/tasks", filters).await?;
        serde_json::from_slice::<Vec<Task>>(&bytes)?
            .into_iter()
            .map(|t| {
                Ok(SwarmTask {
                    id: t.id.unwrap_or_default(),
                    name: t.name.unwrap_or_default(),
                    version: version(t.version),
                    created_at: t.created_at.unwrap_or_default(),
                    updated_at: t.updated_at.unwrap_or_default(),
                    slot: t.slot,
                    node_id: t.node_id.unwrap_or_default(),
                    service_id: t.service_id.unwrap_or_default(),
                    desired_state: t.desired_state.map(|s| s.to_string()).unwrap_or_default(),
                    spec: serde_json::to_value(t.spec)?,
                    status: serde_json::to_value(t.status)?,
                })
            })
            .collect()
    }
    pub(in crate::client) async fn config_inventory(
        &self,
        filters: Option<&str>,
    ) -> Result<Vec<SwarmConfig>, DockerError> {
        let bytes = self.filtered_inventory("/configs", filters).await?;
        serde_json::from_slice::<Vec<Config>>(&bytes)?
            .into_iter()
            .map(|c| {
                Ok(SwarmConfig {
                    id: c.id.unwrap_or_default(),
                    version: version(c.version),
                    created_at: c.created_at.unwrap_or_default(),
                    updated_at: c.updated_at.unwrap_or_default(),
                    spec: serde_json::to_value(c.spec)?,
                })
            })
            .collect()
    }
    async fn filtered_inventory(
        &self,
        path: &str,
        filters: Option<&str>,
    ) -> Result<Vec<u8>, DockerError> {
        if let Some(filters) = filters {
            self.bounded_json_bytes(&format!("{path}?filters={}", urlencoding::encode(filters)))
                .await
        } else {
            self.bounded_json_bytes(path).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_retains_image_and_config_metadata_with_large_unused_fields() {
        let unused = "X".repeat(128 * 1024);
        let task = serde_json::json!({"Version":{"Index":null},"Spec":{"ContainerSpec":{"Image":"app","Env":[unused]}}});
        let config = serde_json::json!({"Spec":{"Name":"one","Labels":{"team":"ops"},"Data":unused,"Templating":{"Name":"golang"}}});
        let task: Task = serde_json::from_slice(&serde_json::to_vec(&task).unwrap()).unwrap();
        let config: Config = serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert_eq!(version(task.version).index, 0);
        assert_eq!(
            serde_json::to_value(task.spec).unwrap(),
            serde_json::json!({"ContainerSpec":{"Image":"app"}})
        );
        assert_eq!(
            serde_json::to_value(config.spec).unwrap(),
            serde_json::json!({"Name":"one","Labels":{"team":"ops"},"Templating":{"Name":"golang"}})
        );
    }
}
