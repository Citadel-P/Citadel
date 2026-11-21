use citadel_platforms::RuntimeCapabilityError;
use citadel_platforms::images::{ImageContainer, ImageInspection, ImageInspectionPort, ImageLayer};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

use super::{
    DockerClient,
    runtime::{cancelled_error, normalize_docker_error},
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ImageMetadata {
    id: Option<String>,
    descriptor: Option<citadel_docker_api::models::OciDescriptor>,
    repo_tags: Option<Vec<String>>,
    os: Option<String>,
    architecture: Option<String>,
    variant: Option<String>,
}
impl DockerClient {
    async fn read_image_identity(
        &self,
        id: &str,
    ) -> Result<citadel_platforms::images::ImageIdentity, super::DockerError> {
        super::transport::validate_identifier(id)?;
        let bytes = self
            .bounded_json_bytes(&format!("/images/{}/json", urlencoding::encode(id)))
            .await?;
        let image: ImageMetadata = serde_json::from_slice(&bytes)?;
        let (config_digest, content_digest) =
            super::mapping::image_digests(image.id.as_deref(), image.descriptor.as_ref());
        Ok(citadel_platforms::images::ImageIdentity {
            id: image.id.unwrap_or_default(),
            config_digest,
            content_digest,
            repo_tags: image.repo_tags.unwrap_or_default(),
            os: image.os.unwrap_or_default(),
            architecture: image.architecture.unwrap_or_default(),
            variant: image.variant,
        })
    }
    pub async fn image_digest_catalog(
        &self,
    ) -> Result<std::collections::BTreeMap<String, Vec<String>>, super::DockerError> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Entry {
            id: String,
            repo_digests: Option<Vec<String>>,
        }
        let _list = citadel_runtime::runtime_metrics::RuntimeWork::DockerImageList.start();
        let bytes = self.bounded_json_bytes("/images/json?all=true").await?;
        Ok(serde_json::from_slice::<Vec<Entry>>(&bytes)?
            .into_iter()
            .map(|e| (e.id, e.repo_digests.unwrap_or_default()))
            .collect())
    }
}

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

    fn image_identity<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<citadel_platforms::images::ImageIdentity, RuntimeCapabilityError>>
    {
        Box::pin(async move {
            tokio::select! {
                biased;
                ()=cancellation.cancelled()=>Err(cancelled_error()),
                result=self.read_image_identity(id)=>result.map_err(normalize_docker_error),
            }
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
                config_digest: image.config_digest,
                content_digest: image.content_digest,
                size: image.size,
                os: image.os,
                created: image.created,
                architecture: image.architecture,
                variant: image.variant,
                user: image.config.user,
                working_dir: image.config.working_dir,
                entry_point: image.config.entry_point,
                stop_signal: image.config.stop_signal,
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
                        ports: crate::containers::ports::normalize(container.summary.ports),
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
