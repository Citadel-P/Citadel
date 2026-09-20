#![forbid(unsafe_code)]
pub mod model;
pub use model::{
    COMMUNITY_EDITION, CURRENT_LICENSE_SCHEMA, CitadelInstanceIdentity, InstalledLicense,
    LEGACY_BUSINESS_EDITION, LEGACY_LICENSE_SCHEMA, LICENSE_AUDIENCE, LICENSE_ISSUER,
    LICENSE_PRODUCT, LicenseCapability, LicenseCustomer, LicensePayload, LicenseState,
    LicenseStatus, LicenseVerificationResult, TEAM_EDITION, VerifiedLicense,
};
