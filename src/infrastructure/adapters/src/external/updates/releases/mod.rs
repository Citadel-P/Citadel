use citadel_updates::{Release, ReleaseChannel, ReleaseSource, UpdateError};
use futures_util::future::BoxFuture;
use std::time::Duration;

async fn fetch_release<T: serde::de::DeserializeOwned>(
    endpoint: &str,
    maximum_bytes: usize,
) -> Result<T, UpdateError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        // No installed version, instance identifier, credentials or telemetry.
        .user_agent("Citadel-Update-Check")
        .build()
        .map_err(unavailable)?;
    let mut response = client
        .get(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .send()
        .await
        .map_err(unavailable)?
        .error_for_status()
        .map_err(unavailable)?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(unavailable)? {
        if chunk.len() > maximum_bytes.saturating_sub(body.len()) {
            return Err(unavailable("release response exceeds size limit"));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&body).map_err(unavailable)?)
}

const RELEASE_URL: &str = "https://api.github.com/repos/Citadel-P/Citadel/releases/latest";
const DEVELOPMENT_RELEASES_URL: &str =
    "https://api.github.com/repos/Citadel-P/Citadel/releases?per_page=100";
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_RELEASE_LIST_BYTES: usize = 8 * 1024 * 1024;

pub struct GithubReleaseSource;

impl ReleaseSource for GithubReleaseSource {
    fn releases(
        &self,
        channel: ReleaseChannel,
    ) -> BoxFuture<'_, Result<Vec<Release>, UpdateError>> {
        Box::pin(read_releases(
            channel,
            match channel {
                ReleaseChannel::Stable => RELEASE_URL,
                ReleaseChannel::Development => DEVELOPMENT_RELEASES_URL,
            },
        ))
    }
}

async fn read_releases(
    channel: ReleaseChannel,
    endpoint: &str,
) -> Result<Vec<Release>, UpdateError> {
    match channel {
        ReleaseChannel::Stable => fetch_release::<GithubRelease>(endpoint, MAX_RESPONSE_BYTES)
            .await
            .map(|release| vec![release.into()]),
        ReleaseChannel::Development => {
            fetch_release::<Vec<GithubRelease>>(endpoint, MAX_RELEASE_LIST_BYTES)
                .await
                .map(|releases| releases.into_iter().map(Into::into).collect())
        }
    }
}

#[cfg(test)]
mod tests;

// GitHub wire fields remain at the HTTP boundary.
#[derive(serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}
impl From<GithubRelease> for Release {
    fn from(release: GithubRelease) -> Self {
        Self {
            tag_name: release.tag_name,
            draft: release.draft,
            prerelease: release.prerelease,
        }
    }
}
fn unavailable(_: impl std::fmt::Display) -> UpdateError {
    UpdateError::Unavailable("The official release source could not be read.".into())
}
