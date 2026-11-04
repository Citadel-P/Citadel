use citadel_execution::{ProcessError, ProcessLimits, ProcessRequest, ProcessRunner};
use citadel_updates::{
    OFFICIAL_REPOSITORY, PublishedReleaseManifest, RELEASE_OIDC_ISSUER, ReleaseArtifactSource,
    UpdateError, VerifiedReleaseArtifact, release_signing_identity, release_version,
};
use futures_util::future::BoxFuture;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

const MAX_MANIFEST: usize = 1024 * 1024;

pub struct GithubReleaseArtifacts {
    client: reqwest::Client,
    process: Arc<dyn ProcessRunner>,
    api_root: String,
    download_root: String,
}

impl GithubReleaseArtifacts {
    pub fn new(process: Arc<dyn ProcessRunner>) -> Result<Self, UpdateError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Citadel-Update-Check")
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() < 3 && permitted_redirect(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|_| unavailable())?;
        Ok(Self {
            client,
            process,
            api_root: format!("https://api.github.com/repos/{OFFICIAL_REPOSITORY}"),
            download_root: format!("https://github.com/{OFFICIAL_REPOSITORY}/releases/download"),
        })
    }

    async fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, UpdateError> {
        let mut response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_| unavailable())?;
        if !response.status().is_success() {
            return Err(unavailable());
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if chunk.len() > limit.saturating_sub(body.len()) {
                return Err(unavailable());
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    async fn published_image(&self, version: &str, agent: bool) -> Result<String, UpdateError> {
        let parsed = release_version(version)?;
        let publication: Publication = serde_json::from_slice(
            &self
                .bytes(
                    &format!("{}/releases/tags/v{version}", self.api_root),
                    MAX_MANIFEST,
                )
                .await?,
        )
        .map_err(|_| unavailable())?;
        if publication.draft
            || publication.tag_name != format!("v{version}")
            || publication.prerelease == parsed.pre.is_empty()
            || ["release-manifest.json", "SHA256SUMS"].iter().any(|name| {
                publication
                    .assets
                    .iter()
                    .filter(|asset| asset.name == *name)
                    .count()
                    != 1
            })
        {
            return Err(unavailable());
        }
        // Paths are derived from the requested canonical version and official
        // repository, never asset URLs supplied by downloaded metadata.
        let root = format!("{}/v{version}", self.download_root);
        let manifest = self
            .bytes(&format!("{root}/release-manifest.json"), MAX_MANIFEST)
            .await?;
        let checksum = self.bytes(&format!("{root}/SHA256SUMS"), 256).await?;
        let digest: String = Sha256::digest(&manifest)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let expected = format!("{digest}  release-manifest.json");
        if std::str::from_utf8(&checksum).map(str::trim).ok() != Some(expected.as_str()) {
            return Err(unavailable());
        }
        let manifest = serde_json::from_slice::<PublishedReleaseManifest>(&manifest)
            .map_err(|_| unavailable())?;
        if agent {
            manifest.agent_image(version)
        } else {
            manifest.core_image(version)
        }
    }

    pub async fn agent(
        &self,
        version: &str,
        cancellation: &CancellationToken,
    ) -> Result<citadel_updates::AgentUpdateRelease, UpdateError> {
        let image = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
            image = self.published_image(version, true) => image?,
        };
        self.verify_image(version, &image, cancellation).await?;
        Ok(citadel_updates::AgentUpdateRelease {
            version: version.into(),
            image,
        })
    }

    async fn verify_image(
        &self,
        version: &str,
        image: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        // Isolate Sigstore's trust cache and Docker credential discovery from
        // Core's HOME. The bounded runner clears inherited tool credentials.
        let mut temporary = tempfile::Builder::new();
        temporary.prefix("citadel-release-verification-");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temporary.permissions(std::fs::Permissions::from_mode(0o700));
        }
        let home = temporary.tempdir().map_err(|_| unavailable())?;
        let identity = release_signing_identity(version)?;
        let request = ProcessRequest::new("cosign")
            .args([
                "verify",
                "--certificate-identity",
                &identity,
                "--certificate-oidc-issuer",
                RELEASE_OIDC_ISSUER,
                "--check-claims=true",
                "--output=json",
                image,
            ])
            .env("HOME", home.path())
            .limits(ProcessLimits {
                timeout: Duration::from_secs(90),
                maximum_stdout_bytes: 256 * 1024,
                maximum_stderr_bytes: 16 * 1024,
                ..ProcessLimits::default()
            });
        let output =
            self.process
                .run(request, cancellation)
                .await
                .map_err(|error| match error {
                    ProcessError::Cancelled => UpdateError::Cancelled,
                    _ => signature_failed(),
                })?;
        if !output.succeeded() || output.stdout_truncated || output.stderr_truncated {
            return Err(signature_failed());
        }
        let claims: Vec<serde_json::Value> =
            serde_json::from_slice(&output.stdout).map_err(|_| signature_failed())?;
        let digest = image
            .split_once('@')
            .map(|(_, digest)| digest)
            .ok_or_else(signature_failed)?;
        if !claims.iter().any(|claim| {
            claim["critical"]["image"]["docker-manifest-digest"].as_str() == Some(digest)
        }) {
            return Err(signature_failed());
        }
        Ok(())
    }
}

impl ReleaseArtifactSource for GithubReleaseArtifacts {
    fn verify<'a>(
        &'a self,
        version: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<VerifiedReleaseArtifact, UpdateError>> {
        Box::pin(async move {
            release_version(version)?;
            let image = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
                image = self.published_image(version, false) => image?,
            };
            if cancellation.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            // Await the runner's cancellation cleanup; never detach a verifier.
            self.verify_image(version, &image, cancellation).await?;
            Ok(VerifiedReleaseArtifact {
                version: version.into(),
                image,
            })
        })
    }
}

fn permitted_redirect(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some("github.com" | "release-assets.githubusercontent.com")
        )
}

#[derive(Deserialize)]
struct Publication {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
}

fn unavailable() -> UpdateError {
    UpdateError::Unavailable(
        "The completed official release manifest could not be verified.".into(),
    )
}
fn signature_failed() -> UpdateError {
    UpdateError::Unavailable(
        "The release image signature could not be verified against Citadel's release workflow."
            .into(),
    )
}

#[cfg(test)]
mod tests;
