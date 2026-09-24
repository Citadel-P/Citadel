use super::{Runtime, docker_error, runtime_error};
use citadel_adapters::connectors::docker::LocalDockerSampler;
use citadel_contracts::citadel::{platforms::v1::*, shared_models::v1::*};
use citadel_platforms::{PlatformRuntimePort, RuntimePlatformStats, prune::PlatformPrunePort};
use futures_util::{StreamExt, stream::BoxStream};
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl platform_service_server::PlatformService for Runtime {
    async fn get_platform_info(
        &self,
        _: Request<()>,
    ) -> Result<Response<PlatformInfoResponse>, Status> {
        let info = self
            .docker
            .get_info(&self.shutdown)
            .await
            .map_err(runtime_error)?;
        let details = self.docker.info().await.map_err(docker_error)?;
        let mut sampler = LocalDockerSampler::new(self.docker.clone(), 8);
        let stats = aggregate(
            sampler
                .sample(&self.shutdown)
                .await
                .map_err(runtime_error)?,
            info.cpu_count,
        )?;
        let agent_runtime_image = if let Some(id) = &self.runtime_container {
            self.docker
                .inspect_container_document(id)
                .await
                .ok()
                .and_then(|v| {
                    v["Config"]["Image"]
                        .as_str()
                        .or_else(|| v["Image"].as_str())
                        .map(str::to_owned)
                })
                .unwrap_or_default()
        } else {
            String::new()
        };
        let manager = info.swarm.as_ref().is_some_and(|s| s.control_available);
        let (service_count, running_task_count) = if manager {
            (
                Some(
                    self.docker
                        .list_swarm_services()
                        .await
                        .map_err(docker_error)?
                        .len() as i64,
                ),
                Some(
                    self.docker
                        .list_swarm_tasks()
                        .await
                        .map_err(docker_error)?
                        .len() as i64,
                ),
            )
        } else {
            (None, None)
        };
        Ok(Response::new(PlatformInfoResponse {
            id: info.daemon_id,
            created: chrono::Utc::now().timestamp(),
            network_count: stats.network_count,
            volume_count: stats.volume_count,
            agent_version: super::version(),
            image_count: stats.image_count,
            driver: details.driver,
            operating_system: info.operating_system,
            os_version: details.os_version,
            os_type: info.os_type,
            architecture: info.architecture,
            cpu_count: info.cpu_count,
            mem_total: info.memory_total,
            server_version: info.server_version,
            api_version: info.api_version,
            minimum_api_version: info.minimum_api_version,
            swarm_info: info.swarm.map(|s| SwarmInfoMessage {
                node_id: s.node_id,
                node_addr: s.node_addr,
                local_node_state: s.local_node_state,
                control_available: s.control_available,
                error: s.error.unwrap_or_default(),
                remote_managers: s
                    .remote_managers
                    .into_iter()
                    .map(|p| SwarmPeerMessage {
                        node_id: p.node_id,
                        addr: p.address,
                    })
                    .collect(),
                nodes: s.nodes,
                managers: s.managers,
                cluster_id: s.cluster_id.unwrap_or_default(),
                cluster_created_at: s.cluster_created_at.map(|t| prost_types::Timestamp {
                    seconds: t.timestamp(),
                    nanos: t.timestamp_subsec_nanos() as i32,
                }),
                service_count,
                running_task_count,
            }),
            platform_stat: Some(stat(&stats)),
            image_used_bytes: stats.image_used_bytes,
            volume_used_bytes: stats.volume_used_bytes,
            agent_runtime_image,
        }))
    }

    async fn check_health(&self, _: Request<()>) -> Result<Response<CheckHealthResponse>, Status> {
        self.docker.ping().await.map_err(docker_error)?;
        self.docker
            .negotiated_version()
            .await
            .map_err(docker_error)?;
        Ok(Response::new(CheckHealthResponse { healthy: true }))
    }

    async fn prune(
        &self,
        request: Request<PruneRequest>,
    ) -> Result<Response<PruneResponse>, Status> {
        use citadel_platforms::prune::PruneResource as Resource;
        let resource = match request.into_inner().resource {
            1 => Resource::All,
            2 => Resource::Volume,
            3 => Resource::Network,
            4 => Resource::Image,
            5 => Resource::Build,
            _ => return Err(Status::invalid_argument("Unknown prune resource")),
        };
        let result = self
            .docker
            .prune(resource, &self.shutdown)
            .await
            .map_err(runtime_error)?;
        Ok(Response::new(PruneResponse {
            resource: resource.code(),
            space_reclaimed: result.space_reclaimed,
            volumes_deleted: result.volumes_deleted,
            networks_deleted: result.networks_deleted,
            images_deleted: result.images_deleted,
            build_cache_deleted: result.build_cache_deleted,
        }))
    }

    type StreamPlatformStatsStream = BoxStream<'static, Result<PlatformStatsResponse, Status>>;
    async fn stream_platform_stats(
        &self,
        request: Request<PlatformStatsRequest>,
    ) -> Result<Response<Self::StreamPlatformStatsStream>, Status> {
        let interval = super::interval(request.into_inner().fetch_interval_ms);
        let docker = self.docker.clone();
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let _guard = guard;
            let info = docker.info().await.map_err(docker_error)?;
            let mut sampler = LocalDockerSampler::new(docker, 8);
            loop {
                let sample = tokio::select! { ()=cancel.cancelled()=>break, value=sampler.sample(&cancel)=>value.map_err(runtime_error) }?;
                let s = aggregate(sample, info.cpu_count as i64)?;
                yield PlatformStatsResponse {
                    network_count: s.network_count, volume_count: s.volume_count, container_count: s.container_count,
                    containers_running: s.containers_running, containers_paused: s.containers_paused, containers_stopped: s.containers_stopped,
                    image_count: s.image_count, mem_total: s.mem_total, cpu_count: info.cpu_count.into(), agent_version: super::version(),
                    stat: Some(stat(&s)), image_used_bytes: s.image_used_bytes, volume_used_bytes: s.volume_used_bytes,
                };
                tokio::select! { ()=cancel.cancelled()=>break, ()=tokio::time::sleep(interval)=>{} }
            }
        })))
    }

    type StreamDaemonEventStream = BoxStream<'static, Result<DaemonEventResponse, Status>>;
    async fn stream_daemon_event(
        &self,
        _: Request<()>,
    ) -> Result<Response<Self::StreamDaemonEventStream>, Status> {
        let docker = self.docker.clone();
        let mut events = docker.events(None, None).await.map_err(docker_error)?;
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let _guard = guard;
            loop {
                let event = tokio::select! { ()=cancel.cancelled()=>break, event=events.next()=>event };
                let Some(event) = event else { break; };
                let e = event.map_err(docker_error)?;
                use daemon_event_response::Kind;
                let scope = match e.scope.as_str() { "local" => 1, "swarm" => 2, _ => 0 };
                let id = e.actor.id;
                let kind = match e.resource_type.as_str() {
                    "container" => {
                        if e.action.starts_with("exec_") || matches!(e.action.as_str(), "kill"|"stop") {continue;}
                        let mut container = if e.action == "destroy" {ContainerMessage { id:id.clone(),..Default::default() }} else {
                            let filter=serde_json::json!({"id":[id]}).to_string();
                            match docker.list_container_models(Some(true),None,None,Some(&filter)).await {
                                Ok(values)=>values.into_iter().next().map(super::containers::summary).transpose()?.unwrap_or_else(||ContainerMessage{id:id.clone(),..Default::default()}),
                                Err(_)=>ContainerMessage{id:id.clone(),..Default::default()},
                            }
                        };
                        container.state=match e.action.as_str(){"start"|"unpause"=>2,"die"=>5,"pause"=>3,"create"=>1,_=>container.state};
                        container.is_swarm_task |= e.actor.attributes.contains_key("com.docker.swarm.task.id");
                        Kind::DaemonContainerEventResponse(DaemonContainerEventResponse{action:e.action,container_id:id,container:Some(container)})
                    }
                    "image"=>{
                        if !matches!(e.action.as_str(),"delete"|"create"|"pull"){continue;}
                        let image=if e.action=="delete" {None} else {docker.list_image_models().await.ok().and_then(|images|images.into_iter().find(|v|v.id==id)).map(super::images::summary)};
                        Kind::DaemonImageEventResponse(DaemonImageEventResponse{action:e.action,image_id:id,image})
                    }
                    "volume"=>{
                        if !matches!(e.action.as_str(),"destroy"|"create"){continue;}
                        let volume=if e.action=="destroy"{None}else{docker.inspect_volume(&id).await.ok().map(super::volumes::volume)};
                        Kind::DaemonVolumeEventResponse(DaemonVolumeEventResponse{action:e.action,volume_id:id,volume})
                    }
                    "network"=>{
                        if !matches!(e.action.as_str(),"destroy"|"create"){
                            if scope!=2 {continue;}
                            Kind::DaemonResourceEventResponse(DaemonResourceEventResponse{r#type:5,action:e.action,resource_id:id})
                        }else{
                            let network=if e.action=="destroy"{None}else{docker.inspect_network(&id).await.ok().map(|v|{let used=!v.containers.is_empty();super::networks::network(v,used)})};
                            Kind::DaemonNetworkEventResponse(DaemonNetworkEventResponse{action:e.action,network_id:id,network})
                        }
                    }
                    other => {
                        let kind = match other { "builder"=>0,"config"=>1,"node"=>6,"secret"=>8,"service"=>9,_=>continue };
                        Kind::DaemonResourceEventResponse(DaemonResourceEventResponse { r#type: kind, action: e.action, resource_id: id })
                    }
                };
                yield DaemonEventResponse { kind: Some(kind), scope };
            }
        })))
    }
}

