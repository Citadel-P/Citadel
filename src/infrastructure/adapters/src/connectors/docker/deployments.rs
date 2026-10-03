//! Local deployment execution, independent of persistence and transport routing.
use super::{DockerClient, DockerError};
use citadel_deployments::{DeploymentError, RuntimeContainerState, RuntimeDeploymentResult};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
const CONTAINER_START_TIMEOUT: Duration = Duration::from_secs(30);

impl DockerClient {
    pub async fn apply_container_config<T: serde::Serialize + ?Sized>(
        &self,
        name: &str,
        image_id: &str,
        body: &T,
        cancellation: &CancellationToken,
    ) -> Result<RuntimeDeploymentResult, DeploymentError> {
        let container_id = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
            result = self.create_container(name, body) => result,
        }
        .map_err(runtime)?;
        let start_result = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
            result = self.start_container(&container_id) => result,
        };
        if let Err(error) = start_result {
            // Docker may have accepted Start before the response failed. Inspect the
            // already-owned Container instead of retrying an ambiguous mutation.
            tracing::warn!(%error, %container_id, "Deployment container Start returned an error; observing its state");
        }
        let state = self.wait_for_container(&container_id, cancellation).await?;
        Ok(RuntimeDeploymentResult {
            docker_container_id: container_id,
            docker_image_id: image_id.to_owned(),
            state,
        })
    }

    async fn wait_for_container(
        &self,
        container_id: &str,
        cancellation: &CancellationToken,
    ) -> Result<RuntimeContainerState, DeploymentError> {
        let deadline = Instant::now() + CONTAINER_START_TIMEOUT;
        loop {
            let inspect = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
                result = self.inspect_container(container_id) => result,
            }
            .map_err(runtime)?;
            if let Some(state) = observed_container_state(&inspect.state) {
                return Ok(state);
            }
            if Instant::now() >= deadline {
                return Ok(RuntimeContainerState::Timeout);
            }
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(DeploymentError::Cancelled),
                () = tokio::time::sleep(Duration::from_millis(500)) => {}
            }
        }
    }
}

pub(crate) fn observed_container_state(
    state: &crate::connectors::docker::projection::ContainerState,
) -> Option<RuntimeContainerState> {
    if let Some(health) = &state.health {
        if health.status.eq_ignore_ascii_case("starting") {
            return None;
        }
        return Some(if health.status.eq_ignore_ascii_case("healthy") {
            RuntimeContainerState::Running
        } else {
            RuntimeContainerState::Exited
        });
    }
    if state.running {
        return Some(RuntimeContainerState::Running);
    }
    (state.status.eq_ignore_ascii_case("exited") || state.dead)
        .then_some(RuntimeContainerState::Exited)
}

fn runtime(error: DockerError) -> DeploymentError {
    DeploymentError::Runtime(error.to_string())
}

use serde_json::{Value, json};
pub fn parse_port(value: &str) -> Result<(String, Option<u16>), DeploymentError> {
    let (port, protocol) = value.split_once('/').unwrap_or((value, "tcp"));
    if !matches!(protocol, "tcp" | "udp" | "sctp") {
        return Err(DeploymentError::Validation(format!(
            "Deployment port '{value}' uses an unsupported protocol."
        )));
    }
    let (host, container) = port
        .split_once(':')
        .map_or((None, port), |(host, container)| (Some(host), container));
    let container = container.parse::<u16>().map_err(|_| {
        DeploymentError::Validation(format!("Deployment port '{value}' is invalid."))
    })?;
    let host = host.map(str::parse::<u16>).transpose().map_err(|_| {
        DeploymentError::Validation(format!("Deployment port '{value}' is invalid."))
    })?;
    Ok((format!("{container}/{protocol}"), host))
}

pub fn parse_mount(value: &str) -> Result<Value, DeploymentError> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(DeploymentError::Validation(format!(
            "Deployment volume '{value}' is invalid."
        )));
    }
    let (source, target, read_only) = match parts.as_slice() {
        [target] => (None, *target, false),
        [source, target] => (Some(*source), *target, false),
        [source, target, mode]
            if mode.eq_ignore_ascii_case("ro") || mode.eq_ignore_ascii_case("rw") =>
        {
            (Some(*source), *target, mode.eq_ignore_ascii_case("ro"))
        }
        _ => {
            return Err(DeploymentError::Validation(format!(
                "Deployment volume '{value}' is invalid."
            )));
        }
    };
    if !target.starts_with('/') {
        return Err(DeploymentError::Validation(format!(
            "Deployment volume target '{target}' must be an absolute container path."
        )));
    }
    let kind = source.map_or("volume", |source| {
        if source.starts_with('/') || source.starts_with("./") || source.starts_with("../") {
            "bind"
        } else {
            "volume"
        }
    });
    Ok(json!({
        "Type": kind,
        "Source": source,
        "Target": target,
        "ReadOnly": read_only,
    }))
}
