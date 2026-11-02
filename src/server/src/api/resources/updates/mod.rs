use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusView {
    pub instance_id: Uuid,
    pub installed_version: String,
    pub channel: String,
    pub checks_allowed: bool,
    pub checks_enabled: bool,
    #[schema(required = true)]
    pub last_attempt: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub last_success: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub check_error: Option<String>,
    #[schema(required = true)]
    pub available: Option<AvailableUpdate>,
    pub installation_unavailable_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpdate {
    pub version: String,
    pub release_url: String,
}

impl From<citadel_updates::ReleaseNotice> for AvailableUpdate {
    fn from(notice: citadel_updates::ReleaseNotice) -> Self {
        Self {
            version: notice.version,
            release_url: notice.release_url,
        }
    }
}

impl UpdateStatusView {
    pub fn new(
        status: citadel_updates::ReleaseCheckStatus,
        instance_id: Uuid,
        checks_allowed: bool,
        checks_enabled: bool,
    ) -> Self {
        let checks = status.checks;
        Self {
            instance_id,
            installed_version: status.installed_version,
            channel: status.channel.as_str().into(),
            checks_allowed,
            checks_enabled,
            last_attempt: checks.last_attempt,
            last_success: checks.last_success,
            check_error: checks.error,
            available: checks.available.map(Into::into),
            installation_unavailable_reason:
                "In-app installation is not available in this build. Use the manual upgrade guide."
                    .into(),
        }
    }
}
