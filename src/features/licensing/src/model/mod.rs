#![forbid(unsafe_code)]

pub mod resource;
pub use resource::{
    COMMUNITY_EDITION, CURRENT_LICENSE_SCHEMA, CitadelInstanceIdentity, InstalledLicense,
    LEGACY_BUSINESS_EDITION, LEGACY_LICENSE_SCHEMA, LICENSE_AUDIENCE, LICENSE_ISSUER,
    LICENSE_PRODUCT, LicenseCustomer, LicensePayload, LicenseState, LicenseVerificationResult,
    TEAM_EDITION, VerifiedLicense,
};
pub mod vocabulary;
pub use vocabulary::{LicenseCapability, LicenseStatus};
