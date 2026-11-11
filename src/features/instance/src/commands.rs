use crate::InstanceOverrides;
use citadel_primitives::PatchField;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchInstanceConfiguration {
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub password_minimum_length: PatchField<u32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub browser_session_lifetime_days: PatchField<u32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub service_account_default_token_lifetime_days: PatchField<i32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub service_account_maximum_token_lifetime_days: PatchField<i32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub service_account_maximum_active_tokens: PatchField<i32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub access_token_lifetime_minutes: PatchField<u32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub statistics_collection_interval_seconds: PatchField<u32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub statistics_write_interval_seconds: PatchField<u32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub scripts_enabled: PatchField<bool>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub scripts_maximum_parallel_runs: PatchField<usize>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub scripts_maximum_remote_runs: PatchField<usize>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub scripts_default_timeout_seconds: PatchField<u32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub scripts_maximum_timeout_seconds: PatchField<u32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub build_preparation_slots: PatchField<usize>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub build_tracking_slots: PatchField<usize>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub build_cleanup_enabled: PatchField<bool>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub build_retention_days: PatchField<i32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub backups_enabled: PatchField<bool>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub backup_tracking_slots: PatchField<usize>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub backup_default_timeout_seconds: PatchField<u32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub openapi_enabled: PatchField<bool>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub require_direct_agent_https: PatchField<bool>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub agent_image_override: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub log_verbosity: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub log_format: PatchField<crate::LogFormat>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchInstallationSettings {
    #[serde(default)]
    pub configuration: citadel_primitives::PatchField<PatchInstanceConfiguration>,
    /// Trusted deployment context supplied by the use case, never deserialized.
    #[serde(skip)]
    pub deployment_overrides: InstanceOverrides,
    #[serde(skip)]
    pub deployment_core_agent_requires_http: bool,
    #[serde(default)]
    pub update_checks_enabled: citadel_primitives::PatchField<bool>,
    #[serde(default)]
    pub update_check_interval_hours: citadel_primitives::PatchField<i32>,
    #[serde(default)]
    pub mfa_policy: citadel_primitives::PatchField<citadel_identity::MfaPolicy>,
}