fn stat(s: &RuntimePlatformStats) -> PlatformStatMessage {
    let count = |n: i64| n.clamp(0, i32::MAX as i64) as i32;
    PlatformStatMessage {
        memory_usage: s.memory_usage,
        cpu_usage: s.cpu_usage,
        rx_bytes: s.receive_bytes,
        tx_bytes: s.transmit_bytes,
        container_count: count(s.container_count),
        containers_running: count(s.containers_running),
        containers_paused: count(s.containers_paused),
        containers_stopped: count(s.containers_stopped),
        disk_used_bytes: s.disk_used_bytes,
        disk_total_bytes: s.disk_total_bytes,
        disk_usage: s.disk_usage,
    }
}

fn aggregate(
    sample: citadel_adapters::connectors::docker::LocalDockerSample,
    cpus: i64,
) -> Result<RuntimePlatformStats, Status> {
    let mut s = sample
        .platform
        .ok_or_else(|| Status::unavailable("Docker platform metadata unavailable"))?;
    s.memory_usage = if s.mem_total > 0 {
        sample
            .containers
            .stats
            .iter()
            .map(|v| v.memory_active)
            .sum::<f64>()
            / s.mem_total as f64
            * 100.0
    } else {
        0.0
    };
    s.cpu_usage = (sample
        .containers
        .stats
        .iter()
        .map(|v| v.cpu_usage)
        .sum::<f64>()
        / cpus.max(1) as f64
        * 100.0)
        .round()
        / 100.0;
    s.receive_bytes = sample.containers.stats.iter().map(|v| v.rx_bytes).sum();
    s.transmit_bytes = sample.containers.stats.iter().map(|v| v.tx_bytes).sum();
    Ok(s)
}
