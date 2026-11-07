use crate::ReleaseNotice;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Counts of unfinished work, without resource names, credentials or runtime output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateDrainBlocker {
    pub kind: String,
    pub count: u64,
}

/// A point-in-time work observation, never checkpoint or replacement permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateDrainOutcome {
    Idle,
    Busy(Vec<UpdateDrainBlocker>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckState {
    pub last_attempt: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub available: Option<ReleaseNotice>,
}

/// Current release identity and persisted check result for presentation.
#[derive(Debug, Clone)]
pub struct ReleaseCheckStatus {
    pub installed_version: String,
    pub channel: crate::ReleaseChannel,
    pub checks: CheckState,
}

/// Authenticated target metadata, not installation or transition approval.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectedRelease {
    pub identity: crate::CoreIdentity,
    pub descriptor: crate::UpgradeDescriptor,
}

/// A consistent read of initialized identity and a fully applied migration catalog.
/// Database size is capacity-planning evidence, not a predicted dump size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateDatabaseState {
    pub instance_id: uuid::Uuid,
    pub catalog_sha256: String,
    pub database_bytes: u64,
}

/// How the connected PostgreSQL server is deployed; this is not update approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseInstallation {
    Bundled,
    External,
    Unknown,
}
