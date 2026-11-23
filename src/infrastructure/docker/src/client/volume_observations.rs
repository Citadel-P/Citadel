//! Fresh, read-only volume evidence; protected mutations still inspect their owner daemon.
use super::{DockerClient, DockerError, projection::DockerVolume};
use citadel_docker_api::models::ContainerSummary;
use futures_util::{StreamExt, stream};

impl DockerClient {
    pub async fn volume_details(
        &self,
        name: &str,
    ) -> Result<(DockerVolume, Vec<ContainerSummary>), DockerError> {
        let generation = self.daemon_generation();
        let inspect = async {
            let mut volume = self.volume_metadata(name).await?;
            self.fill_volume_usage(std::iter::once(&mut volume), false)
                .await?;
            Ok(volume)
        };
        let (mut volume, containers) = tokio::try_join!(
            inspect,
            self.list_container_models(Some(true), None, Some(false), None)
        )?;
        Self::apply_volume_mounts(std::slice::from_mut(&mut volume), &containers);
        let attached = if containers.iter().any(|c| c.mounts.is_none()) {
            // The daemon has stronger membership evidence than an incomplete summary.
            self.volume_containers(name).await?
        } else {
            containers
                .into_iter()
                .filter(|c| {
                    c.mounts.iter().flatten().any(|m| {
                        m.destination.as_deref() == Some(name)
                            || m.name
                                .as_deref()
                                .filter(|v| !v.is_empty())
                                .or(m.source.as_deref())
                                == Some(name)
                    })
                })
                .collect()
        };
        self.check_volume_generation(generation)?;
        Ok((volume, attached))
    }

    /// Names belong to a single contiguous, distinct create-event batch.
    pub async fn observe_created_volumes(
        &self,
        names: Vec<String>,
    ) -> Result<Vec<Option<DockerVolume>>, DockerError> {
        let generation = self.daemon_generation();
        let mut metadata = stream::iter(names)
            .map(|name| async move {
                self.volume_metadata(&name)
                    .await
                    .ok()
                    .filter(|volume| volume.name == name)
            })
            .buffered(4)
            .collect::<Vec<_>>()
            .await;
        if metadata.iter().all(Option::is_none) {
            return Ok(metadata);
        }
        let containers = self
            .list_container_models(Some(true), None, Some(false), None)
            .await?;
        self.fill_volume_usage(metadata.iter_mut().flatten(), true)
            .await?;
        let policy = super::volume_content_policy::StorageProtection::from_containers(&containers);
        for volume in metadata.iter_mut().flatten() {
            policy.apply(volume);
        }
        self.check_volume_generation(generation)?;
        Ok(metadata)
    }

    fn check_volume_generation(&self, generation: u64) -> Result<(), DockerError> {
        if self.daemon_generation() != generation {
            return Err(DockerError::ProtocolIo(std::io::Error::other(
                "Docker daemon changed during volume observation",
            )));
        }
        Ok(())
    }
}
