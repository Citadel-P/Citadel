use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstanceConfigurationView {
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
impl From<citadel_instance::InstanceConfiguration> for InstanceConfigurationView {
    fn from(value: citadel_instance::InstanceConfiguration) -> Self {
        Self {
            password_minimum_length: value.password_minimum_length,
            browser_session_lifetime_days: value.browser_session_lifetime_days,
            service_account_default_token_lifetime_days: value
                .service_account_default_token_lifetime_days,
            service_account_maximum_token_lifetime_days: value
                .service_account_maximum_token_lifetime_days,
            service_account_maximum_active_tokens: value.service_account_maximum_active_tokens,
            access_token_lifetime_minutes: value.access_token_lifetime_minutes,
            statistics_collection_interval_seconds: value.statistics_collection_interval_seconds,
            statistics_write_interval_seconds: value.statistics_write_interval_seconds,
            scripts_enabled: value.scripts_enabled,
            scripts_maximum_parallel_runs: value.scripts_maximum_parallel_runs,
            scripts_maximum_remote_runs: value.scripts_maximum_remote_runs,
            scripts_default_timeout_seconds: value.scripts_default_timeout_seconds,
            scripts_maximum_timeout_seconds: value.scripts_maximum_timeout_seconds,
            build_preparation_slots: value.build_preparation_slots,
            build_tracking_slots: value.build_tracking_slots,
            build_cleanup_enabled: value.build_cleanup_enabled,
            build_retention_days: value.build_retention_days,
            backups_enabled: value.backups_enabled,
            backup_tracking_slots: value.backup_tracking_slots,
            backup_default_timeout_seconds: value.backup_default_timeout_seconds,
            openapi_enabled: value.openapi_enabled,
            require_direct_agent_https: value.require_direct_agent_https,
            agent_image_override: value.agent_image_override,
            log_verbosity: value.log_verbosity,
            log_format: value.log_format,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstanceOverridesView {
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
impl From<citadel_instance::InstanceOverrides> for InstanceOverridesView {
    fn from(value: citadel_instance::InstanceOverrides) -> Self {
        Self {
            password_minimum_length: value.password_minimum_length,
            browser_session_lifetime_days: value.browser_session_lifetime_days,
            service_account_default_token_lifetime_days: value
                .service_account_default_token_lifetime_days,
            service_account_maximum_token_lifetime_days: value
                .service_account_maximum_token_lifetime_days,
            service_account_maximum_active_tokens: value.service_account_maximum_active_tokens,
            access_token_lifetime_minutes: value.access_token_lifetime_minutes,
            statistics_collection_interval_seconds: value.statistics_collection_interval_seconds,
            statistics_write_interval_seconds: value.statistics_write_interval_seconds,
            scripts_enabled: value.scripts_enabled,
            scripts_maximum_parallel_runs: value.scripts_maximum_parallel_runs,
            scripts_maximum_remote_runs: value.scripts_maximum_remote_runs,
            scripts_default_timeout_seconds: value.scripts_default_timeout_seconds,
            scripts_maximum_timeout_seconds: value.scripts_maximum_timeout_seconds,
            build_preparation_slots: value.build_preparation_slots,
            build_tracking_slots: value.build_tracking_slots,
            build_cleanup_enabled: value.build_cleanup_enabled,
            build_retention_days: value.build_retention_days,
            backups_enabled: value.backups_enabled,
            backup_tracking_slots: value.backup_tracking_slots,
            backup_default_timeout_seconds: value.backup_default_timeout_seconds,
            openapi_enabled: value.openapi_enabled,
            require_direct_agent_https: value.require_direct_agent_https,
            agent_image_override: value.agent_image_override,
            log_verbosity: value.log_verbosity,
            log_format: value.log_format,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchInstanceConfigurationRequest {
    #[serde(default)]
    #[schema(value_type = Option<u32>, required = false)]
    pub password_minimum_length: citadel_primitives::PatchField<u32>,
    #[serde(default)]
    #[schema(value_type = Option<u32>, required = false)]
    pub browser_session_lifetime_days: citadel_primitives::PatchField<u32>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    pub service_account_default_token_lifetime_days: citadel_primitives::PatchField<i32>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    pub service_account_maximum_token_lifetime_days: citadel_primitives::PatchField<i32>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    pub service_account_maximum_active_tokens: citadel_primitives::PatchField<i32>,
    #[serde(default)]
    #[schema(value_type = Option<u32>, required = false)]
    pub access_token_lifetime_minutes: citadel_primitives::PatchField<u32>,
    #[serde(default)]
    #[schema(value_type = Option<u32>, required = false)]
    pub statistics_collection_interval_seconds: citadel_primitives::PatchField<u32>,
    #[serde(default)]
    #[schema(value_type = Option<u32>, required = false)]
    pub statistics_write_interval_seconds: citadel_primitives::PatchField<u32>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub scripts_enabled: citadel_primitives::PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<usize>, required = false)]
    pub scripts_maximum_parallel_runs: citadel_primitives::PatchField<usize>,
    #[serde(default)]
    #[schema(value_type = Option<usize>, required = false)]
    pub scripts_maximum_remote_runs: citadel_primitives::PatchField<usize>,
    #[serde(default)]
    #[schema(value_type = Option<u32>, required = false)]
    pub scripts_default_timeout_seconds: citadel_primitives::PatchField<u32>,
    #[serde(default)]
    #[schema(value_type = Option<u32>, required = false)]
    pub scripts_maximum_timeout_seconds: citadel_primitives::PatchField<u32>,
    #[serde(default)]
    #[schema(value_type = Option<usize>, required = false)]
    pub build_preparation_slots: citadel_primitives::PatchField<usize>,
    #[serde(default)]
    #[schema(value_type = Option<usize>, required = false)]
    pub build_tracking_slots: citadel_primitives::PatchField<usize>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub build_cleanup_enabled: citadel_primitives::PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    pub build_retention_days: citadel_primitives::PatchField<i32>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub backups_enabled: citadel_primitives::PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<usize>, required = false)]
    pub backup_tracking_slots: citadel_primitives::PatchField<usize>,
    #[serde(default)]
    #[schema(value_type = Option<u32>, required = false)]
    pub backup_default_timeout_seconds: citadel_primitives::PatchField<u32>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub openapi_enabled: citadel_primitives::PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub require_direct_agent_https: citadel_primitives::PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub agent_image_override: citadel_primitives::PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub log_verbosity: citadel_primitives::PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub log_format: citadel_primitives::PatchField<String>,
}
impl From<PatchInstanceConfigurationRequest> for citadel_instance::PatchInstanceConfiguration {
    fn from(value: PatchInstanceConfigurationRequest) -> Self {
        Self {
            password_minimum_length: value.password_minimum_length,
            browser_session_lifetime_days: value.browser_session_lifetime_days,
            service_account_default_token_lifetime_days: value
                .service_account_default_token_lifetime_days,
            service_account_maximum_token_lifetime_days: value
                .service_account_maximum_token_lifetime_days,
            service_account_maximum_active_tokens: value.service_account_maximum_active_tokens,
            access_token_lifetime_minutes: value.access_token_lifetime_minutes,
            statistics_collection_interval_seconds: value.statistics_collection_interval_seconds,
            statistics_write_interval_seconds: value.statistics_write_interval_seconds,
            scripts_enabled: value.scripts_enabled,
            scripts_maximum_parallel_runs: value.scripts_maximum_parallel_runs,
            scripts_maximum_remote_runs: value.scripts_maximum_remote_runs,
            scripts_default_timeout_seconds: value.scripts_default_timeout_seconds,
            scripts_maximum_timeout_seconds: value.scripts_maximum_timeout_seconds,
            build_preparation_slots: value.build_preparation_slots,
            build_tracking_slots: value.build_tracking_slots,
            build_cleanup_enabled: value.build_cleanup_enabled,
            build_retention_days: value.build_retention_days,
            backups_enabled: value.backups_enabled,
            backup_tracking_slots: value.backup_tracking_slots,
            backup_default_timeout_seconds: value.backup_default_timeout_seconds,
            openapi_enabled: value.openapi_enabled,
            require_direct_agent_https: value.require_direct_agent_https,
            agent_image_override: value.agent_image_override,
            log_verbosity: value.log_verbosity,
            log_format: value.log_format,
        }
    }
}
