use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InitializeCitadelRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub update_checks_enabled: Option<bool>,
    #[serde(default)]
    #[schema(value_type = Option<crate::api::resources::vocabulary::MfaPolicySchema>)]
    pub mfa_policy: Option<citadel_identity::MfaPolicy>,
}

impl From<InitializeCitadelRequest> for citadel_identity::InitializeCitadel {
    fn from(value: InitializeCitadelRequest) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
            update_checks_enabled: value.update_checks_enabled,
            mfa_policy: value.mfa_policy,
        }
    }
}

impl From<citadel_identity::InitializeCitadel> for InitializeCitadelRequest {
    fn from(value: citadel_identity::InitializeCitadel) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
            update_checks_enabled: value.update_checks_enabled,
            mfa_policy: value.mfa_policy,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email_or_name: String,
    pub password: String,
}

impl From<LoginRequest> for citadel_identity::Login {
    fn from(value: LoginRequest) -> Self {
        Self {
            email_or_name: value.email_or_name,
            password: value.password,
        }
    }
}

impl From<citadel_identity::Login> for LoginRequest {
    fn from(value: citadel_identity::Login) -> Self {
        Self {
            email_or_name: value.email_or_name,
            password: value.password,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchInstallationSettingsRequest {
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub update_checks_enabled: citadel_primitives::PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<crate::api::resources::vocabulary::MfaPolicySchema>, required = false)]
    pub mfa_policy: citadel_primitives::PatchField<citadel_identity::MfaPolicy>,
}

impl From<PatchInstallationSettingsRequest> for citadel_identity::PatchInstallationSettings {
    fn from(value: PatchInstallationSettingsRequest) -> Self {
        Self {
            update_checks_enabled: value.update_checks_enabled,
            mfa_policy: value.mfa_policy,
        }
    }
}
