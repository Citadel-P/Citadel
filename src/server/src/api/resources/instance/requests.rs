use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchInstallationSettingsRequest {
    #[serde(default)]
    #[schema(value_type = Option<super::configuration::PatchInstanceConfigurationRequest>, required = false)]
    pub configuration:
        citadel_primitives::PatchField<super::configuration::PatchInstanceConfigurationRequest>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub update_checks_enabled: citadel_primitives::PatchField<bool>,
    /// Automatic release-check interval. Supported values: 6, 12, 24 hours.
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    pub update_check_interval_hours: citadel_primitives::PatchField<i32>,
    #[serde(default)]
    #[schema(value_type = Option<crate::api::resources::vocabulary::MfaPolicySchema>, required = false)]
    pub mfa_policy: citadel_primitives::PatchField<citadel_identity::MfaPolicy>,
}

impl From<PatchInstallationSettingsRequest> for citadel_instance::PatchInstallationSettings {
    fn from(value: PatchInstallationSettingsRequest) -> Self {
        Self {
            configuration: value.configuration.map(Into::into),
            deployment_overrides: Default::default(),
            deployment_core_agent_requires_http: false,
            update_checks_enabled: value.update_checks_enabled,
            update_check_interval_hours: value.update_check_interval_hours,
            mfa_policy: value.mfa_policy,
        }
    }
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestartInstanceRequest {
    pub boot_id: uuid::Uuid,
}
