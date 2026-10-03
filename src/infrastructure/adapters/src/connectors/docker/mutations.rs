use citadel_platforms::{
    CreateRuntimeNetwork, CreateRuntimeVolume, CreatedRuntimeNetwork, RuntimeCapabilityError,
    RuntimeVolumeSummary,
};
use futures_util::{FutureExt, future::BoxFuture};
use tokio_util::sync::CancellationToken;

use super::projection::{NetworkCreateRequest, VolumeCreateOptions};
use super::{DockerClient, DockerError};
use crate::connectors::docker::inventory::map_volume;

impl citadel_platforms::NetworkMutationPort for DockerClient {
    fn create_network<'a>(
        &'a self,
        input: &'a CreateRuntimeNetwork,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<CreatedRuntimeNetwork, RuntimeCapabilityError>> {
        async move {
            let request = NetworkCreateRequest {
                name: input.name.clone(),
                driver: Some(input.driver.clone()),
                scope: Some(input.scope.clone()),
                internal: input.internal,
                attachable: input.attachable,
                ingress: input.ingress,
                enable_ipv6: input.enable_ipv6,
                enable_ipv4: input.enable_ipv4,
                config_only: input.config_only,
                ipam: input.ipam.as_ref().map(|ipam| {
                    serde_json::json!({
                        "Driver": ipam.driver,
                        "Config": ipam.config.iter().filter_map(|config| {
                            let fields: serde_json::Map<String, serde_json::Value> = [
                                ("Subnet", &config.subnet), ("IPRange", &config.ip_range), ("Gateway", &config.gateway),
                            ].into_iter().filter_map(|(key, value)| {
                                value.as_ref().filter(|value| !value.is_empty())
                                    .map(|value| (key.to_owned(), serde_json::Value::String(value.clone())))
                            }).collect();
                            (!fields.is_empty()).then_some(serde_json::Value::Object(fields))
                        }).collect::<Vec<_>>(),
                        "Options": ipam.options,
                    })
                }),
                config_from: input
                    .config_from
                    .as_ref()
                    .map(|config| serde_json::json!({ "Network": config.network })),
                labels: input.labels.clone().into_iter().collect(),
                options: input.options.clone().into_iter().collect(),
            };
            let result = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled()),
                result = DockerClient::create_network(self, &request) => result,
            }
            .map_err(normalize)?;
            Ok(CreatedRuntimeNetwork { id: result.id })
        }
        .boxed()
    }

    fn delete_network<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        async move {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => Err(cancelled()),
                result = DockerClient::delete_network(self, id) => result.map_err(normalize),
            }
        }
        .boxed()
    }
}

impl citadel_platforms::VolumeMutationPort for DockerClient {
    fn create_volume<'a>(
        &'a self,
        input: &'a CreateRuntimeVolume,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        async move {
            let request = VolumeCreateOptions {
                name: input.name.clone(),
                driver: input.driver.clone(),
                driver_options: input.options.clone().into_iter().collect(),
                labels: input.labels.clone().into_iter().collect(),
            };
            tokio::select! {
                biased;
                () = cancellation.cancelled() => Err(cancelled()),
                result = DockerClient::create_volume(self, &request) => result.map(map_volume).map_err(normalize),
            }
        }
        .boxed()
    }

    fn delete_volume<'a>(
        &'a self,
        name: &'a str,
        force: bool,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        async move {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => Err(cancelled()),
                result = DockerClient::delete_volume(self, name, force) => result.map_err(normalize),
            }
        }
        .boxed()
    }
}

fn cancelled() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        citadel_platforms::RuntimeErrorKind::Cancelled,
        "operation cancelled",
        false,
    )
}

fn normalize(error: DockerError) -> RuntimeCapabilityError {
    super::runtime::normalize_docker_error(error)
}
