use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use citadel_platforms::jobs::ContainerStatsSampler;
use citadel_platforms::{
    PlatformInventoryPort, RuntimeCapabilityError, RuntimeContainerStat, RuntimeImageSummary,
    RuntimeNetworkSummary, RuntimeSwarmConfig, RuntimeSwarmNode, RuntimeSwarmSecret,
    RuntimeSwarmService, RuntimeSwarmTask, RuntimeVolumeSummary,
};
use futures_util::{FutureExt, future::BoxFuture};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::DockerClient;
use super::generated::{
    ContainerStats, DockerNetwork, DockerVolume, ImageSummary, SwarmConfig, SwarmNode, SwarmSecret,
    SwarmService, SwarmTask,
};
use super::runtime::{cancelled_error, normalize_docker_error};

impl PlatformInventoryPort for DockerClient {
    fn list_images<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeImageSummary>, RuntimeCapabilityError>> {
        async move {
            let values = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::list_images(self) => result.map_err(normalize_docker_error)?,
            };
            Ok(values.into_iter().map(map_image).collect())
        }
        .boxed()
    }

    fn list_networks<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeNetworkSummary>, RuntimeCapabilityError>> {
        async move {
            let values = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::list_networks(self) => result.map_err(normalize_docker_error)?,
            };
            Ok(values.into_iter().map(map_network).collect())
        }
        .boxed()
    }

    fn inspect_network<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeNetworkSummary, RuntimeCapabilityError>> {
        async move {
            let value = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::inspect_network(self, id) => result.map_err(normalize_docker_error)?,
            };
            Ok(map_network(value))
        }
        .boxed()
    }

    fn list_volumes<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeVolumeSummary>, RuntimeCapabilityError>> {
        async move {
            let values = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::list_volumes(self) => result.map_err(normalize_docker_error)?,
            };
            Ok(values.into_iter().map(map_volume).collect())
        }
        .boxed()
    }

    fn inspect_volume<'a>(
        &'a self,
        name: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        async move {
            let value = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::inspect_volume(self, name) => result.map_err(normalize_docker_error)?,
            };
            Ok(map_volume(value))
        }
        .boxed()
    }

    fn list_swarm_nodes<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmNode>, RuntimeCapabilityError>> {
        async move {
            let values = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::list_swarm_nodes(self) => result.map_err(normalize_docker_error)?,
            };
            Ok(values.into_iter().map(map_node).collect())
        }
        .boxed()
    }

    fn list_swarm_services<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmService>, RuntimeCapabilityError>> {
        async move {
            let values = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::list_swarm_services(self) => result.map_err(normalize_docker_error)?,
            };
            Ok(values.into_iter().map(map_service).collect())
        }
        .boxed()
    }

    fn list_swarm_tasks<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmTask>, RuntimeCapabilityError>> {
        async move {
            let values = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::list_swarm_tasks(self) => result.map_err(normalize_docker_error)?,
            };
            Ok(values.into_iter().map(map_task).collect())
        }
        .boxed()
    }

    fn list_swarm_configs<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmConfig>, RuntimeCapabilityError>> {
        async move {
            let values = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::list_swarm_configs(self) => result.map_err(normalize_docker_error)?,
            };
            Ok(values.into_iter().map(map_config).collect())
        }
        .boxed()
    }

    fn list_swarm_secrets<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmSecret>, RuntimeCapabilityError>> {
        async move {
            let values = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::list_swarm_secrets(self) => result.map_err(normalize_docker_error)?,
            };
            Ok(values.into_iter().map(map_secret).collect())
        }
        .boxed()
    }
}

impl DockerClient {
    pub async fn sample_container_stats(
        &self,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<RuntimeContainerStat, RuntimeCapabilityError> {
        let value = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            result = self.container_stats_once(id) => result.map_err(normalize_docker_error)?,
        };
        Ok(map_container_stat(id, value))
    }
}

impl ContainerStatsSampler for DockerClient {
    fn sample_container_stats<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeContainerStat, RuntimeCapabilityError>> {
        async move { DockerClient::sample_container_stats(self, id, cancellation).await }.boxed()
    }
}

