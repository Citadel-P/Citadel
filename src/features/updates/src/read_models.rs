use crate::ReleaseNotice;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
