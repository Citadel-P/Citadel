use citadel_platforms::RuntimeCapabilityError;
use citadel_platforms::images::{ImageContainer, ImageInspection, ImageInspectionPort, ImageLayer};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

use super::{
    DockerClient,
    runtime::{cancelled_error, normalize_docker_error},
};

impl citadel_platforms::images::ImageDeletionPort for DockerClient {
    fn delete_image<'a>(
        &'a self,
        id: &'a str,
        force: bool,
        no_prune: bool,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<Vec<std::collections::BTreeMap<String, String>>, RuntimeCapabilityError>,
    > {
        Box::pin(async move {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => Err(cancelled_error()),
                result = DockerClient::delete_image(self, id, force, no_prune) => result.map_err(normalize_docker_error),
            }
        })
    }
}

impl ImageInspectionPort for DockerClient {
    fn exposed_ports<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let image = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = DockerClient::inspect_image(self, id) => result.map_err(normalize_docker_error)?,
            };
            let mut ports: Vec<_> = image.config.exposed_ports.into_keys().collect();
            ports.sort();
            Ok(ports)
        })
    }

    fn inspect_image<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImageInspection, RuntimeCapabilityError>> {
        Box::pin(async move {
            let (image, history, containers) = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(cancelled_error()),
                result = async { tokio::try_join!(DockerClient::inspect_image(self,id), self.image_history(id), self.image_containers(id)) } => result.map_err(normalize_docker_error)?,
            };
            let mut value = ImageInspection {
                id: image.id,
                size: image.size,
                os: image.os,
                created: image.created,
                architecture: image.architecture,
                env: image.config.env,
                cmd: image.config.cmd,
                repo_tags: image.repo_tags,
                volumes: image.config.volumes.into_keys().collect(),
                exposed_ports: image.config.exposed_ports.into_keys().collect(),
                labels: image.config.labels.into_iter().collect(),
                layers: history
                    .into_iter()
                    .map(|layer| ImageLayer {
                        id: layer.id,
                        created: layer.created,
                        created_by: layer.created_by,
                        size: layer.size,
                        comment: layer.comment,
                    })
                    .collect(),
                containers: containers
                    .into_iter()
                    .map(|container| ImageContainer {
                        id: container.summary.id,
                        name: container
                            .summary
                            .names
                            .into_iter()
                            .next()
                            .unwrap_or_default(),
                        state: container.summary.state,
                        volumes: container
                            .mounts
                            .into_iter()
                            .filter_map(|mount| mount.name)
                            .collect(),
                        networks: container
                            .network_settings
                            .networks
                            .into_iter()
                            .map(|(name, network)| {
                                let id = if network.network_id.is_empty() {
                                    name.clone()
                                } else {
                                    network.network_id
                                };
                                (name, id)
                            })
                            .collect(),
                        ports: crate::container_ports::normalize(container.summary.ports),
                    })
                    .collect(),
                ..Default::default()
            };
            value.volumes.sort();
            value.exposed_ports.sort();
            value.set_display_reference();
            Ok(value)
        })
    }
}
