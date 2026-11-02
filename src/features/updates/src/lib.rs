//! Instance release checks and durable update/recovery contracts.
#![forbid(unsafe_code)]
mod error;
mod model;
mod read_models;
mod release;
mod repository;
mod runtime;
mod service;

pub use error::UpdateError;
pub use model::{
    CompletionReceipt, CoreIdentity, InstallationKind, UPDATE_PROTOCOL, UpdateJournal,
    UpdateOperation, UpdatePhase, UpdatePlan,
};
pub use read_models::{CheckState, ReleaseCheckStatus};
pub use release::{
    CORE_IMAGE, INSTALLER_FORMAT, OFFICIAL_REPOSITORY, PublicationIdentity,
    PublicationSignaturePolicy, PublishedImage, PublishedReleaseManifest, RELEASE_OIDC_ISSUER,
    Release, ReleaseChannel, ReleaseNotice, UpgradeDescriptor, UpgradeSource,
    VerifiedReleaseArtifact, available_update, development_update, is_development, is_sha256,
    official_image_digest, release_signing_identity, release_version,
};
pub use repository::{ReleaseCheckRepository, UpdateActor, UpdateRepository};
pub use runtime::{ReleaseArtifactSource, ReleaseSource};
pub use service::{
    NoopUpdateChangeNotifier, ReleaseCheckPolicy, ReleaseCheckService, UpdateChangeNotifier,
};
