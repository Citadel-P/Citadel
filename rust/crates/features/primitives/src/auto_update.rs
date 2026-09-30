//! Shared image-update state and digest comparison for managed workloads.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateBehavior {
    #[default]
    #[serde(alias = "disabled")]
    Disabled,
    #[serde(alias = "notify")]
    Notify,
    #[serde(alias = "autoDeploy", alias = "autodeploy")]
    AutoDeploy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AutoUpdateStatus {
    Unknown,
    UpToDate,
    UpdateAvailable,
    Updating,
    Failed,
}

impl AutoUpdateStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::UpToDate => "UpToDate",
            Self::UpdateAvailable => "UpdateAvailable",
            Self::Updating => "Updating",
            Self::Failed => "Failed",
        }
    }
}

impl std::str::FromStr for AutoUpdateStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Unknown" => Ok(Self::Unknown),
            "UpToDate" => Ok(Self::UpToDate),
            "UpdateAvailable" => Ok(Self::UpdateAvailable),
            "Updating" => Ok(Self::Updating),
            "Failed" => Ok(Self::Failed),
            _ => Err(format!("Unknown auto-update status '{value}'.")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateState {
    pub last_checked_at: DateTime<Utc>,
    pub status: AutoUpdateStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl AutoUpdateState {
    pub fn checked(current: &str, remote: &str, checked_at: DateTime<Utc>) -> Self {
        Self {
            last_checked_at: checked_at,
            status: if image_digests_equal(current, remote) {
                AutoUpdateStatus::UpToDate
            } else {
                AutoUpdateStatus::UpdateAvailable
            },
            current_digest: Some(image_digest(current).to_owned()),
            remote_digest: Some(image_digest(remote).to_owned()),
            last_error: None,
        }
    }
}

fn image_digest(reference: &str) -> &str {
    reference.rsplit('@').next().unwrap_or(reference)
}

/// Docker may return either repository@digest or a bare digest.
pub fn image_digests_equal(current: &str, remote: &str) -> bool {
    image_digest(current).eq_ignore_ascii_case(image_digest(remote))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_behavior_keeps_canonical_names_defaults_and_supported_input_aliases() {
        assert_eq!(UpdateBehavior::default(), UpdateBehavior::Disabled);
        for (inputs, expected, canonical) in [
            (
                &["Disabled", "disabled"][..],
                UpdateBehavior::Disabled,
                "Disabled",
            ),
            (&["Notify", "notify"][..], UpdateBehavior::Notify, "Notify"),
            (
                &["AutoDeploy", "autoDeploy", "autodeploy"][..],
                UpdateBehavior::AutoDeploy,
                "AutoDeploy",
            ),
        ] {
            for input in inputs {
                let value: UpdateBehavior =
                    serde_json::from_value(serde_json::json!(input)).unwrap();
                assert_eq!(value, expected);
                assert_eq!(serde_json::to_value(value).unwrap(), canonical);
            }
        }
        for invalid in [
            serde_json::json!("AutoUpdate"),
            serde_json::json!("Notify "),
            serde_json::Value::Null,
        ] {
            assert!(serde_json::from_value::<UpdateBehavior>(invalid).is_err());
        }
    }

    #[test]
    fn persisted_statuses_round_trip_and_unknown_values_are_rejected() {
        for status in [
            AutoUpdateStatus::Unknown,
            AutoUpdateStatus::UpToDate,
            AutoUpdateStatus::UpdateAvailable,
            AutoUpdateStatus::Updating,
            AutoUpdateStatus::Failed,
        ] {
            assert_eq!(status.as_str().parse::<AutoUpdateStatus>().unwrap(), status);
            assert_eq!(
                serde_json::from_str::<AutoUpdateStatus>(&serde_json::to_string(&status).unwrap())
                    .unwrap(),
                status
            );
        }
        assert!("Unexpected".parse::<AutoUpdateStatus>().is_err());
        assert!(serde_json::from_str::<AutoUpdateStatus>("\"Unexpected\"").is_err());
    }

    #[test]
    fn digest_checks_compare_content_and_keep_the_applied_baseline() {
        let checked_at = Utc::now();
        for (current, remote) in [
            ("registry/app@sha256:ABC", "sha256:abc"),
            ("sha256:abc", "registry/app@sha256:ABC"),
            ("one/app@sha256:abc", "two/app@sha256:abc"),
        ] {
            let equal = AutoUpdateState::checked(current, remote, checked_at);
            assert_eq!(equal.status, AutoUpdateStatus::UpToDate);
            assert_eq!(equal.last_checked_at, checked_at);
            assert!(equal.last_error.is_none());
        }
        let changed = AutoUpdateState::checked("registry/app@sha256:old", "sha256:new", checked_at);
        assert_eq!(changed.status, AutoUpdateStatus::UpdateAvailable);
        assert_eq!(changed.current_digest.as_deref(), Some("sha256:old"));
        assert_eq!(changed.remote_digest.as_deref(), Some("sha256:new"));
    }

    #[test]
    fn absent_digest_and_error_fields_are_omitted_and_deserialize_as_none() {
        let state: AutoUpdateState = serde_json::from_value(serde_json::json!({
            "status": "Unknown", "lastCheckedAt": "2026-09-30T00:00:00Z"
        }))
        .unwrap();
        assert!(state.current_digest.is_none());
        assert!(state.remote_digest.is_none());
        assert!(state.last_error.is_none());
        let json = serde_json::to_value(state).unwrap();
        assert_eq!(json.as_object().unwrap().len(), 2);
    }
}
