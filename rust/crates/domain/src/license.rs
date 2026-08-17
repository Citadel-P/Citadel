use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{ActorId, LicenseCapability, LicenseStatus};

pub const LICENSE_PRODUCT: &str = "citadel";
pub const LICENSE_ISSUER: &str = "citadel-p";
pub const LICENSE_AUDIENCE: &str = "citadel-core";
pub const COMMUNITY_EDITION: &str = "Community";
pub const TEAM_EDITION: &str = "Team";
pub const LEGACY_BUSINESS_EDITION: &str = "Business";
pub const LEGACY_LICENSE_SCHEMA: u8 = 1;
pub const CURRENT_LICENSE_SCHEMA: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitadelInstanceIdentity {
    pub instance_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledLicense {
    pub raw_license: String,
    pub fingerprint: String,
    pub installed_at: DateTime<Utc>,
    pub installed_by_actor_id: Option<ActorId>,
    pub last_validated_at: Option<DateTime<Utc>>,
    pub last_validation_status: Option<LicenseStatus>,
    pub last_validation_error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LicenseCustomer {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicensePayload {
    pub schema: u8,
    pub product: String,
    pub issuer: String,
    pub audience: String,
    pub license_id: String,
    pub replaced_license_id: Option<String>,
    pub customer: LicenseCustomer,
    pub edition: String,
    pub instance_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub not_before: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub grace_until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub limits: BTreeMap<String, i64>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedLicense {
    pub raw_license: String,
    pub fingerprint: String,
    pub payload: LicensePayload,
    pub known_capabilities: BTreeSet<LicenseCapability>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseVerificationResult {
    pub status: LicenseStatus,
    pub license: Option<VerifiedLicense>,
    pub error_code: Option<&'static str>,
    pub error_message: Option<String>,
}

impl LicenseVerificationResult {
    #[must_use]
    pub const fn is_accepted(&self) -> bool {
        matches!(
            self.status,
            LicenseStatus::Valid | LicenseStatus::GracePeriod | LicenseStatus::NotYetValid
        ) && self.license.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseState {
    pub status: LicenseStatus,
    pub effective_edition: String,
    pub licensed_edition: Option<String>,
    pub instance_id: Uuid,
    pub license_schema: Option<u8>,
    pub license_id: Option<String>,
    pub replaced_license_id: Option<String>,
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub fingerprint: Option<String>,
    pub issued_at: Option<DateTime<Utc>>,
    pub not_before: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub grace_until: Option<DateTime<Utc>>,
    pub effective_capabilities: BTreeSet<LicenseCapability>,
    pub warnings: Vec<String>,
}

impl LicenseState {
    #[must_use]
    pub fn community(instance_id: Uuid) -> Self {
        Self {
            status: LicenseStatus::Community,
            effective_edition: COMMUNITY_EDITION.to_owned(),
            licensed_edition: None,
            instance_id,
            license_schema: None,
            license_id: None,
            replaced_license_id: None,
            customer_id: None,
            customer_name: None,
            fingerprint: None,
            issued_at: None,
            not_before: None,
            expires_at: None,
            grace_until: None,
            effective_capabilities: BTreeSet::new(),
            warnings: Vec::new(),
        }
    }

    #[must_use]
    pub fn capability_enabled(&self, capability: LicenseCapability) -> bool {
        self.effective_capabilities.contains(&capability)
    }
}
