use serde::Deserialize;
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MfaVerificationInput {
    pub code: Option<String>,
    pub recovery_code: Option<String>,
}

impl From<MfaVerificationInput> for citadel_identity::MfaVerificationInput {
    fn from(value: MfaVerificationInput) -> Self {
        Self {
            code: value.code,
            recovery_code: value.recovery_code,
        }
    }
}

impl From<citadel_identity::MfaVerificationInput> for MfaVerificationInput {
    fn from(value: citadel_identity::MfaVerificationInput) -> Self {
        Self {
            code: value.code,
            recovery_code: value.recovery_code,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmMandatoryMfaSetupInput {
    pub code: String,
}

impl From<ConfirmMandatoryMfaSetupInput> for citadel_identity::ConfirmMandatoryMfaSetupInput {
    fn from(value: ConfirmMandatoryMfaSetupInput) -> Self {
        Self { code: value.code }
    }
}

impl From<citadel_identity::ConfirmMandatoryMfaSetupInput> for ConfirmMandatoryMfaSetupInput {
    fn from(value: citadel_identity::ConfirmMandatoryMfaSetupInput) -> Self {
        Self { code: value.code }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartProfileMfaSetupInput {
    pub password: String,
}

impl From<StartProfileMfaSetupInput> for citadel_identity::StartProfileMfaSetupInput {
    fn from(value: StartProfileMfaSetupInput) -> Self {
        Self {
            password: value.password,
        }
    }
}

impl From<citadel_identity::StartProfileMfaSetupInput> for StartProfileMfaSetupInput {
    fn from(value: citadel_identity::StartProfileMfaSetupInput) -> Self {
        Self {
            password: value.password,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmProfileMfaSetupInput {
    pub code: String,
}

impl From<ConfirmProfileMfaSetupInput> for citadel_identity::ConfirmProfileMfaSetupInput {
    fn from(value: ConfirmProfileMfaSetupInput) -> Self {
        Self { code: value.code }
    }
}

impl From<citadel_identity::ConfirmProfileMfaSetupInput> for ConfirmProfileMfaSetupInput {
    fn from(value: citadel_identity::ConfirmProfileMfaSetupInput) -> Self {
        Self { code: value.code }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisableProfileMfaInput {
    pub password: String,
    pub code: Option<String>,
    pub recovery_code: Option<String>,
}

impl From<DisableProfileMfaInput> for citadel_identity::DisableProfileMfaInput {
    fn from(value: DisableProfileMfaInput) -> Self {
        Self {
            password: value.password,
            code: value.code,
            recovery_code: value.recovery_code,
        }
    }
}

impl From<citadel_identity::DisableProfileMfaInput> for DisableProfileMfaInput {
    fn from(value: citadel_identity::DisableProfileMfaInput) -> Self {
        Self {
            password: value.password,
            code: value.code,
            recovery_code: value.recovery_code,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegenerateProfileMfaRecoveryCodesInput {
    pub password: String,
    pub code: String,
}

impl From<RegenerateProfileMfaRecoveryCodesInput>
    for citadel_identity::RegenerateProfileMfaRecoveryCodesInput
{
    fn from(value: RegenerateProfileMfaRecoveryCodesInput) -> Self {
        Self {
            password: value.password,
            code: value.code,
        }
    }
}

impl From<citadel_identity::RegenerateProfileMfaRecoveryCodesInput>
    for RegenerateProfileMfaRecoveryCodesInput
{
    fn from(value: citadel_identity::RegenerateProfileMfaRecoveryCodesInput) -> Self {
        Self {
            password: value.password,
            code: value.code,
        }
    }
}
