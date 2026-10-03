//! Direct transport and protobuf boundaries over the shared local runtime.
mod auth;
mod container_config;
mod container_mapping;
mod containers;
mod deployments;
mod image_builds;
mod images;
mod networks;
mod platforms;
mod sampling;
mod stack_source;
mod stacks;
mod swarm;
mod swarm_mapping;
mod swarm_spec;
mod volumes;

use axum::{Router, middleware};
use citadel_adapters::connectors::docker::{DockerClient, DockerError};
use citadel_contracts::citadel::{
    networks::v1::network_service_server::NetworkServiceServer,
    volumes::v1::volume_service_server::VolumeServiceServer,
};
use ed25519_dalek::VerifyingKey;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tonic::Status;

pub const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone)]
pub struct Runtime {
    docker: DockerClient,
    samples: Arc<sampling::SharedSampler>,
    mutations: Arc<tokio::sync::Semaphore>,
    docker_cli: String,
    shutdown: CancellationToken,
    runtime_container: Option<String>,
}

impl Runtime {
    pub(crate) fn new(
        docker: DockerClient,
        shutdown: CancellationToken,
        runtime_container: Option<String>,
    ) -> Self {
        Self {
            mutations: Arc::new(tokio::sync::Semaphore::new(8)),
            samples: Arc::new(sampling::SharedSampler::new(docker.clone())),
            docker,
            docker_cli: "docker".into(),
            shutdown,
            runtime_container,
        }
    }
}

pub fn router(
    docker: DockerClient,
    key: VerifyingKey,
    shutdown: CancellationToken,
    runtime_container: Option<String>,
) -> Router {
    let verifier = auth::Verifier::new(key).with_shutdown(shutdown.clone());
    let runtime = Runtime::new(docker, shutdown, runtime_container);
    tonic::service::Routes::new(citadel_contracts::citadel::containers::v1::container_service_server::ContainerServiceServer::new(runtime.clone()).max_decoding_message_size(MAX_MESSAGE_BYTES).max_encoding_message_size(MAX_MESSAGE_BYTES))
        .add_service(citadel_contracts::citadel::platforms::v1::platform_service_server::PlatformServiceServer::new(runtime.clone()).max_decoding_message_size(MAX_MESSAGE_BYTES).max_encoding_message_size(MAX_MESSAGE_BYTES))
        .add_service(citadel_contracts::citadel::deployments::v1::deployment_service_server::DeploymentServiceServer::new(runtime.clone()).max_decoding_message_size(MAX_MESSAGE_BYTES).max_encoding_message_size(MAX_MESSAGE_BYTES))
        .add_service(citadel_contracts::citadel::images::v1::image_service_server::ImageServiceServer::new(runtime.clone()).max_decoding_message_size(MAX_MESSAGE_BYTES).max_encoding_message_size(MAX_MESSAGE_BYTES))
        .add_service(citadel_contracts::citadel::swarm::v1::swarm_service_server::SwarmServiceServer::new(runtime.clone()).max_decoding_message_size(MAX_MESSAGE_BYTES).max_encoding_message_size(MAX_MESSAGE_BYTES))
        .add_service(citadel_contracts::citadel::stacks::v1::stack_service_server::StackServiceServer::new(runtime.clone()).max_decoding_message_size(MAX_MESSAGE_BYTES).max_encoding_message_size(MAX_MESSAGE_BYTES))
        .add_service(VolumeServiceServer::new(runtime.clone())
        .max_decoding_message_size(MAX_MESSAGE_BYTES).max_encoding_message_size(MAX_MESSAGE_BYTES))
        .add_service(NetworkServiceServer::new(runtime)
            .max_decoding_message_size(MAX_MESSAGE_BYTES).max_encoding_message_size(MAX_MESSAGE_BYTES))
        .into_axum_router()
        .layer(middleware::from_fn_with_state(Arc::new(verifier), auth::authenticate))
}

fn docker_error(error: DockerError) -> Status {
    match error {
        DockerError::InvalidIdentifier | DockerError::InvalidEndpoint(_) => {
            Status::invalid_argument(error.to_string())
        }
        DockerError::Api { status, message } => {
            let detail = serde_json::from_str::<serde_json::Value>(&message)
                .ok()
                .and_then(|v| v["message"].as_str().map(str::to_owned))
                .unwrap_or_else(|| "Docker rejected the operation".into());
            match status.as_u16() {
                400 | 422 => Status::invalid_argument(detail),
                401 => Status::unauthenticated(detail),
                403 => Status::permission_denied(detail),
                404 => Status::not_found(detail),
                409 => Status::failed_precondition(detail),
                429 => Status::resource_exhausted(detail),
                _ => Status::unavailable(detail),
            }
        }
        DockerError::Transport(ref error) if error.is_timeout() => {
            Status::deadline_exceeded("Docker request timed out")
        }
        DockerError::ResponseTooLarge { .. } | DockerError::StreamItemTooLarge { .. } => {
            Status::resource_exhausted(error.to_string())
        }
        DockerError::InvalidJson(_) | DockerError::ProtocolIo(_) => {
            Status::data_loss(error.to_string())
        }
        _ => Status::unavailable(error.to_string()),
    }
}

fn filters<const N: usize>(values: [(&str, Option<String>); N]) -> Option<String> {
    let values: std::collections::BTreeMap<_, _> = values
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, vec![value])))
        .collect();
    (!values.is_empty()).then(|| serde_json::to_string(&values).expect("string filters"))
}

fn text(value: &serde_json::Value, name: &str) -> String {
    value[name].as_str().unwrap_or_default().to_owned()
}
fn strings(value: &serde_json::Value) -> std::collections::HashMap<String, String> {
    value
        .as_object()
        .into_iter()
        .flatten()
        .map(|(key, value)| {
            (
                key.clone(),
                value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string()),
            )
        })
        .collect()
}

fn runtime_error(error: citadel_platforms::RuntimeCapabilityError) -> Status {
    use citadel_platforms::RuntimeErrorKind::*;
    let code = match error.kind {
        Cancelled => tonic::Code::Cancelled,
        Timeout => tonic::Code::DeadlineExceeded,
        Authentication => tonic::Code::Unauthenticated,
        PermissionDenied => tonic::Code::PermissionDenied,
        InvalidRequest => tonic::Code::InvalidArgument,
        NotFound => tonic::Code::NotFound,
        Conflict => tonic::Code::FailedPrecondition,
        ResourceExhausted => tonic::Code::ResourceExhausted,
        Unavailable | Remote => tonic::Code::Unavailable,
    };
    Status::new(code, error.message)
}
fn interval(milliseconds: i32) -> std::time::Duration {
    std::time::Duration::from_millis(if milliseconds <= 0 {
        1000
    } else {
        milliseconds.max(100) as u64
    })
}
fn version() -> String {
    crate::VERSION.to_owned()
}

#[cfg(test)]
mod runtime_audit_tests;
