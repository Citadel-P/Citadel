use std::time::Duration;

use async_stream::stream;
use citadel_platforms::{
    PlatformRuntimePort, RuntimeCapabilityError, RuntimeErrorKind, RuntimeStatsStream,
};
use citadel_platforms::{
    RuntimeContainerSummary, RuntimePlatformInfo, RuntimePlatformStats, RuntimeSwarmInfo,
    RuntimeSwarmPeer,
};
use futures_util::{FutureExt, future::BoxFuture};
use reqwest::StatusCode;
use tokio_util::sync::CancellationToken;

use super::generated::ContainerSummary;
use super::{DockerClient, DockerError};

impl PlatformRuntimePort for DockerClient {
    fn get_info<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        async move {
            let value = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                value = async {
                    let version = self.version().await?;
                    let negotiated = self.negotiated_version().await?;
                    let info = self.info().await?;
                    Ok::<_, DockerError>((version, negotiated, info))
                } => value.map_err(normalize_docker_error)?,
            };
            let (version, negotiated, info) = value;
            let swarm = info.swarm.map(|swarm| RuntimeSwarmInfo {
                node_id: swarm.node_id,
                node_addr: swarm.node_addr,
                local_node_state: swarm.local_node_state,
                control_available: swarm.control_available,
                error: (!swarm.error.trim().is_empty()).then_some(swarm.error),
                remote_managers: swarm
                    .remote_managers
                    .into_iter()
                    .map(|manager| RuntimeSwarmPeer {
                        node_id: manager.node_id,
                        address: manager.address,
                    })
                    .collect(),
                nodes: swarm.nodes,
                managers: swarm.managers,
                cluster_id: swarm
                    .cluster
                    .as_ref()
                    .map(|cluster| cluster.id.trim().to_owned())
                    .filter(|value| !value.is_empty()),
                cluster_created_at: swarm.cluster.and_then(|cluster| {
                    chrono::DateTime::parse_from_rfc3339(&cluster.created_at)
                        .ok()
                        .map(|value| value.with_timezone(&chrono::Utc))
                }),
            });
            Ok(RuntimePlatformInfo {
                daemon_id: info.id,
                server_version: version.version,
                operating_system: info.operating_system,
                os_type: info.os_type,
                architecture: info.architecture,
                cpu_count: i64::from(info.cpu_count),
                memory_total: bounded_i64(info.memory_total),
                container_count: bounded_i64(info.containers),
                containers_running: bounded_i64(info.containers_running),
                containers_paused: bounded_i64(info.containers_paused),
                containers_stopped: bounded_i64(info.containers_stopped),
                api_version: negotiated.to_string(),
                minimum_api_version: version.min_api_version,
                agent_version: None,
                swarm,
            })
        }
        .boxed()
    }

    fn list_containers<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
        async move {
            let containers = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = self.list_containers(true) => result.map_err(normalize_docker_error)?,
            };
            let mut containers = containers
                .into_iter()
                .map(map_container)
                .collect::<Vec<_>>();
            containers.sort_unstable_by(|left, right| left.id.cmp(&right.id));
            Ok(containers)
        }
        .boxed()
    }

    fn stream_stats<'a>(
        &'a self,
        fetch_interval: Duration,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeStatsStream, RuntimeCapabilityError>> {
        async move {
            if fetch_interval.is_zero() {
                return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::InvalidRequest,
                    "the local stats interval must be greater than zero",
                    false,
                ));
            }
            let client = self.clone();
            let cancellation = cancellation.clone();
            Ok(Box::pin(stream! {
                let mut interval = tokio::time::interval(fetch_interval);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    tokio::select! {
                        biased;
                        () = cancellation.cancelled() => break,
                        _ = interval.tick() => {}
                    }
                    let result = tokio::select! {
                        biased;
                        () = cancellation.cancelled() => break,
                        result = client.info() => result,
                    };
                    match result {
                        Ok(info) => yield Ok(RuntimePlatformStats {
                            memory_usage: 0.0,
                            cpu_usage: 0.0,
                            receive_bytes: 0.0,
                            transmit_bytes: 0.0,
                            container_count: bounded_i64(info.containers),
                            containers_running: bounded_i64(info.containers_running),
                            containers_paused: bounded_i64(info.containers_paused),
                            containers_stopped: bounded_i64(info.containers_stopped),
                        }),
                        Err(error) => {
                            yield Err(normalize_docker_error(error));
                            break;
                        }
                    }
                }
            }) as RuntimeStatsStream)
        }
        .boxed()
    }
}

