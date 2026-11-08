use serde::{Deserialize, Serialize};

/// Non-secret, typed installation choices. Consumers apply these at Core startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct InstanceConfiguration {
    pub password_minimum_length: u32,
    pub browser_session_lifetime_days: u32,
    pub service_account_default_token_lifetime_days: i32,
    pub service_account_maximum_token_lifetime_days: i32,
    pub service_account_maximum_active_tokens: i32,
    pub access_token_lifetime_minutes: u32,
    pub statistics_collection_interval_seconds: u32,
    pub statistics_write_interval_seconds: u32,
    pub scripts_enabled: bool,
    pub scripts_maximum_parallel_runs: usize,
    pub scripts_maximum_remote_runs: usize,
    pub scripts_default_timeout_seconds: u32,
    pub scripts_maximum_timeout_seconds: u32,
    pub build_preparation_slots: usize,
    pub build_tracking_slots: usize,
    pub build_cleanup_enabled: bool,
    pub build_retention_days: i32,
    pub backups_enabled: bool,
    pub backup_tracking_slots: usize,
    pub backup_default_timeout_seconds: u32,
    pub openapi_enabled: bool,
    pub require_direct_agent_https: bool,
    pub agent_image_override: String,
    pub log_verbosity: String,
    pub log_format: String,
}

impl Default for InstanceConfiguration {
    fn default() -> Self {
        Self {
            password_minimum_length: 15,
            browser_session_lifetime_days: 30,
            service_account_default_token_lifetime_days: 90,
            service_account_maximum_token_lifetime_days: 365,
            service_account_maximum_active_tokens: 10,
            access_token_lifetime_minutes: 15,
            statistics_collection_interval_seconds: 10,
            statistics_write_interval_seconds: 60,
            scripts_enabled: true,
            scripts_maximum_parallel_runs: 4,
            scripts_maximum_remote_runs: 16,
            scripts_default_timeout_seconds: 300,
            scripts_maximum_timeout_seconds: 1800,
            build_preparation_slots: 2,
            build_tracking_slots: 16,
            build_cleanup_enabled: true,
            build_retention_days: 90,
            backups_enabled: true,
            backup_tracking_slots: 16,
            backup_default_timeout_seconds: 120,
            openapi_enabled: false,
            require_direct_agent_https: false,
            agent_image_override: String::new(),
            log_verbosity: "info".into(),
            log_format: "Text".into(),
        }
    }
}

/// Explicit deployment overrides; absence is distinct from a parsed default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstanceOverrides {
    pub password_minimum_length: Option<u32>,
    pub browser_session_lifetime_days: Option<u32>,
    pub service_account_default_token_lifetime_days: Option<i32>,
    pub service_account_maximum_token_lifetime_days: Option<i32>,
    pub service_account_maximum_active_tokens: Option<i32>,
    pub access_token_lifetime_minutes: Option<u32>,
    pub statistics_collection_interval_seconds: Option<u32>,
    pub statistics_write_interval_seconds: Option<u32>,
    pub scripts_enabled: Option<bool>,
    pub scripts_maximum_parallel_runs: Option<usize>,
    pub scripts_maximum_remote_runs: Option<usize>,
    pub scripts_default_timeout_seconds: Option<u32>,
    pub scripts_maximum_timeout_seconds: Option<u32>,
    pub build_preparation_slots: Option<usize>,
    pub build_tracking_slots: Option<usize>,
    pub build_cleanup_enabled: Option<bool>,
    pub build_retention_days: Option<i32>,
    pub backups_enabled: Option<bool>,
    pub backup_tracking_slots: Option<usize>,
    pub backup_default_timeout_seconds: Option<u32>,
    pub openapi_enabled: Option<bool>,
    pub require_direct_agent_https: Option<bool>,
    pub agent_image_override: Option<String>,
    pub log_verbosity: Option<String>,
    pub log_format: Option<String>,
}

/// Stored once for the installation, independently of personal preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationSettings {
    pub configuration: InstanceConfiguration,
    pub initialized: bool,
    pub update_checks_enabled: bool,
    pub update_check_interval_hours: i32,
    pub mfa_policy: citadel_identity::MfaPolicy,
}

/// Deployment restrictions remain authoritative over database choices.
#[derive(Debug, Clone)]
pub struct InstallationPolicy {
    pub running: InstanceConfiguration,
    pub overrides: InstanceOverrides,
    pub update_checks_allowed: bool,
    pub minimum_mfa_policy: citadel_identity::MfaPolicy,
}

impl Default for InstallationPolicy {
    fn default() -> Self {
        Self {
            running: Default::default(),
            overrides: Default::default(),
            update_checks_allowed: true,
            minimum_mfa_policy: citadel_identity::MfaPolicy::Optional,
        }
    }
}