fn map_container_stat(id: &str, value: ContainerStats) -> RuntimeContainerStat {
    let usage = value.memory_stats.usage;
    let inactive = value
        .memory_stats
        .stats
        .get("total_inactive_file")
        .or_else(|| value.memory_stats.stats.get("inactive_file"))
        .copied()
        .filter(|inactive| *inactive < usage)
        .unwrap_or_default();
    let memory_cache = value
        .memory_stats
        .stats
        .get("cache")
        .or_else(|| value.memory_stats.stats.get("file"))
        .copied()
        .unwrap_or_default();
    let cpu_delta = value
        .cpu_stats
        .cpu_usage
        .total_usage
        .saturating_sub(value.precpu_stats.cpu_usage.total_usage);
    let system_delta = value
        .cpu_stats
        .system_cpu_usage
        .saturating_sub(value.precpu_stats.system_cpu_usage);
    let cpu_count = if value.cpu_stats.online_cpus > 0 {
        value.cpu_stats.online_cpus as usize
    } else {
        value.cpu_stats.cpu_usage.percpu_usage.len().max(1)
    };
    let cpu_usage = if cpu_delta == 0 || system_delta == 0 {
        0.0
    } else {
        ((cpu_delta as f64 / system_delta as f64) * cpu_count as f64 * 10_000.0).round() / 100.0
    };
    let (rx_bytes, tx_bytes) = value.networks.values().fold((0_u64, 0_u64), |total, item| {
        (
            total.0.saturating_add(item.rx_bytes),
            total.1.saturating_add(item.tx_bytes),
        )
    });
    RuntimeContainerStat {
        docker_container_id: id.to_owned(),
        memory_active: usage.saturating_sub(inactive) as f64,
        memory_cache: memory_cache as f64,
        cpu_usage,
        memory_limit: value.memory_stats.limit as f64,
        rx_bytes: rx_bytes as f64,
        tx_bytes: tx_bytes as f64,
        created: Utc::now().timestamp(),
    }
}

fn map_image(value: ImageSummary) -> RuntimeImageSummary {
    RuntimeImageSummary {
        id: value.id,
        repo_tags: value.repo_tags,
        repo_digests: value.repo_digests,
        created: value.created,
        size: value.size,
        containers: value.containers,
    }
}

fn map_network(value: DockerNetwork) -> RuntimeNetworkSummary {
    RuntimeNetworkSummary {
        id: value.id,
        name: value.name,
        created: value.created,
        driver: value.driver,
        scope: value.scope,
        enable_ipv4: value.enable_ipv4,
        enable_ipv6: value.enable_ipv6,
        internal: value.internal,
        attachable: value.attachable,
        ingress: value.ingress,
        config_only: value.config_only,
        config_from: value
            .config_from
            .as_ref()
            .and_then(|item| string(item, &["Network"])),
        ipam: value.ipam,
        options: value.options.into_iter().collect(),
        labels: value.labels.into_iter().collect(),
        container_count: value.containers.len(),
        containers: value.containers.into_iter().collect(),
        peers: value.peers,
    }
}

pub(super) fn map_volume(value: DockerVolume) -> RuntimeVolumeSummary {
    RuntimeVolumeSummary {
        name: value.name,
        in_use: value
            .usage_data
            .as_ref()
            .and_then(|usage| usage.get("RefCount"))
            .and_then(Value::as_i64)
            .is_some_and(|count| count > 0),
        scope: value.scope,
        driver: value.driver,
        mountpoint: value.mountpoint,
        created_at: value.created_at,
        cluster_volume: value.cluster_volume,
        usage_data: value.usage_data,
        status: value.status.into_iter().collect(),
        labels: value.labels.into_iter().collect(),
        options: value.options.into_iter().collect(),
        containers: Vec::new(),
    }
}

