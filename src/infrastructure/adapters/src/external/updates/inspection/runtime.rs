//! Coordinate verified image inspection and await probe cleanup.
use super::docker::{Docker, failed};
use super::probe::{OWNER, Probe};
use citadel_execution::ProcessRunner;
use citadel_updates::{
    ReleaseInspectionRuntime, UpdateError, UpgradeDescriptor, VerifiedReleaseArtifact, is_sha256,
    official_image_digest, release_version,
};
use futures_util::future::BoxFuture;
use serde_json::Value;
use std::{os::unix::fs::PermissionsExt, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct DockerReleaseInspection {
    process: Arc<dyn ProcessRunner>,
}

impl DockerReleaseInspection {
    pub fn new(process: Arc<dyn ProcessRunner>) -> Self {
        Self { process }
    }
}

impl ReleaseInspectionRuntime for DockerReleaseInspection {
    fn inspect<'a>(
        &'a self,
        artifact: &'a VerifiedReleaseArtifact,
        architecture: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<UpgradeDescriptor, UpdateError>> {
        Box::pin(async move {
            release_version(&artifact.version)?;
            if !official_image_digest(&artifact.image) || !matches!(architecture, "amd64" | "arm64")
            {
                return Err(failed());
            }
            let home = tempfile::Builder::new()
                .prefix("citadel-update-inspection-")
                .permissions(std::fs::Permissions::from_mode(0o700))
                .tempdir()
                .map_err(|_| failed())?;
            let docker = Docker {
                process: self.process.as_ref(),
                home: home.path(),
            };
            let platform = format!("linux/{architecture}");
            docker
                .run(
                    &["pull", "--platform", &platform, &artifact.image],
                    Duration::from_secs(300),
                    cancellation,
                )
                .await?;
            let image: Value = serde_json::from_slice(
                &docker
                    .run(
                        &[
                            "image",
                            "inspect",
                            "--platform",
                            &platform,
                            "--format",
                            "{{json .}}",
                            &artifact.image,
                        ],
                        Duration::from_secs(15),
                        cancellation,
                    )
                    .await?,
            )
            .map_err(|_| failed())?;
            image["Id"]
                .as_str()
                .filter(|id| image_id_valid(id))
                .ok_or_else(failed)?;
            if image["Os"] != "linux"
                || image["Architecture"] != architecture
                || !image["RepoDigests"].as_array().is_some_and(|digests| {
                    digests.iter().any(|v| v.as_str() == Some(&artifact.image))
                })
                || !(image["Config"]["Volumes"].is_null()
                    || image["Config"]["Volumes"]
                        .as_object()
                        .is_some_and(|v| v.is_empty()))
            {
                return Err(failed());
            }
            // With the containerd image store, platform inspection can return a
            // child manifest ID that is not itself a locally addressable image.
            // Resolve the local identity separately; only the platform-specific
            // inspection above supplies architecture and configuration evidence.
            let local_id = docker
                .run(
                    &["image", "inspect", "--format", "{{.Id}}", &artifact.image],
                    Duration::from_secs(15),
                    cancellation,
                )
                .await?;
            let image_id = std::str::from_utf8(&local_id)
                .map(str::trim)
                .ok()
                .filter(|id| image_id_valid(id))
                .ok_or_else(failed)?;
            if cancellation.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            let nonce = Uuid::now_v7().to_string();
            let name = format!("citadel-update-inspection-{nonce}");
            let label = format!("{OWNER}={nonce}");
            let probe = Probe {
                docker: &docker,
                name: &name,
                nonce: &nonce,
                label: &label,
                image: image_id,
            };
            let result = probe.read(&platform, cancellation).await;
            // A cancelled docker client does not stop its daemon-side container.
            // Cleanup is bounded and awaited with an independent token, including
            // after an ambiguous create. Never report success if cleanup fails.
            if probe.cleanup().await.is_err() {
                return Err(UpdateError::Unavailable(format!(
                    "Could not verify cleanup of inspection container {name}. Inspect it on the local Docker daemon before retrying."
                )));
            }
            result
        })
    }
}

fn image_id_valid(id: &str) -> bool {
    id.strip_prefix("sha256:").is_some_and(is_sha256)
}
