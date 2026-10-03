#![forbid(unsafe_code)]
pub mod model;
pub use model::{
    COMMUNITY_EDITION, CURRENT_LICENSE_SCHEMA, CitadelInstanceIdentity, InstalledLicense,
    LEGACY_BUSINESS_EDITION, LEGACY_LICENSE_SCHEMA, LICENSE_AUDIENCE, LICENSE_ISSUER,
    LICENSE_PRODUCT, LicenseCapability, LicenseCustomer, LicensePayload, LicenseState,
    LicenseStatus, LicenseVerificationResult, TEAM_EDITION, VerifiedLicense,
};

mod changes;
mod error;
mod jobs;
mod read_models;
mod repository;
mod runtime;
mod service;
mod state;
pub use changes::LicenseChange;
pub use error::LicenseError;
pub use jobs::{
    LICENSE_TRANSITION_BOUNDARY_MARGIN, LICENSE_TRANSITION_MAXIMUM_CHECK_INTERVAL,
    LicenseTransitionMonitor, license_transition_delay, next_license_boundary,
};
pub use read_models::{LicenseRequest, LicenseSource, LicenseTransitionCheck};
pub use repository::{LicenseStore, LicenseValidationPersistence};
pub use runtime::{LicenseClock, LicenseStateNotifier, LicenseVerifier};
pub use service::LicenseService;
pub use state::build_state;
