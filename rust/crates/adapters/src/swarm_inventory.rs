//! Native Swarm operations across the three existing transports.
use crate::{
    agent::AgentClient,
    docker::{DockerClient, runtime::normalize_docker_error},
    edge::EdgeRuntime,
};
use base64::Engine;
use citadel_contracts::citadel::{edge::v1::EdgeCommandKind, swarm::v1::*};
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind, RuntimeSwarmNode, swarm_mutations::*,
};
use tokio_util::sync::CancellationToken;

pub enum SwarmInventoryClient<'a> {
    Local(&'a DockerClient),
    Agent(&'a AgentClient),
    Edge(&'a EdgeRuntime),
}

macro_rules! remote {
    ($client:expr, $direct:ident, $kind:ident, $request:expr, $cancel:expr) => {
        match $client {
            SwarmInventoryClient::Agent(agent) => agent.$direct($request, $cancel).await,
            SwarmInventoryClient::Edge(edge) => {
                crate::agent_execution::unary(
                    &edge.session,
                    EdgeCommandKind::$kind,
                    $request,
                    $cancel,
                )
                .await
            }
            SwarmInventoryClient::Local(_) => unreachable!("local transport handled above"),
        }
    };
}
fn conflict(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Conflict, message, false)
}
impl SwarmInventoryClient<'_> {
    pub async fn inspect_service(
        &self,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<SwarmServiceMessage, RuntimeCapabilityError> {
        if let Self::Local(docker) = self {
            let (native, tasks) = tokio::try_join!(
                docker.inspect_swarm_service(id),
                docker.list_swarm_service_tasks(id)
            )
            .map_err(normalize_docker_error)?;
            return crate::swarm_service_inspection::inspect_service_message(native, &tasks);
        }
        match self {
            Self::Agent(agent) => agent.inspect_managed_swarm_service(id, cancel).await,
            Self::Edge(edge) => {
                crate::agent_execution::unary(
                    &edge.session,
                    EdgeCommandKind::SwarmServiceInspect,
                    InspectSwarmServiceRequest {
                        service_id: id.into(),
                    },
                    cancel,
                )
                .await
            }
            Self::Local(_) => unreachable!(),
        }
    }
    pub async fn inspect_node(
        &self,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<(RuntimeSwarmNode, i32, i32), RuntimeCapabilityError> {
        if let Self::Local(docker) = self {
            let (node, tasks) = tokio::try_join!(
                docker.inspect_swarm_node(id),
                docker.list_swarm_node_tasks(id)
            )
            .map_err(normalize_docker_error)?;
            let current: Vec<_> = tasks
                .iter()
                .filter(|task| {
                    task.node_id == id && task.desired_state.eq_ignore_ascii_case("running")
                })
                .collect();
            let running = current
                .iter()
                .filter(|task| task.status["State"].as_str() == Some("running"))
                .count();
            return Ok((
                crate::docker::inventory::map_node(node),
                running as i32,
                current.len() as i32,
            ));
        }
        let node: SwarmNodeMessage = remote!(
            self,
            swarm_inspect_node,
            SwarmNodeInspect,
            InspectSwarmNodeRequest { node_id: id.into() },
            cancel
        )?;
        let counts = (node.running_task_count, node.desired_task_count);
        Ok((crate::agent::map_swarm_node(node), counts.0, counts.1))
    }
    pub async fn update_node(
        &self,
        id: &str,
        input: &UpdateSwarmNodeInput,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        if let Self::Local(docker) = self {
            let mut live = docker
                .inspect_swarm_node(id)
                .await
                .map_err(normalize_docker_error)?;
            if live.id != id || live.version.index != input.version_index as u64 {
                return Err(conflict("Node changed. Reload it before saving."));
            }
            live.spec["Availability"] = serde_json::json!(input.availability.to_ascii_lowercase());
            live.spec["Labels"] = serde_json::json!(input.labels);
            return docker
                .update_swarm_node(id, input.version_index, &live.spec)
                .await
                .map_err(normalize_docker_error);
        }
        remote!(
            self,
            swarm_update_node,
            SwarmNodeUpdate,
            UpdateSwarmNodeRequest {
                node_id: id.into(),
                version_index: input.version_index as u64,
                availability: input.availability.clone(),
                labels: input.labels.clone().into_iter().collect()
            },
            cancel
        )
    }
    pub async fn restart_service(
        &self,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        if let Self::Local(docker) = self {
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
            return docker
                .update_swarm_service(
                    id,
                    i64::try_from(live.version.index)
                        .map_err(|_| conflict("Invalid Service version."))?,
                    &live.spec,
                )
                .await
                .map(|_| ())
                .map_err(normalize_docker_error);
        }
        remote!(
            self,
            swarm_restart_service,
            SwarmServiceRestart,
            RestartSwarmServiceRequest {
                service_id: id.into()
            },
            cancel
        )
    }
    pub async fn delete_service(
        &self,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        match self {
            Self::Local(docker) => docker
                .delete_swarm_service(id)
                .await
                .map_err(normalize_docker_error),
            Self::Agent(agent) => {
                agent
                    .delete_managed_swarm_service(
                        DeleteManagedSwarmServiceRequest {
                            service_id: id.into(),
                            ..Default::default()
                        },
                        cancel,
                    )
                    .await
            }
            Self::Edge(edge) => {
                crate::agent_execution::unary(
                    &edge.session,
                    EdgeCommandKind::SwarmServiceDelete,
                    DeleteManagedSwarmServiceRequest {
                        service_id: id.into(),
                        ..Default::default()
                    },
                    cancel,
                )
                .await
            }
        }
    }
    pub async fn create_material(
        &self,
        secret: bool,
        input: &CreateSwarmMaterialInput,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        if let Self::Local(docker) = self {
            let spec = serde_json::json!({"Name":input.name,"Labels":input.labels,"Data":base64::engine::general_purpose::STANDARD.encode(input.data.as_bytes())});
            let response = docker
                .create_swarm_material(secret, &spec)
                .await
                .map_err(normalize_docker_error)
                .map_err(material_error)?;
            return if response["ID"].as_str().is_some_and(|id| !id.is_empty()) {
                Ok(())
            } else {
                Err(conflict("Docker returned no id for the created resource."))
            };
        }
        let response: SwarmResourceCreateResponse = if secret {
            remote!(
                self,
                swarm_create_secret,
                SwarmSecretCreate,
                CreateSwarmSecretRequest {
                    name: input.name.clone(),
                    data: input.data.as_bytes().to_vec(),
                    labels: input.labels.clone().into_iter().collect()
                },
                cancel
            )
            .map_err(material_error)?
        } else {
            remote!(
                self,
                swarm_create_config,
                SwarmConfigCreate,
                CreateSwarmConfigRequest {
                    name: input.name.clone(),
                    data: input.data.as_bytes().to_vec(),
                    labels: input.labels.clone().into_iter().collect()
                },
                cancel
            )
            .map_err(material_error)?
        };
        if response.resource_id.is_empty() {
            Err(conflict("Agent returned no id for the created resource."))
        } else {
            Ok(())
        }
    }
    pub async fn update_labels(
        &self,
        secret: bool,
        id: &str,
        input: &UpdateSwarmResourceLabelsInput,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        if let Self::Local(docker) = self {
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
            return docker
                .update_swarm_material(secret, id, input.version_index, &spec)
                .await
                .map_err(normalize_docker_error);
        }
        let request = UpdateSwarmResourceLabelsRequest {
            resource_id: id.into(),
            version_index: input.version_index as u64,
            labels: input.labels.clone().into_iter().collect(),
        };
        if secret {
            remote!(
                self,
                swarm_update_secret,
                SwarmSecretUpdate,
                request,
                cancel
            )
        } else {
            remote!(
                self,
                swarm_update_config,
                SwarmConfigUpdate,
                request,
                cancel
            )
        }
    }
    pub async fn delete_material(
        &self,
        secret: bool,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        if let Self::Local(docker) = self {
            return if secret {
                docker.delete_swarm_secret(id).await
            } else {
                docker.delete_swarm_config(id).await
            }
            .map_err(normalize_docker_error);
        }
        if secret {
            remote!(
                self,
                swarm_delete_secret,
                SwarmSecretDelete,
                DeleteSwarmSecretRequest {
                    secret_id: id.into()
                },
                cancel
            )
        } else {
            remote!(
                self,
                swarm_delete_config,
                SwarmConfigDelete,
                DeleteSwarmConfigRequest {
                    config_id: id.into()
                },
                cancel
            )
        }
    }
    pub async fn config_data(
        &self,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<String, RuntimeCapabilityError> {
        let bytes = if let Self::Local(docker) = self {
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
        } else {
            let response: SwarmConfigDataResponse = remote!(
                self,
                swarm_config_data,
                SwarmConfigData,
                InspectSwarmConfigRequest {
                    config_id: id.into()
                },
                cancel
            )?;
            response.data
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
