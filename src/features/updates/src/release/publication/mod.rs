use super::{CORE_IMAGE, OFFICIAL_REPOSITORY, is_sha256, release_version};
use crate::UpdateError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const RELEASE_OIDC_ISSUER: &str = "https://token.actions.githubusercontent.com";

pub fn release_signing_identity(version: &str) -> Result<String, UpdateError> {
    release_version(version)?;
    Ok(format!(
        "https://github.com/{OFFICIAL_REPOSITORY}/.github/workflows/ci.yml@refs/tags/v{version}"
    ))
}

/// Publication metadata is untrusted until the selected image's signature has
/// been verified. Neither checksums nor publisher receipts establish trust.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedReleaseManifest {
    pub schema_version: u32,
    pub tag: String,
    pub metadata: PublicationIdentity,
    pub architectures: Vec<String>,
    pub images: BTreeMap<String, Vec<PublishedImage>>,
    pub signatures: PublicationSignaturePolicy,
    pub verified_signatures: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationIdentity {
    pub display_version: String,
    pub source_revision: String,
    pub dirty: bool,
    pub identity_status: String,
    #[serde(default)]
    pub release_eligible: bool,
    #[serde(default)]
    pub development_release_eligible: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedImage {
    pub tag: String,
    pub digest: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationSignaturePolicy {
    pub tool: String,
    pub certificate_identity: String,
    pub certificate_oidc_issuer: String,
}

impl PublishedReleaseManifest {
    /// Resolve only official publication identities. The returned reference is
    /// a candidate for cryptographic verification, not installation approval.
    pub fn core_image(&self, version: &str) -> Result<String, UpdateError> {
        let parsed = release_version(version)?;
        let stable = parsed.pre.is_empty();
        let identity = release_signing_identity(version)?;
        if self.schema_version != 1
            || self.tag != format!("v{version}")
            || self.metadata.display_version != version
            || self.metadata.dirty
            || self.metadata.identity_status != "resolved"
            || self.metadata.source_revision.len() != 40
            || !self
                .metadata
                .source_revision
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.metadata.release_eligible != stable
            || self.metadata.development_release_eligible == stable
            || self.architectures != ["amd64", "arm64"]
            || self.signatures.tool != "cosign"
            || self.signatures.certificate_identity != identity
            || self.signatures.certificate_oidc_issuer != RELEASE_OIDC_ISSUER
        {
            return Err(invalid_publication());
        }
        let core = self.images.get("core").ok_or_else(invalid_publication)?;
        let repositories = if stable {
            vec![CORE_IMAGE, "docker.io/citadelplane/citadel"]
        } else {
            vec![CORE_IMAGE]
        };
        if core.len() != repositories.len() {
            return Err(invalid_publication());
        }
        let mut expected_digest = None;
        for (image, repository) in core.iter().zip(repositories) {
            let digest = image
                .digest
                .strip_prefix(&format!("{repository}@sha256:"))
                .filter(|value| is_sha256(value))
                .ok_or_else(invalid_publication)?;
            if image.tag != format!("{repository}:{version}")
                || self.verified_signatures.get(&image.tag) != Some(&format!("sha256:{digest}"))
                || expected_digest.is_some_and(|expected| expected != digest)
            {
                return Err(invalid_publication());
            }
            expected_digest = Some(digest);
        }
        Ok(core[0].digest.clone())
    }
}

fn invalid_publication() -> UpdateError {
    UpdateError::Unavailable(
        "The release manifest does not match Citadel's official publication policy.".into(),
    )
}

/// A signed Core artifact, not a verified source-to-target upgrade path.
/// Catalog, installation and Agent compatibility must still be established.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedReleaseArtifact {
    pub version: String,
    pub image: String,
}

#[cfg(test)]
mod tests;
