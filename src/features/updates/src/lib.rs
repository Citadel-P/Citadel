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
    CompletionReceipt, CoreIdentity, InstallationKind, InstallationSnapshot, UPDATE_PROTOCOL,
    UpdateJournal, UpdateOperation, UpdatePhase, UpdatePlan,
};
pub use read_models::{
    CheckState, InspectedRelease, ReleaseCheckStatus, UpdateDatabaseState, UpdateDrainBlocker,
    UpdateDrainOutcome,
};
pub use release::{
    CORE_IMAGE, INSTALLER_FORMAT, OFFICIAL_REPOSITORY, PublicationIdentity,
    PublicationSignaturePolicy, PublishedImage, PublishedReleaseManifest, RELEASE_OIDC_ISSUER,
    Release, ReleaseChannel, ReleaseNotice, UpgradeDescriptor, UpgradeSource,
    VerifiedReleaseArtifact, available_update, development_update, is_development, is_sha256,
    official_image_digest, release_signing_identity, release_version,
};
pub use repository::{
    ReleaseCheckRepository, UpdateActor, UpdateDrainRepository, UpdateRepository,
};
pub use runtime::{
    ReleaseArtifactSource, ReleaseInspectionRuntime, ReleaseSource, UpdateDrainRuntime,
    UpdateHandoffRuntime, UpdateInstallationRuntime,
};
pub use service::{
    NoopUpdateChangeNotifier, ReleaseCheckPolicy, ReleaseCheckService, ReleaseInspectionService,
    UpdateChangeNotifier, UpdateDrainService, UpdateInstallCommand, UpdateInstallService,
};

pub use model::{
    AgentUpdateEligibility, AgentUpdateKind, AgentUpdateOperation, AgentUpdatePhase,
    AgentUpdateRelease, AgentUpdateResult, AgentUpdateReview, AgentUpdateState, AgentUpdateTarget,
    UpdateSchedule,
};

pub use model::{UpdateSchedulePhase, UpdateScheduleRun};
pub use repository::AgentUpdateRepository;
pub use runtime::{AgentUpdateEntitlements, AgentUpdateRuntime};
pub use service::AgentUpdateService;

mod availability;
pub use availability::agent_availability;
