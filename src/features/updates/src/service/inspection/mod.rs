use crate::{
    CoreIdentity, InspectedRelease, ReleaseArtifactSource, ReleaseInspectionRuntime, UpdateError,
    official_image_digest, release_version,
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// No review, claim or installation is created by target inspection.
pub struct ReleaseInspectionService {
    artifacts: Arc<dyn ReleaseArtifactSource>,
    runtime: Arc<dyn ReleaseInspectionRuntime>,
}

impl ReleaseInspectionService {
    pub fn new(
        artifacts: Arc<dyn ReleaseArtifactSource>,
        runtime: Arc<dyn ReleaseInspectionRuntime>,
    ) -> Self {
        Self { artifacts, runtime }
    }

    pub async fn inspect(
        &self,
        version: &str,
        architecture: &str,
        cancellation: &CancellationToken,
    ) -> Result<InspectedRelease, UpdateError> {
        release_version(version)?;
        if !matches!(architecture, "amd64" | "arm64") {
            return Err(UpdateError::Validation(
                "Unsupported target architecture.".into(),
            ));
        }
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        // Do not race/drop either future: the ports own subprocess/container cleanup.
        let artifact = self.artifacts.verify(version, cancellation).await?;
        if artifact.version != version || !official_image_digest(&artifact.image) {
            return Err(UpdateError::Unavailable(
                "Verified release identity does not match the requested target.".into(),
            ));
        }
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let descriptor = self
            .runtime
            .inspect(&artifact, architecture, cancellation)
            .await?;
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let identity = CoreIdentity {
            version: artifact.version,
            image: artifact.image,
            architecture: architecture.into(),
            catalog_sha256: descriptor.target_catalog_sha256.clone(),
        };
        descriptor.validate_target(&identity)?;
        Ok(InspectedRelease {
            identity,
            descriptor,
        })
    }
}

#[cfg(test)]
mod tests;
