mod channel;
mod descriptor;
mod publication;
mod selection;

pub use channel::ReleaseChannel;
pub use descriptor::{
    CORE_IMAGE, INSTALLER_FORMAT, OFFICIAL_REPOSITORY, ReleaseNotice, UpgradeDescriptor,
    UpgradeSource, is_sha256, official_image_digest, release_version,
};
pub use publication::{
    PublicationIdentity, PublicationSignaturePolicy, PublishedImage, PublishedReleaseManifest,
    RELEASE_OIDC_ISSUER, VerifiedReleaseArtifact, release_signing_identity,
};
pub use selection::{Release, available_update, development_update, is_development};

#[cfg(test)]
mod tests;
