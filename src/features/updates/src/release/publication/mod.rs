mod manifest;
pub use manifest::{
    PublicationIdentity, PublicationSignaturePolicy, PublishedImage, PublishedReleaseManifest,
    RELEASE_OIDC_ISSUER, VerifiedReleaseArtifact, release_signing_identity,
};

#[cfg(test)]
mod tests;
