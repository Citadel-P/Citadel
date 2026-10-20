//! Native Swarm operations across the three existing transports.
use crate::connectors::agent::client::AgentClient;
use crate::connectors::edge::EdgeRuntime;
use citadel_contracts::citadel::{edge::v1::EdgeCommandKind, swarm::v1::*};
use citadel_docker::client::DockerClient;
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
                crate::connectors::agent::execution::unary(
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
            return citadel_docker::swarm::inventory::LocalSwarmInventory(docker)
                .inspect_service(id, cancel)
                .await;
        }
        match self {
            Self::Agent(agent) => agent.inspect_managed_swarm_service(id, cancel).await,
            Self::Edge(edge) => {
                crate::connectors::agent::execution::unary(
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
            return citadel_docker::swarm::inventory::LocalSwarmInventory(docker)
                .inspect_node(id, cancel)
                .await;
        }
        let node: SwarmNodeMessage = remote!(
            self,
            swarm_inspect_node,
            SwarmNodeInspect,
            InspectSwarmNodeRequest { node_id: id.into() },
            cancel
        )?;
        let counts = (node.running_task_count, node.desired_task_count);
        Ok((
            crate::connectors::agent::client::map_swarm_node(node),
            counts.0,
            counts.1,
        ))
    }
    pub async fn update_node(
        &self,
        id: &str,
        input: &UpdateSwarmNodeInput,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        if let Self::Local(docker) = self {
            return citadel_docker::swarm::inventory::LocalSwarmInventory(docker)
                .update_node(id, input, cancel)
                .await;
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
            return citadel_docker::swarm::inventory::LocalSwarmInventory(docker)
                .restart_service(id, cancel)
                .await;
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
            Self::Local(docker) => {
                citadel_docker::swarm::inventory::LocalSwarmInventory(docker)
                    .delete_service(id, cancel)
                    .await
            }
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
                crate::connectors::agent::execution::unary(
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
            return citadel_docker::swarm::inventory::LocalSwarmInventory(docker)
                .create_material(secret, input, cancel)
                .await;
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
            return citadel_docker::swarm::inventory::LocalSwarmInventory(docker)
                .update_labels(secret, id, input, cancel)
                .await;
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
            return citadel_docker::swarm::inventory::LocalSwarmInventory(docker)
                .delete_material(secret, id, cancel)
                .await;
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
            return citadel_docker::swarm::inventory::LocalSwarmInventory(docker)
                .config_data(id, cancel)
                .await;
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
