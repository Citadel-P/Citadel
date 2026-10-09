//! Optional public release checks. Only the small notification is retained.
use std::{sync::Arc, time::Duration};

use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const RELEASE_URL: &str = "https://api.github.com/repos/Citadel-P/Citadel/releases/latest";
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const MAX_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpdate {
    pub version: String,
    pub release_url: String,
}

#[derive(Clone, Default)]
pub struct UpdateChecker(Arc<RwLock<Option<AvailableUpdate>>>);

impl UpdateChecker {
    pub async fn available(&self) -> Option<AvailableUpdate> {
        self.0.read().await.clone()
    }

    /// Disabled checks do not even construct an HTTP client.
    pub async fn run(
        self,
        enabled: bool,
        cancellation: CancellationToken,
    ) -> Result<(), std::convert::Infallible> {
        if !enabled {
            cancellation.cancelled().await;
            return Ok(());
        }
        loop {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                () = self.check(RELEASE_URL, env!("CITADEL_BUILD_VERSION")) => {}
            }
            tokio::select! {
                () = cancellation.cancelled() => return Ok(()),
                () = tokio::time::sleep(CHECK_INTERVAL) => {}
            }
        }
    }

    async fn check(&self, endpoint: &str, current: &str) {
        match fetch_release(endpoint)
            .await
            .and_then(|release| available_update(current, release))
        {
            Ok(update) => *self.0.write().await = update,
            // Keep the last successful result; an unavailable endpoint does not
            // mean that the installation is up to date.
            Err(error) => tracing::debug!(%error, "Citadel update check unavailable"),
        }
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

type CheckError = Box<dyn std::error::Error + Send + Sync>;

async fn fetch_release(endpoint: &str) -> Result<Release, CheckError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        // No installed version, instance identifier, credentials or telemetry.
        .user_agent("Citadel-Update-Check")
        .build()?;
    let mut response = client
        .get(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .send()
        .await?
        .error_for_status()?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > MAX_RESPONSE_BYTES.saturating_sub(body.len()) {
            return Err("release response exceeds size limit".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&body)?)
}

fn available_update(
    current: &str,
    release: Release,
) -> Result<Option<AvailableUpdate>, CheckError> {
    let current = Version::parse(current.trim_start_matches('v'))?;
    let latest = Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name),
    )?;
    if release.draft
        || release.prerelease
        || !latest.pre.is_empty()
        || !latest.cmp_precedence(&current).is_gt()
    {
        return Ok(None);
    }
    Ok(Some(AvailableUpdate {
        version: latest.to_string(),
        // Never trust URLs supplied by release metadata.
        release_url: format!(
            "https://github.com/Citadel-P/Citadel/releases/tag/{}",
            release.tag_name
        ),
    }))
}

#[cfg(test)]
mod tests;
