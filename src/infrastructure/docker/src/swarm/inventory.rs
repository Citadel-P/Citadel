use crate::client::{DockerClient, runtime::normalize_docker_error};
use base64::Engine;
use citadel_contracts::citadel::swarm::v1::*;
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind, RuntimeSwarmNode, swarm_mutations::*,
};
use tokio_util::sync::CancellationToken;
pub struct LocalSwarmInventory<'a>(pub &'a DockerClient);
fn conflict(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Conflict, message, false)
}
impl LocalSwarmInventory<'_> {
    pub async fn inspect_service(
        &self,
        id: &str,
        _cancel: &CancellationToken,
    ) -> Result<SwarmServiceMessage, RuntimeCapabilityError> {
        let docker = self.0;

        let (native, tasks) = tokio::try_join!(
            docker.inspect_swarm_service(id),
            docker.list_swarm_service_tasks(id)
        )
        .map_err(normalize_docker_error)?;
        crate::swarm::inspection::inspect_service_message(native, &tasks)
    }
    pub async fn inspect_node(
        &self,
        id: &str,
        _cancel: &CancellationToken,
    ) -> Result<(RuntimeSwarmNode, i32, i32), RuntimeCapabilityError> {
        let docker = self.0;

        let (node, tasks) = tokio::try_join!(
            docker.inspect_swarm_node(id),
            docker.list_swarm_node_tasks(id)
        )
        .map_err(normalize_docker_error)?;
        let current: Vec<_> = tasks
            .iter()
            .filter(|task| task.node_id == id && task.desired_state.eq_ignore_ascii_case("running"))
            .collect();
        let running = current
            .iter()
            .filter(|task| task.status["State"].as_str() == Some("running"))
            .count();
        Ok((
            crate::client::inventory::map_node(node),
            running as i32,
            current.len() as i32,
        ))
    }
    pub async fn update_node(
        &self,
        id: &str,
        input: &UpdateSwarmNodeInput,
        _cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let docker = self.0;

        let mut live = docker
            .inspect_swarm_node(id)
            .await
            .map_err(normalize_docker_error)?;
        if live.id != id || live.version.index != input.version_index as u64 {
            return Err(conflict("Node changed. Reload it before saving."));
        }
        live.spec["Availability"] = serde_json::json!(input.availability.to_ascii_lowercase());
        live.spec["Labels"] = serde_json::json!(input.labels);
        docker
            .update_swarm_node(id, input.version_index, &live.spec)
            .await
            .map_err(normalize_docker_error)
    }
    pub async fn restart_service(
        &self,
        id: &str,
        _cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let docker = self.0;

        let mut live = docker
            .inspect_swarm_service(id)
            .await
            .map_err(normalize_docker_error)?;
        if live.id != id {
            return Err(conflict("Docker returned a different Service."));
        }
        let force = live.spec["TaskTemplate"]["ForceUpdate"]
            .as_u64()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| conflict("Service restart counter overflow."))?;
        live.spec["TaskTemplate"]["ForceUpdate"] = serde_json::json!(force);
        docker
            .update_swarm_service(
                id,
                i64::try_from(live.version.index)
                    .map_err(|_| conflict("Invalid Service version."))?,
                &live.spec,
            )
            .await
            .map(|_| ())
            .map_err(normalize_docker_error)
    }
    pub async fn delete_service(
        &self,
        id: &str,
        _cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let docker = self.0;
        docker
            .delete_swarm_service(id)
            .await
            .map_err(normalize_docker_error)
    }
    pub async fn create_material(
        &self,
        secret: bool,
        input: &CreateSwarmMaterialInput,
        _cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let docker = self.0;

        let spec = serde_json::json!({"Name":input.name,"Labels":input.labels,"Data":base64::engine::general_purpose::STANDARD.encode(input.data.as_bytes())});
        let response = docker
            .create_swarm_material(secret, &spec)
            .await
            .map_err(normalize_docker_error)
            .map_err(material_error)?;
        if response["ID"].as_str().is_some_and(|id| !id.is_empty()) {
            Ok(())
        } else {
            Err(conflict("Docker returned no id for the created resource."))
        }
    }
    pub async fn update_labels(
        &self,
        secret: bool,
        id: &str,
        input: &UpdateSwarmResourceLabelsInput,
        _cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let docker = self.0;

        let (live_id, version, mut spec) = if secret {
            let value = docker
                .inspect_swarm_secret(id)
                .await
                .map_err(normalize_docker_error)?;
            (value.id, value.version.index, value.spec)
        } else {
            let value = docker
                .inspect_swarm_config(id)
                .await
                .map_err(normalize_docker_error)?;
            (value.id, value.version.index, value.spec)
        };
        if live_id != id || version != input.version_index as u64 {
            return Err(conflict("Resource changed. Reload it before saving."));
        }
        spec["Labels"] = serde_json::json!(input.labels);
        // Config updates must resubmit exactly the original data. Secret
        // inspection intentionally omits data, so metadata-only is correct.
        if secret && let Some(object) = spec.as_object_mut() {
            object.remove("Data");
        }
        docker
            .update_swarm_material(secret, id, input.version_index, &spec)
            .await
            .map_err(normalize_docker_error)
    }
    pub async fn delete_material(
        &self,
        secret: bool,
        id: &str,
        _cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let docker = self.0;

        if secret {
            docker.delete_swarm_secret(id).await
        } else {
            docker.delete_swarm_config(id).await
        }
        .map_err(normalize_docker_error)
    }
    pub async fn config_data(
        &self,
        id: &str,
        _cancel: &CancellationToken,
    ) -> Result<String, RuntimeCapabilityError> {
        let docker = self.0;
        let bytes = {
            let config = docker
                .inspect_swarm_config(id)
                .await
                .map_err(normalize_docker_error)?;
            if config.id != id {
                return Err(conflict("Docker returned a different Config."));
            }
            base64::engine::general_purpose::STANDARD
                .decode(config.spec["Data"].as_str().unwrap_or_default())
                .map_err(|_| conflict("Docker returned invalid Config data."))?
        };
        if bytes.len() > 1000 * 1024 {
            return Err(conflict("Config data exceeds the permitted size."));
        }
        String::from_utf8(bytes).map_err(|_| RuntimeCapabilityError::new(RuntimeErrorKind::InvalidRequest,
            "This Swarm config contains binary data and cannot be displayed in the text editor.", false))
    }
}
fn material_error(error: RuntimeCapabilityError) -> RuntimeCapabilityError {
    // A daemon or remote driver may echo submitted Secret data in its error.
    RuntimeCapabilityError::new(
        error.kind,
        "Docker rejected the resource creation request.",
        false,
    )
}
