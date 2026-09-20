//! Explicit protocol-to-adapter normalization. Generated models never cross the adapter boundary.
use super::{DockerError, projection::*};
use citadel_docker_api::models as wire;
use serde::Serialize;
use serde_json::Value;

fn json(value: impl Serialize) -> Result<Value, DockerError> {
    Ok(serde_json::to_value(value)?)
}
fn version(value: Option<Box<wire::ObjectVersion>>) -> ObjectVersion {
    ObjectVersion {
        index: value.and_then(|v| v.index).unwrap_or_default(),
    }
}
impl TryFrom<wire::SystemInfo> for DockerInfo {
    type Error = DockerError;
    fn try_from(v: wire::SystemInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id.unwrap_or_default(),
            containers: v.containers.unwrap_or_default().max(0) as u64,
            containers_running: v.containers_running.unwrap_or_default().max(0) as u64,
            containers_paused: v.containers_paused.unwrap_or_default().max(0) as u64,
            containers_stopped: v.containers_stopped.unwrap_or_default().max(0) as u64,
            images: v.images.unwrap_or_default().max(0) as u64,
            docker_root_dir: v.docker_root_dir.unwrap_or_default(),
            cpu_count: v.ncpu.unwrap_or_default().max(0) as u32,
            memory_total: v.mem_total.unwrap_or_default().max(0) as u64,
            operating_system: v.operating_system.unwrap_or_default(),
            os_type: v.os_type.unwrap_or_default(),
            architecture: v.architecture.unwrap_or_default(),
            swarm: v.swarm.map(|s| DockerSwarmInfo {
                node_id: s.node_id.unwrap_or_default(),
                node_addr: s.node_addr.unwrap_or_default(),
                local_node_state: s
                    .local_node_state
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                control_available: s.control_available.unwrap_or_default(),
                error: s.error.unwrap_or_default(),
                remote_managers: s
                    .remote_managers
                    .flatten()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| DockerPeerNode {
                        node_id: p.node_id.unwrap_or_default(),
                        address: p.addr.unwrap_or_default(),
                    })
                    .collect(),
                nodes: i64::from(s.nodes.flatten().unwrap_or_default()),
                managers: i64::from(s.managers.flatten().unwrap_or_default()),
                cluster: s.cluster.flatten().map(|c| DockerClusterInfo {
                    id: c.id.unwrap_or_default(),
                    created_at: c.created_at.unwrap_or_default(),
                }),
            }),
        })
    }
}
impl TryFrom<wire::ContainerSummary> for ContainerSummary {
    type Error = DockerError;
    fn try_from(v: wire::ContainerSummary) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id.unwrap_or_default(),
            names: v.names.unwrap_or_default(),
            image: v.image.unwrap_or_default(),
            image_id: v.image_id.unwrap_or_default(),
            created: v.created.unwrap_or_default(),
            labels: v.labels.unwrap_or_default(),
            state: v
                .state
                .map(|s| {
                    match s {
                        wire::container_summary::State::Created => "created",
                        wire::container_summary::State::Running => "running",
                        wire::container_summary::State::Paused => "paused",
                        wire::container_summary::State::Restarting => "restarting",
                        wire::container_summary::State::Removing => "removing",
                        wire::container_summary::State::Exited => "exited",
                        wire::container_summary::State::Dead => "dead",
                    }
                    .to_owned()
                })
                .unwrap_or_default(),
            status: v.status.unwrap_or_default(),
            ports: json(v.ports)?,
        })
    }
}
impl TryFrom<wire::ContainerSummary> for ImageUsageContainer {
    type Error = DockerError;
    fn try_from(mut v: wire::ContainerSummary) -> Result<Self, Self::Error> {
        let mounts = v
            .mounts
            .take()
            .unwrap_or_default()
            .into_iter()
            .map(|m| ImageUsageMount { name: m.name })
            .collect();
        let networks = v
            .network_settings
            .take()
            .and_then(|n| n.networks)
            .unwrap_or_default()
            .into_iter()
            .map(|(name, n)| {
                (
                    name,
                    ImageUsageNetwork {
                        network_id: n.network_id.unwrap_or_default(),
                    },
                )
            })
            .collect();
        Ok(Self {
            summary: v.try_into()?,
            mounts,
            network_settings: ImageUsageNetworks { networks },
        })
    }
}
impl From<wire::ContainerState> for ContainerState {
    fn from(v: wire::ContainerState) -> Self {
        Self {
            status: v
                .status
                .map(|s| {
                    match s {
                        wire::container_state::Status::Created => "created",
                        wire::container_state::Status::Running => "running",
                        wire::container_state::Status::Paused => "paused",
                        wire::container_state::Status::Restarting => "restarting",
                        wire::container_state::Status::Removing => "removing",
                        wire::container_state::Status::Exited => "exited",
                        wire::container_state::Status::Dead => "dead",
                    }
                    .to_owned()
                })
                .unwrap_or_default(),
            running: v.running.unwrap_or_default(),
            paused: v.paused.unwrap_or_default(),
            restarting: v.restarting.unwrap_or_default(),
            oom_killed: v.oom_killed.unwrap_or_default(),
            dead: v.dead.unwrap_or_default(),
            pid: i64::from(v.pid.unwrap_or_default()),
            exit_code: i64::from(v.exit_code.unwrap_or_default()),
            error: v.error.unwrap_or_default(),
            started_at: v.started_at.unwrap_or_default(),
            finished_at: v.finished_at.unwrap_or_default(),
            health: v.health.flatten().map(|h| ContainerHealth {
                status: h
                    .status
                    .map(|s| {
                        match s {
                            wire::health::Status::None => "none",
                            wire::health::Status::Starting => "starting",
                            wire::health::Status::Healthy => "healthy",
                            wire::health::Status::Unhealthy => "unhealthy",
                        }
                        .to_owned()
                    })
                    .unwrap_or_default(),
            }),
        }
    }
}
impl TryFrom<wire::ContainerInspectResponse> for ContainerInspect {
    type Error = DockerError;
    fn try_from(v: wire::ContainerInspectResponse) -> Result<Self, Self::Error> {
        let c = v.config.unwrap_or_default();
        Ok(Self {
            id: v.id.unwrap_or_default(),
            created: v.created.flatten().unwrap_or_default(),
            path: v.path.unwrap_or_default(),
            args: v.args.unwrap_or_default(),
            state: v.state.flatten().map(|s| (*s).into()).unwrap_or_default(),
            image: v.image.unwrap_or_default(),
            name: v.name.unwrap_or_default(),
            config: ContainerConfig {
                environment: c.env.unwrap_or_default(),
                image: c.image.unwrap_or_default(),
                labels: c.labels.unwrap_or_default(),
            },
        })
    }
}
impl TryFrom<wire::ImageSummary> for ImageSummary {
    type Error = DockerError;
    fn try_from(v: wire::ImageSummary) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id,
            repo_tags: v.repo_tags.unwrap_or_default(),
            repo_digests: v.repo_digests.unwrap_or_default(),
            created: i64::from(v.created),
            size: v.size,
            labels: v.labels.unwrap_or_default(),
            containers: i64::from(v.containers),
        })
    }
}
impl TryFrom<wire::ImageInspect> for ImageInspect {
    type Error = DockerError;
    fn try_from(v: wire::ImageInspect) -> Result<Self, Self::Error> {
        let c = v.config.unwrap_or_default();
        Ok(Self {
            id: v.id.unwrap_or_default(),
            repo_tags: v.repo_tags.unwrap_or_default(),
            size: v.size.unwrap_or_default(),
            created: v.created.flatten().unwrap_or_default(),
            os: v.os.unwrap_or_default(),
            architecture: v.architecture.unwrap_or_default(),
            config: ImageConfig {
                env: c.env.unwrap_or_default(),
                cmd: c.cmd.unwrap_or_default(),
                volumes: c.volumes.unwrap_or_default(),
                exposed_ports: c.exposed_ports.flatten().unwrap_or_default(),
                labels: c.labels.unwrap_or_default(),
            },
        })
    }
}
impl TryFrom<wire::HistoryResponseItem> for ImageHistoryItem {
    type Error = DockerError;
    fn try_from(v: wire::HistoryResponseItem) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id,
            created: v.created,
            created_by: v.created_by,
            size: v.size,
            comment: v.comment,
        })
    }
}
impl TryFrom<wire::Volume> for DockerVolume {
    type Error = DockerError;
    fn try_from(v: wire::Volume) -> Result<Self, Self::Error> {
        Ok(Self {
            name: v.name,
            driver: v.driver,
            mountpoint: v.mountpoint,
            created_at: v.created_at.unwrap_or_default(),
            status: v.status.unwrap_or_default(),
            labels: v.labels.unwrap_or_default(),
            scope: v
                .scope
                .map(|s| {
                    match s {
                        wire::volume::Scope::Local => "local",
                        wire::volume::Scope::Global => "global",
                    }
                    .to_owned()
                })
                .unwrap_or_default(),
            cluster_volume: v.cluster_volume.map(json).transpose()?,
            options: v.options.unwrap_or_default(),
            usage_data: v.usage_data.flatten().map(json).transpose()?,
        })
    }
}
impl TryFrom<wire::VolumeListResponse> for VolumeListResponse {
    type Error = DockerError;
    fn try_from(v: wire::VolumeListResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            volumes: v
                .volumes
                .unwrap_or_default()
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            warnings: v.warnings.unwrap_or_default(),
        })
    }
}
impl TryFrom<wire::Network> for DockerNetwork {
    type Error = DockerError;
    fn try_from(v: wire::Network) -> Result<Self, Self::Error> {
        Ok(Self {
            name: v.name.unwrap_or_default(),
            id: v.id.unwrap_or_default(),
            created: v.created.unwrap_or_default(),
            scope: v.scope.unwrap_or_default(),
            driver: v.driver.unwrap_or_default(),
            enable_ipv4: v.enable_ipv4.unwrap_or_default(),
            enable_ipv6: v.enable_ipv6.unwrap_or_default(),
            ipam: v.ipam.map(json).transpose()?,
            internal: v.internal.unwrap_or_default(),
            attachable: v.attachable.unwrap_or_default(),
            ingress: v.ingress.unwrap_or_default(),
            config_from: v.config_from.map(json).transpose()?,
            config_only: v.config_only.unwrap_or_default(),
            containers: v
                .containers
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| Ok((k, json(v)?)))
                .collect::<Result<_, DockerError>>()?,
            peers: v
                .peers
                .flatten()
                .unwrap_or_default()
                .into_iter()
                .map(json)
                .collect::<Result<_, _>>()?,
            options: v.options.unwrap_or_default(),
            labels: v.labels.unwrap_or_default(),
        })
    }
}
impl TryFrom<wire::Swarm> for SwarmInspect {
    type Error = DockerError;
    fn try_from(v: wire::Swarm) -> Result<Self, Self::Error> {
        let t = v.join_tokens.unwrap_or_default();
        Ok(Self {
            id: v.id.unwrap_or_default(),
            version: version(v.version),
            created_at: v.created_at.unwrap_or_default(),
            updated_at: v.updated_at.unwrap_or_default(),
            root_rotation_in_progress: v.root_rotation_in_progress.unwrap_or_default(),
            join_tokens: JoinTokens {
                worker: t.worker.unwrap_or_default(),
                manager: t.manager.unwrap_or_default(),
            },
        })
    }
}
impl TryFrom<wire::Node> for SwarmNode {
    type Error = DockerError;
    fn try_from(v: wire::Node) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id.unwrap_or_default(),
            version: version(v.version),
            created_at: v.created_at.unwrap_or_default(),
            updated_at: v.updated_at.unwrap_or_default(),
            spec: json(v.spec)?,
            description: json(v.description)?,
            status: json(v.status)?,
            manager_status: json(v.manager_status.flatten())?,
        })
    }
}
impl TryFrom<wire::Service> for SwarmService {
    type Error = DockerError;
    fn try_from(v: wire::Service) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id.unwrap_or_default(),
            version: version(v.version),
            created_at: v.created_at.unwrap_or_default(),
            updated_at: v.updated_at.unwrap_or_default(),
            spec: json(v.spec)?,
            endpoint: json(v.endpoint)?,
            update_status: json(v.update_status)?,
            service_status: json(v.service_status)?,
        })
    }
}
impl TryFrom<wire::Task> for SwarmTask {
    type Error = DockerError;
    fn try_from(v: wire::Task) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id.unwrap_or_default(),
            version: version(v.version),
            created_at: v.created_at.unwrap_or_default(),
            updated_at: v.updated_at.unwrap_or_default(),
            name: v.name.unwrap_or_default(),
            spec: json(v.spec)?,
            service_id: v.service_id.unwrap_or_default(),
            slot: v.slot,
            node_id: v.node_id.unwrap_or_default(),
            status: json(v.status)?,
            desired_state: v.desired_state.map(|s| s.to_string()).unwrap_or_default(),
        })
    }
}
impl TryFrom<wire::Secret> for SwarmSecret {
    type Error = DockerError;
    fn try_from(v: wire::Secret) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id.unwrap_or_default(),
            version: version(v.version),
            created_at: v.created_at.unwrap_or_default(),
            updated_at: v.updated_at.unwrap_or_default(),
            spec: json(v.spec)?,
        })
    }
}
impl TryFrom<wire::Config> for SwarmConfig {
    type Error = DockerError;
    fn try_from(v: wire::Config) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id.unwrap_or_default(),
            version: version(v.version),
            created_at: v.created_at.unwrap_or_default(),
            updated_at: v.updated_at.unwrap_or_default(),
            spec: json(v.spec)?,
        })
    }
}
impl From<wire::ContainerCpuStats> for CpuStats {
    fn from(v: wire::ContainerCpuStats) -> Self {
        let c = v.cpu_usage.flatten().unwrap_or_default();
        Self {
            cpu_usage: CpuUsage {
                total_usage: c.total_usage.unwrap_or_default(),
                percpu_usage: c.percpu_usage.flatten().unwrap_or_default(),
            },
            system_cpu_usage: v.system_cpu_usage.flatten().unwrap_or_default(),
            online_cpus: v.online_cpus.flatten().unwrap_or_default(),
        }
    }
}
impl TryFrom<wire::ContainerStatsResponse> for ContainerStats {
    type Error = DockerError;
    fn try_from(v: wire::ContainerStatsResponse) -> Result<Self, Self::Error> {
        let m = v.memory_stats.unwrap_or_default();
        Ok(Self {
            name: v.name.flatten().unwrap_or_default(),
            id: v.id.flatten().unwrap_or_default(),
            read: v.read.map(|d| d.to_rfc3339()).unwrap_or_default(),
            preread: v.preread.map(|d| d.to_rfc3339()).unwrap_or_default(),
            cpu_stats: v
                .cpu_stats
                .flatten()
                .map(|c| (*c).into())
                .unwrap_or_default(),
            precpu_stats: v
                .precpu_stats
                .flatten()
                .map(|c| (*c).into())
                .unwrap_or_default(),
            memory_stats: MemoryStats {
                usage: m.usage.flatten().unwrap_or_default(),
                stats: m.stats.unwrap_or_default(),
                limit: m.limit.flatten().unwrap_or_default(),
            },
            networks: v
                .networks
                .flatten()
                .unwrap_or_default()
                .into_iter()
                .map(|(k, n)| {
                    (
                        k,
                        NetworkStats {
                            rx_bytes: n.rx_bytes.unwrap_or_default(),
                            rx_packets: n.rx_packets.unwrap_or_default(),
                            rx_errors: n.rx_errors.unwrap_or_default(),
                            rx_dropped: n.rx_dropped.unwrap_or_default(),
                            tx_bytes: n.tx_bytes.unwrap_or_default(),
                            tx_packets: n.tx_packets.unwrap_or_default(),
                            tx_errors: n.tx_errors.unwrap_or_default(),
                            tx_dropped: n.tx_dropped.unwrap_or_default(),
                        },
                    )
                })
                .collect(),
        })
    }
}