fn map_node(value: SwarmNode) -> RuntimeSwarmNode {
    RuntimeSwarmNode {
        id: value.id,
        version_index: bounded_i64(value.version.index),
        hostname: string_or_default(&value.description, &["Hostname"]),
        role: string_or_default(&value.spec, &["Role"]),
        is_leader: boolean(&value.manager_status, &["Leader"]),
        reachability: string_or_default(&value.manager_status, &["Reachability"]),
        status: string_or_default(&value.status, &["State"]),
        status_message: string(&value.status, &["Message"]),
        availability: string_or_default(&value.spec, &["Availability"]),
        engine_version: string_or_default(&value.description, &["Engine", "EngineVersion"]),
        operating_system: string_or_default(&value.description, &["Platform", "OS"]),
        architecture: string_or_default(&value.description, &["Platform", "Architecture"]),
        address: string_or_default(&value.status, &["Addr"]),
        labels: object_map(&value.spec, &["Labels"]),
        created_at: timestamp(&value.created_at),
        updated_at: timestamp(&value.updated_at),
    }
}

fn map_service(value: SwarmService) -> RuntimeSwarmService {
    let mode = value
        .spec
        .get("Mode")
        .and_then(Value::as_object)
        .and_then(|mode| mode.keys().next())
        .cloned()
        .unwrap_or_default();
    RuntimeSwarmService {
        id: value.id,
        version_index: bounded_i64(value.version.index),
        name: string_or_default(&value.spec, &["Name"]),
        mode,
        image: string_or_default(&value.spec, &["TaskTemplate", "ContainerSpec", "Image"]),
        running_task_count: bounded_i32(unsigned(&value.service_status, &["RunningTasks"])),
        desired_task_count: bounded_i32(unsigned(&value.service_status, &["DesiredTasks"])),
        update_state: string_or_default(&value.update_status, &["State"]),
        update_message: string(&value.update_status, &["Message"]),
        ports: port_strings(&value.endpoint),
        network_ids: string_array(&value.spec, &["TaskTemplate", "Networks"], "Target"),
        secret_ids: string_array(
            &value.spec,
            &["TaskTemplate", "ContainerSpec", "Secrets"],
            "SecretID",
        ),
        config_ids: string_array(
            &value.spec,
            &["TaskTemplate", "ContainerSpec", "Configs"],
            "ConfigID",
        ),
        labels: object_map(&value.spec, &["Labels"]),
        stack_namespace: string(&value.spec, &["Labels", "com.docker.stack.namespace"]),
        ownership: Default::default(),
        ownership_diagnostic: None,
        swarm_service_id: None,
        stack_id: None,
        force_update: signed(&value.spec, &["TaskTemplate", "ForceUpdate"]),
        runtime_hash: String::new(),
        created_at: timestamp(&value.created_at),
        updated_at: timestamp(&value.updated_at),
    }
    .normalize_ownership()
}

fn map_task(value: SwarmTask) -> RuntimeSwarmTask {
    RuntimeSwarmTask {
        id: value.id,
        version_index: bounded_i64(value.version.index),
        name: value.name,
        service_id: value.service_id,
        slot: value.slot,
        node_id: value.node_id,
        desired_state: value.desired_state,
        state: string_or_default(&value.status, &["State"]),
        status_message: string(&value.status, &["Message"]),
        error: string(&value.status, &["Err"]).filter(|value| !value.is_empty()),
        image: string_or_default(&value.spec, &["ContainerSpec", "Image"]),
        ports: Vec::new(),
        container_id: string(&value.status, &["ContainerStatus", "ContainerID"])
            .filter(|value| !value.is_empty()),
        status_timestamp: string(&value.status, &["Timestamp"]).and_then(|value| timestamp(&value)),
        created_at: timestamp(&value.created_at),
        updated_at: timestamp(&value.updated_at),
    }
}

fn map_config(value: SwarmConfig) -> RuntimeSwarmConfig {
    RuntimeSwarmConfig {
        id: value.id,
        version_index: bounded_i64(value.version.index),
        name: string_or_default(&value.spec, &["Name"]),
        templating_driver: string(&value.spec, &["Templating", "Name"]),
        labels: object_map(&value.spec, &["Labels"]),
        created_at: timestamp(&value.created_at),
        updated_at: timestamp(&value.updated_at),
    }
}

fn map_secret(value: SwarmSecret) -> RuntimeSwarmSecret {
    RuntimeSwarmSecret {
        id: value.id,
        version_index: bounded_i64(value.version.index),
        name: string_or_default(&value.spec, &["Name"]),
        driver: string(&value.spec, &["Driver", "Name"]),
        labels: object_map(&value.spec, &["Labels"]),
        created_at: timestamp(&value.created_at),
        updated_at: timestamp(&value.updated_at),
    }
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, key| current.get(key))
}

