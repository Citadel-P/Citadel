//! Optional public release checks. Only the small notification is retained.
use std::{sync::Arc, time::Duration};

use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const RELEASE_URL: &str = "https://api.github.com/repos/Citadel-P/Citadel/releases/latest";
const DEVELOPMENT_RELEASES_URL: &str =
    "https://api.github.com/repos/Citadel-P/Citadel/releases?per_page=100";
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_RELEASE_LIST_BYTES: usize = 8 * 1024 * 1024;

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

    /// Observe committed installation settings; no request runs before setup.
    pub async fn run(
        self,
        identity: Arc<citadel_identity::IdentityService>,
        cancellation: CancellationToken,
    ) -> Result<(), std::convert::Infallible> {
        let (enabled, receiver) = tokio::sync::watch::channel(false);
        let policy = watch_policy(identity, enabled, cancellation.clone());
        let current = env!("CITADEL_BUILD_VERSION");
        tokio::select! {
            biased;
            () = policy => Ok(()),
            result = self.run_enabled(receiver, cancellation, release_endpoint(current), current) => result,
        }
    }

    async fn run_enabled(
        self,
        mut enabled: tokio::sync::watch::Receiver<bool>,
        cancellation: CancellationToken,
        endpoint: &str,
        current: &str,
    ) -> Result<(), std::convert::Infallible> {
        loop {
            if !*enabled.borrow_and_update() {
                *self.0.write().await = None;
                tokio::select! {
                    biased;
                    () = cancellation.cancelled() => return Ok(()),
                    result = enabled.changed() => if result.is_err() { return Ok(()); },
                }
                continue;
            }
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                result = enabled.changed() => {
                    if result.is_err() { return Ok(()); }
                    continue;
                },
                () = self.check(endpoint, current) => {},
            }
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                result = enabled.changed() => if result.is_err() { return Ok(()); },
                () = tokio::time::sleep(CHECK_INTERVAL) => {},
            }
        }
    }

    async fn check(&self, endpoint: &str, current: &str) {
        let result = async {
            if is_development(&Version::parse(current.trim_start_matches('v'))?) {
                let releases = fetch_release(endpoint, MAX_RELEASE_LIST_BYTES).await?;
                development_update(current, releases)
            } else {
                let release = fetch_release(endpoint, MAX_RESPONSE_BYTES).await?;
                available_update(current, release)
            }
        }
        .await;
        match result {
            Ok(update) => *self.0.write().await = update,
            // Keep the last successful result; an unavailable endpoint does not
            // mean that the installation is up to date.
            Err(error) => tracing::debug!(%error, "Citadel update check unavailable"),
        }
    }
}

async fn watch_policy(
    identity: Arc<citadel_identity::IdentityService>,
    enabled: tokio::sync::watch::Sender<bool>,
    cancellation: CancellationToken,
) {
    let mut changes = identity.installation_changes();
    let mut poll = tokio::time::interval(Duration::from_secs(30));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    poll.reset();
    loop {
        let settings = tokio::select! {
            biased;
            () = cancellation.cancelled() => return,
            result = identity.installation_settings() => result,
        };
        let next = match settings {
            Ok(settings) => settings.initialized && settings.update_checks_enabled,
            Err(error) => {
                tracing::debug!(%error, "Citadel update-check configuration unavailable");
                false
            }
        };
        enabled.send_if_modified(|value| {
            if *value == next {
                return false;
            }
            *value = next;
            true
        });
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return,
            result = changes.changed() => if result.is_err() { return; },
            _ = poll.tick() => {},
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

fn is_development(version: &Version) -> bool {
    version.pre.as_str().starts_with("dev.")
}

fn release_endpoint(current: &str) -> &'static str {
    if Version::parse(current.trim_start_matches('v')).is_ok_and(|v| is_development(&v)) {
        DEVELOPMENT_RELEASES_URL
    } else {
        RELEASE_URL
    }
}

async fn fetch_release<T: serde::de::DeserializeOwned>(
    endpoint: &str,
    maximum_bytes: usize,
) -> Result<T, CheckError> {
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
        if chunk.len() > maximum_bytes.saturating_sub(body.len()) {
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
    let same_channel = if is_development(&current) {
        release.prerelease && is_development(&latest)
    } else {
        !release.prerelease && latest.pre.is_empty()
    };
    if release.draft || !same_channel || !latest.cmp_precedence(&current).is_gt() {
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

fn development_update(
    current: &str,
    releases: Vec<Release>,
) -> Result<Option<AvailableUpdate>, CheckError> {
    Version::parse(current.trim_start_matches('v'))?;
    // GitHub orders by creation time, which need not match version precedence.
    // Ignore unrelated/non-version tags without hiding a valid newer release.
    Ok(releases
        .into_iter()
        .filter_map(|release| available_update(current, release).ok().flatten())
        .filter_map(|update| Version::parse(&update.version).ok().map(|v| (v, update)))
        .max_by(|(a, _), (b, _)| a.cmp_precedence(b))
        .map(|(_, update)| update))
}

#[cfg(test)]
mod tests;