fn map_container(container: ContainerSummary) -> RuntimeContainerSummary {
    let is_swarm_task = container.labels.contains_key("com.docker.swarm.task.id");
    let stack = container
        .labels
        .get("com.docker.compose.project")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .or_else(|| {
            is_swarm_task
                .then(|| container.labels.get("com.docker.stack.namespace"))
                .flatten()
                .filter(|value| !value.trim().is_empty())
                .cloned()
        });
    let is_system = container
        .labels
        .get("com.citadel.system")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));
    let system_role = is_system
        .then(|| container.labels.get("com.citadel.system-role"))
        .flatten()
        .cloned();
    let has_citadel_ownership_labels = container.labels.keys().any(|key| {
        key.get(..12)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("com.citadel."))
            || key
                .get(..10)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("x-citadel."))
    });
    RuntimeContainerSummary {
        stack,
        is_system,
        system_role,
        has_citadel_ownership_labels,
        is_swarm_task,
        id: container.id,
        name: container
            .names
            .first()
            .map_or_else(String::new, |name| name.trim_start_matches('/').to_owned()),
        image: container.image,
        image_id: container.image_id,
        created: container.created,
        state: container.state.to_ascii_lowercase(),
        status: container.status,
        labels: container.labels.into_iter().collect(),
        ports: container.ports,
    }
}

fn bounded_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(super) fn cancelled_error() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled, "Docker call cancelled", false)
}

pub(super) fn normalize_docker_error(error: DockerError) -> RuntimeCapabilityError {
    let (kind, retryable) = match &error {
        DockerError::Transport(transport) if transport.is_timeout() => {
            (RuntimeErrorKind::Timeout, true)
        }
        DockerError::Transport(transport) if transport.is_connect() => {
            (RuntimeErrorKind::Unavailable, true)
        }
        DockerError::Api { status, .. } if *status == StatusCode::NOT_FOUND => {
            (RuntimeErrorKind::NotFound, false)
        }
        DockerError::Api { status, .. } if *status == StatusCode::CONFLICT => {
            (RuntimeErrorKind::Conflict, false)
        }
        DockerError::Api { status, .. } if status.is_client_error() => {
            (RuntimeErrorKind::InvalidRequest, false)
        }
        DockerError::Api { status, .. } if status.is_server_error() => {
            (RuntimeErrorKind::Unavailable, true)
        }
        DockerError::UnsupportedPlatform
        | DockerError::InvalidVersion(_)
        | DockerError::IncompatibleVersion { .. }
        | DockerError::InvalidPing(_)
        | DockerError::InvalidIdentifier
        | DockerError::InvalidMethod(_) => (RuntimeErrorKind::InvalidRequest, false),
        DockerError::Transport(_)
        | DockerError::ResponseTooLarge { .. }
        | DockerError::StreamItemTooLarge { .. }
        | DockerError::InvalidJson(_) => (RuntimeErrorKind::Remote, false),
        DockerError::Api { .. } => (RuntimeErrorKind::Remote, false),
    };
    RuntimeCapabilityError::new(kind, error.to_string(), retryable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn maps_docker_failures_without_retrying_client_errors() {
        let not_found = normalize_docker_error(DockerError::Api {
            status: StatusCode::NOT_FOUND,
            message: "missing".to_owned(),
        });
        assert_eq!(not_found.kind, RuntimeErrorKind::NotFound);
        assert!(!not_found.retryable);

        let unavailable = normalize_docker_error(DockerError::Api {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: "offline".to_owned(),
        });
        assert_eq!(unavailable.kind, RuntimeErrorKind::Unavailable);
        assert!(unavailable.retryable);
    }

    #[test]
    fn container_mapping_matches_citadel_ownership_and_swarm_grouping_rules() {
        let container = map_container(ContainerSummary {
            id: "container-1".to_owned(),
            names: vec!["/web.1.task".to_owned()],
            state: "RUNNING".to_owned(),
            labels: HashMap::from([
                ("com.docker.swarm.task.id".to_owned(), "task-1".to_owned()),
                ("com.docker.stack.namespace".to_owned(), "demo".to_owned()),
                ("com.citadel.system".to_owned(), "TRUE".to_owned()),
                (
                    "com.citadel.system-role".to_owned(),
                    "node-agent".to_owned(),
                ),
            ]),
            ..Default::default()
        });

        assert_eq!(container.name, "web.1.task");
        assert_eq!(container.state, "running");
        assert_eq!(container.stack.as_deref(), Some("demo"));
        assert!(container.is_swarm_task);
        assert!(container.is_system);
        assert_eq!(container.system_role.as_deref(), Some("node-agent"));
        assert!(container.has_citadel_ownership_labels);
    }

    #[test]
    fn legacy_ownership_labels_remain_visible_during_migration() {
        let container = map_container(ContainerSummary {
            labels: HashMap::from([("X-Citadel.Owner".to_owned(), "legacy".to_owned())]),
            ..Default::default()
        });

        assert!(container.has_citadel_ownership_labels);
    }
}
