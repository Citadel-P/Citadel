use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusView {
    pub instance_id: Uuid,
    pub installed_version: String,
    /// PostgreSQL server version; absence never means the engine is up to date.
    #[schema(required = true)]
    pub database_version: Option<String>,
    #[schema(required = true)]
    pub database_available: Option<String>,
    pub database_managed: bool,
    /// Deployment topology is independent of installation registration.
    pub database_installation: DatabaseInstallationView,
    #[schema(required = true)]
    pub database_update_reason: Option<String>,
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
    #[schema(required = true)]
    pub installation_unavailable_reason: Option<String>,
    #[schema(required = true)]
    pub operation: Option<UpdateOperationView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum DatabaseInstallationView {
    Bundled,
    External,
    Unknown,
}

impl From<citadel_updates::DatabaseInstallation> for DatabaseInstallationView {
    fn from(value: citadel_updates::DatabaseInstallation) -> Self {
        match value {
            citadel_updates::DatabaseInstallation::Bundled => Self::Bundled,
            citadel_updates::DatabaseInstallation::External => Self::External,
            citadel_updates::DatabaseInstallation::Unknown => Self::Unknown,
        }
    }
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
    pub fn set_database_update(
        &mut self,
        result: Result<Option<citadel_updates::DatabaseUpdatePlan>, citadel_updates::UpdateError>,
    ) {
        use citadel_updates::UpdateError;
        self.database_managed = false;
        self.database_available = None;
        self.database_update_reason = match result {
            Ok(database) => {
                self.database_managed = true;
                self.database_available = database.map(|db| db.target_display());
                None
            }
            Err(
                UpdateError::Unavailable(reason)
                | UpdateError::Conflict(reason)
                | UpdateError::Validation(reason),
            ) => Some(reason),
            // Storage errors can contain private connection details. Other errors
            // also must not be presented as proof the database is unreachable.
            Err(_) => Some("Could not check database update eligibility. Try again.".into()),
        };
    }

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
            database_version: None,
            database_available: None,
            database_managed: false,
            database_installation: DatabaseInstallationView::Unknown,
            database_update_reason: None,
            channel: status.channel.as_str().into(),
            checks_allowed,
            checks_enabled,
            last_attempt: checks.last_attempt,
            last_success: checks.last_success,
            check_error: checks.error,
            available: checks.available.map(Into::into),
            installation_unavailable_reason: Some(
                "Register this installation before installing updates.".into(),
            ),
            operation: None,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOperationView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    pub id: Uuid,
    pub version: String,
    pub phase: String,
    pub updated_at: DateTime<Utc>,
}
impl From<citadel_updates::UpdateOperation> for UpdateOperationView {
    fn from(operation: citadel_updates::UpdateOperation) -> Self {
        Self {
            component: operation.plan.database.as_ref().map(|_| "Database".into()),
            id: operation.id,
            version: operation
                .plan
                .database
                .as_ref()
                .map(|db| db.target_display())
                .unwrap_or(operation.plan.target.version),
            phase: format!("{:?}", operation.phase),
            updated_at: operation.updated_at,
        }
    }
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReviewView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    pub id: Uuid,
    pub installed_version: String,
    pub target_version: String,
    pub expires_at: DateTime<Utc>,
}
impl From<citadel_updates::UpdatePlan> for UpdateReviewView {
    fn from(plan: citadel_updates::UpdatePlan) -> Self {
        Self {
            component: plan.database.as_ref().map(|_| "Database".into()),
            id: plan.id,
            installed_version: plan
                .database
                .as_ref()
                .map(|db| {
                    format!(
                        "{}.{}",
                        db.source_version / 10000,
                        db.source_version % 10000
                    )
                })
                .unwrap_or(plan.source.version),
            target_version: plan
                .database
                .as_ref()
                .map(|db| db.target_display())
                .unwrap_or(plan.target.version),
            expires_at: plan.expires_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_updates::{
        CheckState, DatabaseUpdatePlan, ReleaseChannel, ReleaseCheckStatus, UpdateError,
    };

    fn status() -> UpdateStatusView {
        UpdateStatusView::new(
            ReleaseCheckStatus {
                installed_version: "0.1.3".into(),
                channel: ReleaseChannel::Stable,
                checks: CheckState::default(),
            },
            Uuid::new_v4(),
            true,
            true,
        )
    }

    #[test]
    fn database_eligibility_preserves_actionable_errors_and_clears_stale_targets() {
        let mut view = status();
        let reason = "PostgreSQL 17 is outside the supported update range.";
        for error in [
            UpdateError::Conflict(reason.into()),
            UpdateError::Unavailable(reason.into()),
            UpdateError::Validation(reason.into()),
        ] {
            view.set_database_update(Ok(Some(DatabaseUpdatePlan {
                source_version: 180001,
                target_version: 180006,
                target_image: format!("postgres@sha256:{}", "a".repeat(64)),
            })));
            assert!(view.database_managed);
            assert_eq!(view.database_available.as_deref(), Some("18.6"));
            assert!(view.database_update_reason.is_none());

            view.set_database_update(Err(error));
            assert!(!view.database_managed);
            assert!(view.database_available.is_none());
            assert_eq!(view.database_update_reason.as_deref(), Some(reason));
        }
        view.set_database_update(Ok(None));
        assert!(view.database_managed);
        assert!(view.database_available.is_none());
        assert!(view.database_update_reason.is_none());
    }

    #[test]
    fn database_status_redacts_storage_errors_without_inventing_connectivity_failures() {
        let mut view = status();
        for error in [
            UpdateError::Storage("private connection details".into()),
            UpdateError::Cancelled,
        ] {
            view.set_database_update(Err(error));
            assert_eq!(
                view.database_update_reason.as_deref(),
                Some("Could not check database update eligibility. Try again.")
            );
        }
    }
}
