use std::time::Duration;

use async_stream::stream;
use citadel_application::{
    PlatformRuntimePort, RuntimeCapabilityError, RuntimeErrorKind, RuntimeStatsStream,
};
use citadel_domain::{RuntimeContainerSummary, RuntimePlatformInfo, RuntimePlatformStats};
use futures_util::{FutureExt, future::BoxFuture};
use reqwest::StatusCode;
use tokio_util::sync::CancellationToken;

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
            Ok(RuntimePlatformInfo {
                daemon_id: info.id,
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
                .map(|container| RuntimeContainerSummary {
                    id: container.id,
                    name: container
                        .names
                        .first()
                        .map_or_else(String::new, |name| name.trim_start_matches('/').to_owned()),
                    image: container.image,
                    state: container.state.to_ascii_lowercase(),
                })
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

fn bounded_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn cancelled_error() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled, "Docker call cancelled", false)
}

fn normalize_docker_error(error: DockerError) -> RuntimeCapabilityError {
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
        | DockerError::InvalidIdentifier => (RuntimeErrorKind::InvalidRequest, false),
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
}
