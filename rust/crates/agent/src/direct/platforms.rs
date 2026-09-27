use super::{Runtime, docker_error, runtime_error};
use citadel_adapters::connectors::docker::events::{
    self, ContainerChange, ResourceChange, RuntimeEventKind, SwarmResource,
};
use citadel_contracts::citadel::{platforms::v1::*, shared_models::v1::*};
use citadel_platforms::{RuntimePlatformStats, prune::PlatformPrunePort};
use futures_util::{StreamExt, stream::BoxStream};
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl platform_service_server::PlatformService for Runtime {
    async fn get_platform_info(
        &self,
        _: Request<()>,
    ) -> Result<Response<PlatformInfoResponse>, Status> {
        let (info, details) = self
            .docker
            .get_info_with_details(&self.shutdown)
            .await
            .map_err(runtime_error)?;
        // Dynamic resource counts and usage belong to the statistics stream;
        // metadata requests must never enumerate all runtime resources.
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
        Ok(Response::new(PlatformInfoResponse {
            id: info.daemon_id,
            created: chrono::Utc::now().timestamp(),
            network_count: 0,
            volume_count: 0,
            agent_version: super::version(),
            image_count: 0,
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
                service_count: None,
                running_task_count: None,
            }),
            platform_stat: None,
            image_used_bytes: None,
            volume_used_bytes: None,
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
        let sampler = self.samples.clone();
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let _guard = guard;
            let info = docker.info().await.map_err(docker_error)?;
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
        let samples = self.samples.clone();
        let mut events = docker.events(None, None).await.map_err(docker_error)?;
        // A newly established stream cannot prove that cached running identities
        // survived the previous connection's gap.
        samples.invalidate();
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            let _guard = guard;
            loop {
                let event = tokio::select! { ()=cancel.cancelled()=>break, event=events.next()=>event };
                let Some(event) = event else { break; };
                let e = event.map_err(docker_error)?;
                use daemon_event_response::Kind;
                let Some(normalized) = events::normalize(&e) else { continue; };
                let scope = match e.scope.as_str() { "local" => 1, "swarm" => 2, _ => 0 };
                let id = e.actor.id;
                let kind = match normalized {
                    RuntimeEventKind::Container(change) => {
                        samples.invalidate();
                        let mut container = if change == ContainerChange::Tombstone { None } else {
                            let filter=serde_json::json!({"id":[id]}).to_string();
                            let observed = async {
                                let Some(mut model) = docker.list_container_models(Some(true),None,None,Some(&filter)).await?
                                    .into_iter().find(|c| c.id.as_deref() == Some(id.as_str())) else { return Ok(None); };
                                if model.image_id.as_deref().is_none_or(str::is_empty) {
                                    let inspected = docker.inspect_container(&id).await?;
                                    if inspected.id != id || inspected.image.is_empty() { return Ok(None); }
                                    model.image_id = Some(inspected.image);
                                }
                                Ok::<_, citadel_adapters::connectors::docker::DockerError>(Some(model))
                            }.await;
                            match observed {
                                Ok(Some(model)) => Some(super::containers::summary(model)?),
                                Ok(None) => None,
                                Err(error) => { tracing::debug!(%error, container_id=%id, "Event metadata unavailable; Core will reconcile"); None }
                            }
                        };
                        if let Some(container) = &mut container {
                            if let Some(state) = change.state() { container.state = super::container_mapping::state(state); }
                            container.is_swarm_task |= e.actor.attributes.contains_key("com.docker.swarm.task.id");
                        }
                        Kind::DaemonContainerEventResponse(DaemonContainerEventResponse{action:e.action,container_id:id,container})
                    }
                    RuntimeEventKind::Image(change)=>{
                        let image=if change == ResourceChange::Tombstone {None} else {docker.image_event_model(&id).await.ok().filter(|image|image.id==id).map(super::images::summary)};
                        Kind::DaemonImageEventResponse(DaemonImageEventResponse{action:e.action,image_id:id,image})
                    }
                    RuntimeEventKind::Volume(change)=>{
                        let volume=if change == ResourceChange::Tombstone{None}else{docker.inspect_volume(&id).await.ok().map(super::volumes::volume)};
                        Kind::DaemonVolumeEventResponse(DaemonVolumeEventResponse{action:e.action,volume_id:id,volume})
                    }
                    RuntimeEventKind::Network(change)=>{
                        let network=if change == ResourceChange::Tombstone {None}else{docker.inspect_network(&id).await.ok().map(|v|{let used=!v.containers.is_empty();super::networks::network(v,used)})};
                        Kind::DaemonNetworkEventResponse(DaemonNetworkEventResponse{action:e.action,network_id:id,network})
                    }
                    RuntimeEventKind::SwarmDirty(resource) => {
                        let kind = match resource { SwarmResource::Config=>1, SwarmResource::Network=>5, SwarmResource::Node=>6, SwarmResource::Secret=>8, SwarmResource::Service=>9 };
                        Kind::DaemonResourceEventResponse(DaemonResourceEventResponse { r#type: kind, action: e.action, resource_id: id })
                    }
                    RuntimeEventKind::Unknown => Kind::DaemonResourceEventResponse(DaemonResourceEventResponse { r#type: -1, action: e.action, resource_id: id }),
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

#[cfg(test)]
#[path = "platforms/event_tests.rs"]
mod event_tests;
