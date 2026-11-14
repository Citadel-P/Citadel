//! Short-lived, bounded reuse of successful Agent digest verification only.
use super::GithubReleaseArtifacts;
use citadel_execution::{ProcessError, ProcessLimits, ProcessRequest};
use citadel_updates::{RELEASE_OIDC_ISSUER, UpdateError, release_signing_identity};
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

pub(super) const AGENT_VERIFICATION_TTL: Duration = Duration::from_secs(15 * 60);

pub(super) struct VerifiedAgentImage {
    version: String,
    image: String,
    expires: Instant,
}

impl GithubReleaseArtifacts {
    pub(super) async fn verify_agent_image(
        &self,
        version: &str,
        image: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        // One slot bounds retained data and one lock coalesces concurrent checks.
        // Metadata/checksum validation runs before this method on every request.
        let mut cached = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
            guard = self.verified_agent.lock() => guard,
        };
        if cached.as_ref().is_some_and(|entry| {
            entry.version == version && entry.image == image && Instant::now() < entry.expires
        }) {
            return Ok(());
        }
        *cached = None;
        // Await the runner through cancellation cleanup, as for uncached verification.
        self.verify_image(version, image, cancellation).await?;
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        *cached = Some(VerifiedAgentImage {
            version: version.into(),
            image: image.into(),
            expires: Instant::now() + AGENT_VERIFICATION_TTL,
        });
        Ok(())
    }

    pub(super) async fn verify_image(
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
        let home = temporary.tempdir().map_err(|_| signature_failed())?;
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

fn signature_failed() -> UpdateError {
    UpdateError::Unavailable(
        "The release image signature could not be verified against Citadel's release workflow."
            .into(),
    )
}
