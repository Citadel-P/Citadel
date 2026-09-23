use crate::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;
#[derive(Debug, Clone)]
pub struct LicenseSource {
    pub identity: CitadelInstanceIdentity,
    pub installed: Option<InstalledLicense>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LicenseTransitionCheck {
    pub persistence: LicenseValidationPersistence,
    pub instance_id: Option<Uuid>,
    pub status: Option<LicenseStatus>,
    pub next_boundary: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseRequest {
    pub product: &'static str,
    pub instance_id: Uuid,
    pub core_version: String,
    pub generated_at: DateTime<Utc>,
}
