use serde::Serialize;

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