fn string(value: &Value, path: &[&str]) -> Option<String> {
    at(value, path).and_then(Value::as_str).map(str::to_owned)
}

fn string_or_default(value: &Value, path: &[&str]) -> String {
    string(value, path).unwrap_or_default()
}

fn boolean(value: &Value, path: &[&str]) -> bool {
    at(value, path).and_then(Value::as_bool).unwrap_or(false)
}

fn unsigned(value: &Value, path: &[&str]) -> u64 {
    at(value, path).and_then(Value::as_u64).unwrap_or_default()
}

fn signed(value: &Value, path: &[&str]) -> i64 {
    at(value, path).and_then(Value::as_i64).unwrap_or_default()
}

fn object_map(value: &Value, path: &[&str]) -> BTreeMap<String, String> {
    at(value, path)
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_owned())))
        .collect()
}

fn string_array(value: &Value, path: &[&str], field: &str) -> Vec<String> {
    at(value, path)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| value.get(field).and_then(Value::as_str).map(str::to_owned))
        .collect()
}

fn port_strings(endpoint: &Value) -> Vec<String> {
    endpoint
        .get("Ports")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|port| {
            let target = port
                .get("TargetPort")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let published = port
                .get("PublishedPort")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let protocol = port
                .get("Protocol")
                .and_then(Value::as_str)
                .unwrap_or("tcp");
            if published == 0 {
                format!("{target}/{protocol}")
            } else {
                format!("{published}:{target}/{protocol}")
            }
        })
        .collect()
}

fn timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn bounded_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn bounded_i32(value: u64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker::generated::{ObjectVersion, SwarmService};
    use serde_json::json;

    #[test]
    fn maps_the_swarm_service_fields_used_by_the_projection() {
        let service = map_service(SwarmService {
            id: "service-1".to_owned(),
            version: ObjectVersion { index: 4 },
            spec: json!({
                "Name": "web",
                "Labels": { "com.docker.stack.namespace": "demo" },
                "TaskTemplate": {
                    "ContainerSpec": { "Image": "nginx:alpine" },
                    "Networks": [{ "Target": "network-1" }]
                },
                "Mode": { "Replicated": { "Replicas": 2 } }
            }),
            service_status: json!({ "RunningTasks": 2, "DesiredTasks": 2 }),
            ..Default::default()
        });

        assert_eq!(service.name, "web");
        assert_eq!(service.mode, "Replicated");
        assert_eq!(service.image, "nginx:alpine");
        assert_eq!(service.network_ids, ["network-1"]);
        assert_eq!(service.stack_namespace.as_deref(), Some("demo"));
    }

    #[test]
    fn maps_container_stats_like_docker_stats_without_cache() {
        let value = ContainerStats {
            cpu_stats: super::super::generated::CpuStats {
                cpu_usage: super::super::generated::CpuUsage {
                    total_usage: 300,
                    percpu_usage: vec![1, 1],
                },
                system_cpu_usage: 1_000,
                online_cpus: 2,
            },
            precpu_stats: super::super::generated::CpuStats {
                cpu_usage: super::super::generated::CpuUsage {
                    total_usage: 200,
                    ..Default::default()
                },
                system_cpu_usage: 500,
                ..Default::default()
            },
            memory_stats: super::super::generated::MemoryStats {
                usage: 1_000,
                stats: [("inactive_file".to_owned(), 250), ("file".to_owned(), 300)]
                    .into_iter()
                    .collect(),
                limit: 2_000,
            },
            networks: [(
                "eth0".to_owned(),
                super::super::generated::NetworkStats {
                    rx_bytes: 10,
                    tx_bytes: 20,
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        let stat = map_container_stat("container-1", value);
        assert_eq!(stat.memory_active, 750.0);
        assert_eq!(stat.memory_cache, 300.0);
        assert_eq!(stat.cpu_usage, 40.0);
        assert_eq!(stat.rx_bytes, 10.0);
        assert_eq!(stat.tx_bytes, 20.0);
    }
}
