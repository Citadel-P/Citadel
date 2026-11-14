use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConnectionsView {
    pub maximum: u32,
    pub open: u32,
    pub idle: u32,
    pub borrowed: u32,
    pub shared_buffers_bytes: Option<u64>,
}

impl From<citadel_instance::DatabaseConnections> for DatabaseConnectionsView {
    fn from(value: citadel_instance::DatabaseConnections) -> Self {
        Self {
            maximum: value.maximum,
            open: value.open,
            idle: value.idle,
            borrowed: value.borrowed,
            shared_buffers_bytes: value.shared_buffers_bytes,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstallationSettingsView {
    pub configuration: super::configuration::InstanceConfigurationView,
    pub effective_configuration: super::configuration::InstanceConfigurationView,
    pub running_configuration: super::configuration::InstanceConfigurationView,
    pub deployment_overrides: super::configuration::InstanceOverridesView,
    pub restart_required: bool,
    pub update_checks_enabled: bool,
    pub update_check_interval_hours: i32,
    pub update_checks_allowed: bool,
    #[schema(value_type = crate::api::resources::vocabulary::MfaPolicySchema)]
    pub mfa_policy: citadel_identity::MfaPolicy,
    #[schema(value_type = crate::api::resources::vocabulary::MfaPolicySchema)]
    pub minimum_mfa_policy: citadel_identity::MfaPolicy,
}

impl InstallationSettingsView {
    pub fn new(
        settings: citadel_instance::InstallationSettings,
        policy: citadel_instance::InstallationPolicy,
    ) -> Self {
        Self {
            restart_required: settings.configuration.effective(&policy.overrides) != policy.running,
            effective_configuration: settings.configuration.effective(&policy.overrides).into(),
            configuration: settings.configuration.into(),
            running_configuration: policy.running.into(),
            deployment_overrides: policy.overrides.into(),
            update_checks_enabled: settings.update_checks_enabled,
            update_check_interval_hours: settings.update_check_interval_hours,
            update_checks_allowed: policy.update_checks_allowed,
            mfa_policy: settings.mfa_policy,
            minimum_mfa_policy: policy.minimum_mfa_policy,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRestartView {
    pub available: bool,
    pub pending: bool,
    pub boot_id: Option<uuid::Uuid>,
}
