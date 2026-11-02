use crate::{CoreIdentity, UPDATE_PROTOCOL, UpdateError};
use semver::Version;
use serde::{Deserialize, Serialize};

pub const OFFICIAL_REPOSITORY: &str = "Citadel-P/Citadel";
pub const CORE_IMAGE: &str = "ghcr.io/citadel-p/citadel";
pub const INSTALLER_FORMAT: u32 = 1;

/// Embedded in the signed image. A matching copy in a release attachment alone
/// is not proof of authenticity; adapters must verify the image before reading it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpgradeDescriptor {
    pub schema_version: u32,
    pub product: String,
    pub target_version: String,
    pub target_catalog_sha256: String,
    pub installer_format: u32,
    pub update_protocol: u32,
    pub sources: Vec<UpgradeSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpgradeSource {
    pub version: String,
    pub catalog_sha256: String,
    pub architectures: Vec<String>,
    /// Explicitly qualified transition versions; empty means Core-only.
    pub agent_versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseNotice {
    pub version: String,
    pub release_url: String,
}

pub fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn official_image_digest(value: &str) -> bool {
    value
        .strip_prefix(&format!("{CORE_IMAGE}@sha256:"))
        .is_some_and(is_sha256)
}

pub fn release_version(value: &str) -> Result<Version, UpdateError> {
    let version = Version::parse(value)
        .map_err(|_| UpdateError::Validation("Invalid release version.".into()))?;
    if !version.build.is_empty()
        || (!version.pre.is_empty()
            && !version
                .pre
                .as_str()
                .strip_prefix("dev.")
                .is_some_and(|n| n.parse::<u64>().is_ok_and(|n| n > 0)))
    {
        return Err(UpdateError::Validation(
            "Only published stable and numbered development releases are supported.".into(),
        ));
    }
    Ok(version)
}

impl UpgradeDescriptor {
    pub fn validate_transition(
        &self,
        source: &CoreIdentity,
        target: &CoreIdentity,
        agents: &[String],
    ) -> Result<(), UpdateError> {
        let from = release_version(&source.version)?;
        let to = release_version(&target.version)?;
        if self.schema_version != 1
            || self.product != "citadel"
            || self.update_protocol != UPDATE_PROTOCOL
            || self.installer_format != INSTALLER_FORMAT
            || self.target_version != target.version
            || self.target_catalog_sha256 != target.catalog_sha256
            || !is_sha256(&source.catalog_sha256)
            || !is_sha256(&target.catalog_sha256)
            || !official_image_digest(&source.image)
            || !official_image_digest(&target.image)
            || source.architecture != target.architecture
            || !matches!(target.architecture.as_str(), "amd64" | "arm64")
            || from >= to
            || from.pre.is_empty() != to.pre.is_empty()
            || self.sources.len() > 64
        {
            return Err(UpdateError::Unavailable(
                "This release does not support the selected installation transition.".into(),
            ));
        }
        let candidates = self
            .sources
            .iter()
            .filter(|s| {
                s.version == source.version
                    && s.catalog_sha256 == source.catalog_sha256
                    && s.architectures.contains(&source.architecture)
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(UpdateError::Unavailable(
                "The installed release and database catalog have no verified upgrade path.".into(),
            ));
        }
        if agents
            .iter()
            .any(|v| !candidates[0].agent_versions.contains(v))
        {
            return Err(UpdateError::Unavailable(
                "An Agent requires a coordinated upgrade that this release does not support."
                    .into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

mod selection;
pub use selection::{Release, available_update, development_update, is_development};

mod publication;
pub use publication::{
    PublicationIdentity, PublicationSignaturePolicy, PublishedImage, PublishedReleaseManifest,
    RELEASE_OIDC_ISSUER, VerifiedReleaseArtifact, release_signing_identity,
};
mod channel;
pub use channel::ReleaseChannel;
