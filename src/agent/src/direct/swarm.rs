use super::{Runtime, docker_error, runtime_error, swarm_mapping as map, text};
use base64::Engine;
use citadel_adapters::connectors::swarm::{
    inspection::inspect_service_message, inventory::SwarmInventoryClient,
};
use citadel_contracts::citadel::swarm::v1::{swarm_service_server::SwarmService, *};
use citadel_platforms::{
    logs::{LogReadPort, LogResource},
    swarm_mutations::{UpdateSwarmNodeInput, UpdateSwarmResourceLabelsInput},
};
use futures_util::StreamExt;
use serde_json::json;
use tonic::{Request, Response, Status};
fn limit(n: i32) -> usize {
    if n == i32::MAX {
        i32::MAX as usize
    } else if n <= 0 {
        500
    } else {
        n.min(500) as usize
    }
}
fn version(n: u64) -> Result<i64, Status> {
    i64::try_from(n).map_err(|_| Status::invalid_argument("Version index exceeds Docker range"))
}
#[tonic::async_trait]
impl SwarmService for Runtime {
    async fn list_nodes(
        &self,
        r: Request<ListSwarmNodesRequest>,
    ) -> Result<Response<ListSwarmNodesResponse>, Status> {
        let r = r.into_inner();
        let mut nodes = self.docker.list_swarm_nodes().await.map_err(docker_error)?;
        nodes.sort_by_key(|v| map::node_name(v).to_lowercase());
        let tasks = if r.include_task_counts {
            self.docker.list_swarm_tasks().await.map_err(docker_error)?
        } else {
            vec![]
        };
        Ok(Response::new(ListSwarmNodesResponse {
            nodes: nodes
                .into_iter()
                .take(limit(r.max_items))
                .map(|n| map::node(n, &tasks))
                .collect(),
        }))
    }
    async fn inspect_node(
        &self,
        r: Request<InspectSwarmNodeRequest>,
    ) -> Result<Response<SwarmNodeMessage>, Status> {
        let id = r.into_inner().node_id;
        let (node, tasks) = tokio::try_join!(
            self.docker.inspect_swarm_node(&id),
            self.docker.list_swarm_node_tasks(&id)
        )
        .map_err(docker_error)?;
        Ok(Response::new(map::node(node, &tasks)))
    }
    async fn update_node(
        &self,
        r: Request<UpdateSwarmNodeRequest>,
    ) -> Result<Response<()>, Status> {
        let r = r.into_inner();
        let availability = r.availability.to_ascii_lowercase();
        if !matches!(availability.as_str(), "active" | "pause" | "drain") {
            return Err(Status::invalid_argument("Invalid node availability"));
        }
        SwarmInventoryClient::Local(&self.docker)
            .update_node(
                &r.node_id,
                &UpdateSwarmNodeInput {
                    version_index: version(r.version_index)?,
                    availability,
                    labels: r.labels.into_iter().collect(),
                },
                &self.shutdown,
            )
            .await
            .map_err(runtime_error)?;
        Ok(Response::new(()))
    }
    async fn list_services(
        &self,
        r: Request<ListSwarmServicesRequest>,
    ) -> Result<Response<ListSwarmServicesResponse>, Status> {
        let mut values = self
            .docker
            .list_swarm_services()
            .await
            .map_err(docker_error)?;
        values.sort_by_key(|v| text(&v.spec, "Name").to_lowercase());
        let services = values
            .into_iter()
            .take(limit(r.into_inner().max_items))
            .map(|v| {
                let running = v.service_status["RunningTasks"]
                    .as_i64()
                    .unwrap_or_default();
                let desired = v.service_status["DesiredTasks"]
                    .as_i64()
                    .unwrap_or_default();
                let mut result = inspect_service_message(v, &[]).map_err(runtime_error)?;
                result.running_task_count = running as i32;
                result.desired_task_count = desired as i32;
                result.definition = None;
                Ok(result)
            })
            .collect::<Result<_, Status>>()?;
        Ok(Response::new(ListSwarmServicesResponse { services }))
    }
    async fn inspect_service(
        &self,
        r: Request<InspectSwarmServiceRequest>,
    ) -> Result<Response<SwarmServiceMessage>, Status> {
        Ok(Response::new(
            SwarmInventoryClient::Local(&self.docker)
                .inspect_service(&r.into_inner().service_id, &self.shutdown)
                .await
                .map_err(runtime_error)?,
        ))
    }
    async fn restart_service(
        &self,
        r: Request<RestartSwarmServiceRequest>,
    ) -> Result<Response<()>, Status> {
        SwarmInventoryClient::Local(&self.docker)
            .restart_service(&r.into_inner().service_id, &self.shutdown)
            .await
            .map_err(runtime_error)?;
        Ok(Response::new(()))
    }
    async fn delete_service(
        &self,
        r: Request<DeleteManagedSwarmServiceRequest>,
    ) -> Result<Response<()>, Status> {
        self.docker
            .delete_swarm_service(&r.into_inner().service_id)
            .await
            .map_err(docker_error)?;
        Ok(Response::new(()))
    }
    async fn get_service_logs(
        &self,
        r: Request<SwarmLogsRequest>,
    ) -> Result<Response<SwarmLogsResponse>, Status> {
        let r = r.into_inner();
        let v = self
            .docker
            .read_logs(
                LogResource::Service(&r.resource_id),
                r.tail.clamp(1, 200) as u16,
                &self.shutdown,
            )
            .await
            .map_err(runtime_error)?;
        Ok(Response::new(SwarmLogsResponse {
            lines: v.lines,
            truncated: v.truncated,
        }))
    }
    async fn list_tasks(
        &self,
        r: Request<ListSwarmTasksRequest>,
    ) -> Result<Response<ListSwarmTasksResponse>, Status> {
        let mut tasks = self.docker.list_swarm_tasks().await.map_err(docker_error)?;
        tasks.sort_by_key(|v| {
            std::cmp::Reverse(
                map::time(&text(&v.status, "Timestamp")).map(|v| (v.seconds, v.nanos)),
            )
        });
        Ok(Response::new(ListSwarmTasksResponse {
            tasks: tasks
                .into_iter()
                .take(limit(r.into_inner().max_items))
                .map(map::task)
                .collect(),
        }))
    }
    async fn inspect_task(
        &self,
        r: Request<InspectSwarmTaskRequest>,
    ) -> Result<Response<SwarmTaskMessage>, Status> {
        Ok(Response::new(map::task(
            self.docker
                .inspect_swarm_task(&r.into_inner().task_id)
                .await
                .map_err(docker_error)?,
        )))
    }
    async fn get_task_logs(
        &self,
        r: Request<SwarmLogsRequest>,
    ) -> Result<Response<SwarmLogsResponse>, Status> {
        let r = r.into_inner();
        let mut stream = self
            .docker
            .task_logs(&r.resource_id, r.tail.clamp(1, 200) as u16, &self.shutdown)
            .await
            .map_err(docker_error)?;
        let mut data = Vec::new();
        let mut truncated = false;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(runtime_error)?;
            let remaining = (1024 * 1024usize).saturating_sub(data.len());
            data.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
            if chunk.len() > remaining {
                truncated = true;
                break;
            }
        }
        Ok(Response::new(SwarmLogsResponse {
            lines: String::from_utf8_lossy(&data)
                .split_inclusive('\n')
                .map(str::to_owned)
                .collect(),
            truncated,
        }))
    }
    async fn list_networks(
        &self,
        r: Request<ListSwarmNetworksRequest>,
    ) -> Result<Response<ListSwarmNetworksResponse>, Status> {
        let mut values = self
            .docker
            .list_networks_filtered(Some(r#"{"scope":["swarm"]}"#))
            .await
            .map_err(docker_error)?;
        values.sort_by_key(|v| v.name.to_lowercase());
        Ok(Response::new(ListSwarmNetworksResponse {
            networks: values
                .into_iter()
                .take(limit(r.into_inner().max_items))
                .map(map::network)
                .collect(),
        }))
    }
    async fn inspect_network(
        &self,
        r: Request<InspectSwarmNetworkRequest>,
    ) -> Result<Response<SwarmNetworkMessage>, Status> {
        Ok(Response::new(map::network(
            self.docker
                .inspect_network(&r.into_inner().network_id)
                .await
                .map_err(docker_error)?,
        )))
    }
    async fn list_secrets(
        &self,
        r: Request<ListSwarmSecretsRequest>,
    ) -> Result<Response<ListSwarmSecretsResponse>, Status> {
        let mut values = self
            .docker
            .list_swarm_secrets()
            .await
            .map_err(docker_error)?;
        values.sort_by_key(|v| text(&v.spec, "Name").to_lowercase());
        Ok(Response::new(ListSwarmSecretsResponse {
            secrets: values
                .into_iter()
                .take(limit(r.into_inner().max_items))
                .map(map::secret)
                .collect(),
        }))
    }
    async fn inspect_secret(
        &self,
        r: Request<InspectSwarmSecretRequest>,
    ) -> Result<Response<SwarmSecretMessage>, Status> {
        Ok(Response::new(map::secret(
            self.docker
                .inspect_swarm_secret(&r.into_inner().secret_id)
                .await
                .map_err(docker_error)?,
        )))
    }
    async fn list_configs(
        &self,
        r: Request<ListSwarmConfigsRequest>,
    ) -> Result<Response<ListSwarmConfigsResponse>, Status> {
        let mut values = self
            .docker
            .list_swarm_configs()
            .await
            .map_err(docker_error)?;
        values.sort_by_key(|v| text(&v.spec, "Name").to_lowercase());
        Ok(Response::new(ListSwarmConfigsResponse {
            configs: values
                .into_iter()
                .take(limit(r.into_inner().max_items))
                .map(map::config)
                .collect(),
        }))
    }
    async fn inspect_config(
        &self,
        r: Request<InspectSwarmConfigRequest>,
    ) -> Result<Response<SwarmConfigMessage>, Status> {
        Ok(Response::new(map::config(
            self.docker
                .inspect_swarm_config(&r.into_inner().config_id)
                .await
                .map_err(docker_error)?,
        )))
    }
    async fn get_config_data(
        &self,
        r: Request<InspectSwarmConfigRequest>,
    ) -> Result<Response<SwarmConfigDataResponse>, Status> {
        let v = self
            .docker
            .inspect_swarm_config(&r.into_inner().config_id)
            .await
            .map_err(docker_error)?;
        let data = base64::engine::general_purpose::STANDARD
            .decode(v.spec["Data"].as_str().unwrap_or_default())
            .map_err(|_| Status::data_loss("Invalid config data"))?;
        Ok(Response::new(SwarmConfigDataResponse { data }))
    }
    async fn create_secret(
        &self,
        r: Request<CreateSwarmSecretRequest>,
    ) -> Result<Response<SwarmResourceCreateResponse>, Status> {
        let r = r.into_inner();
        self.create_material(true, r.name, r.data, r.labels).await
    }
    async fn create_config(
        &self,
        r: Request<CreateSwarmConfigRequest>,
    ) -> Result<Response<SwarmResourceCreateResponse>, Status> {
        let r = r.into_inner();
        self.create_material(false, r.name, r.data, r.labels).await
    }
    async fn update_secret_labels(
        &self,
        r: Request<UpdateSwarmResourceLabelsRequest>,
    ) -> Result<Response<()>, Status> {
        self.update_material(true, r.into_inner()).await
    }
    async fn update_config_labels(
        &self,
        r: Request<UpdateSwarmResourceLabelsRequest>,
    ) -> Result<Response<()>, Status> {
        self.update_material(false, r.into_inner()).await
    }
    async fn delete_secret(
        &self,
        r: Request<DeleteSwarmSecretRequest>,
    ) -> Result<Response<()>, Status> {
        self.docker
            .delete_swarm_secret(&r.into_inner().secret_id)
            .await
            .map_err(docker_error)?;
        Ok(Response::new(()))
    }
    async fn delete_config(
        &self,
        r: Request<DeleteSwarmConfigRequest>,
    ) -> Result<Response<()>, Status> {
        self.docker
            .delete_swarm_config(&r.into_inner().config_id)
            .await
            .map_err(docker_error)?;
        Ok(Response::new(()))
    }
    async fn create_service(
        &self,
        r: Request<CreateManagedSwarmServiceRequest>,
    ) -> Result<Response<SwarmServiceMutationResponse>, Status> {
        let r = r.into_inner();
        let spec = super::swarm_spec::managed(
            r.spec
                .ok_or_else(|| Status::invalid_argument("Service spec required"))?,
            r.docker_name,
            r.labels,
            None,
        )?;
        let result = self
            .docker
            .create_swarm_service_authenticated(&spec, Some(&r.registry_auth))
            .await
            .map_err(docker_error)?;
        mutation(result, None)
    }
    async fn update_service(
        &self,
        r: Request<UpdateManagedSwarmServiceRequest>,
    ) -> Result<Response<SwarmServiceMutationResponse>, Status> {
        let r = r.into_inner();
        let version = version(r.version_index)?;
        let live = self
            .docker
            .inspect_swarm_service(&r.service_id)
            .await
            .map_err(docker_error)?;
        if live.version.index != r.version_index {
            return Err(Status::failed_precondition("Service version changed"));
        }
        let spec = super::swarm_spec::managed(
            r.spec
                .ok_or_else(|| Status::invalid_argument("Service spec required"))?,
            text(&live.spec, "Name"),
            r.labels,
            Some(live.spec),
        )?;
        let result = self
            .docker
            .update_swarm_service_authenticated(
                &r.service_id,
                version,
                &spec,
                Some(&r.registry_auth),
            )
            .await
            .map_err(docker_error)?;
        mutation(result, Some(r.service_id))
    }
    async fn create_system_service(
        &self,
        r: Request<CreateSystemSwarmServiceRequest>,
    ) -> Result<Response<SwarmServiceMutationResponse>, Status> {
        let r = r.into_inner();
        let spec = super::swarm_spec::system(
            r.spec
                .ok_or_else(|| Status::invalid_argument("System spec required"))?,
            r.docker_name,
            r.labels,
            r.container_labels,
        )?;
        mutation(
            self.docker
                .create_swarm_service(&spec)
                .await
                .map_err(docker_error)?,
            None,
        )
    }
    async fn update_system_service(
        &self,
        r: Request<UpdateSystemSwarmServiceRequest>,
    ) -> Result<Response<SwarmServiceMutationResponse>, Status> {
        let r = r.into_inner();
        let version = version(r.version_index)?;
        let live = self
            .docker
            .inspect_swarm_service(&r.service_id)
            .await
            .map_err(docker_error)?;
        if live.version.index != r.version_index {
            return Err(Status::failed_precondition("Service version changed"));
        }
        let spec = super::swarm_spec::system(
            r.spec
                .ok_or_else(|| Status::invalid_argument("System spec required"))?,
            text(&live.spec, "Name"),
            r.labels,
            r.container_labels,
        )?;
        mutation(
            self.docker
                .update_swarm_service(&r.service_id, version, &spec)
                .await
                .map_err(docker_error)?,
            Some(r.service_id),
        )
    }
}
impl Runtime {
    async fn create_material(
        &self,
        secret: bool,
        name: String,
        data: Vec<u8>,
        labels: std::collections::HashMap<String, String>,
    ) -> Result<Response<SwarmResourceCreateResponse>, Status> {
        if name.trim().is_empty() {
            return Err(Status::invalid_argument("Resource name required"));
        }
        let spec = json!({"Name":name,"Labels":labels,"Data":base64::engine::general_purpose::STANDARD.encode(data)});
        let result = self
            .docker
            .create_swarm_material(secret, &spec)
            .await
            .map_err(docker_error)?;
        let id = text(&result, "ID");
        if id.is_empty() {
            return Err(Status::data_loss("Docker returned no resource id"));
        }
        Ok(Response::new(SwarmResourceCreateResponse {
            resource_id: id,
        }))
    }
    async fn update_material(
        &self,
        secret: bool,
        r: UpdateSwarmResourceLabelsRequest,
    ) -> Result<Response<()>, Status> {
        SwarmInventoryClient::Local(&self.docker)
            .update_labels(
                secret,
                &r.resource_id,
                &UpdateSwarmResourceLabelsInput {
                    version_index: version(r.version_index)?,
                    labels: r.labels.into_iter().collect(),
                },
                &self.shutdown,
            )
            .await
            .map_err(runtime_error)?;
        Ok(Response::new(()))
    }
}
fn mutation(
    v: serde_json::Value,
    id: Option<String>,
) -> Result<Response<SwarmServiceMutationResponse>, Status> {
    let id = id.unwrap_or_else(|| text(&v, "ID"));
    if id.is_empty() {
        return Err(Status::data_loss("Docker returned no service id"));
    }
    Ok(Response::new(SwarmServiceMutationResponse {
        service_id: id,
        warnings: v["Warnings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
    }))
}
