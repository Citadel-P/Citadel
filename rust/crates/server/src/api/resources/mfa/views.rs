use chrono::{DateTime, Utc};
use citadel_identity::MfaPolicy;
use serde::Serialize;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MandatoryMfaSetupView {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
}

impl From<MandatoryMfaSetupView> for citadel_identity::MandatoryMfaSetupDetails {
    fn from(value: MandatoryMfaSetupView) -> Self {
        Self {
            secret: value.secret,
            otp_auth_uri: value.otp_auth_uri,
            expires_at: value.expires_at,
        }
    }
}

impl From<citadel_identity::MandatoryMfaSetupDetails> for MandatoryMfaSetupView {
    fn from(value: citadel_identity::MandatoryMfaSetupDetails) -> Self {
        Self {
            secret: value.secret,
            otp_auth_uri: value.otp_auth_uri,
            expires_at: value.expires_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MandatoryMfaSetupCompleteView {
    pub access_token: String,
    pub recovery_codes: Vec<String>,
}

impl From<MandatoryMfaSetupCompleteView> for citadel_identity::MandatoryMfaSetupCompleteDetails {
    fn from(value: MandatoryMfaSetupCompleteView) -> Self {
        Self {
            access_token: value.access_token,
            recovery_codes: value.recovery_codes,
        }
    }
}

impl From<citadel_identity::MandatoryMfaSetupCompleteDetails> for MandatoryMfaSetupCompleteView {
    fn from(value: citadel_identity::MandatoryMfaSetupCompleteDetails) -> Self {
        Self {
            access_token: value.access_token,
            recovery_codes: value.recovery_codes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MfaVerificationView {
    pub access_token: String,
}

impl From<MfaVerificationView> for citadel_identity::MfaVerificationDetails {
    fn from(value: MfaVerificationView) -> Self {
        Self {
            access_token: value.access_token,
        }
    }
}

impl From<citadel_identity::MfaVerificationDetails> for MfaVerificationView {
    fn from(value: citadel_identity::MfaVerificationDetails) -> Self {
        Self {
            access_token: value.access_token,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaSetupView {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
}

impl From<ProfileMfaSetupView> for citadel_identity::ProfileMfaSetupDetails {
    fn from(value: ProfileMfaSetupView) -> Self {
        Self {
            secret: value.secret,
            otp_auth_uri: value.otp_auth_uri,
            expires_at: value.expires_at,
        }
    }
}

impl From<citadel_identity::ProfileMfaSetupDetails> for ProfileMfaSetupView {
    fn from(value: citadel_identity::ProfileMfaSetupDetails) -> Self {
        Self {
            secret: value.secret,
            otp_auth_uri: value.otp_auth_uri,
            expires_at: value.expires_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaStatusView {
    pub enabled: bool,
    pub remaining_recovery_codes: i32,
    #[schema(value_type = crate::api::resources::vocabulary::MfaPolicySchema)]
    pub policy: MfaPolicy,
    pub can_disable: bool,
}

impl From<ProfileMfaStatusView> for citadel_identity::ProfileMfaStatusDetails {
    fn from(value: ProfileMfaStatusView) -> Self {
        Self {
            enabled: value.enabled,
            remaining_recovery_codes: value.remaining_recovery_codes,
            policy: value.policy,
            can_disable: value.can_disable,
        }
    }
}

impl From<citadel_identity::ProfileMfaStatusDetails> for ProfileMfaStatusView {
    fn from(value: citadel_identity::ProfileMfaStatusDetails) -> Self {
        Self {
            enabled: value.enabled,
            remaining_recovery_codes: value.remaining_recovery_codes,
            policy: value.policy,
            can_disable: value.can_disable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaRecoveryCodesView {
    pub enabled: bool,
    pub recovery_codes: Vec<String>,
}

impl From<ProfileMfaRecoveryCodesView> for citadel_identity::ProfileMfaRecoveryCodesDetails {
    fn from(value: ProfileMfaRecoveryCodesView) -> Self {
        Self {
            enabled: value.enabled,
            recovery_codes: value.recovery_codes,
        }
    }
}

impl From<citadel_identity::ProfileMfaRecoveryCodesDetails> for ProfileMfaRecoveryCodesView {
    fn from(value: citadel_identity::ProfileMfaRecoveryCodesDetails) -> Self {
        Self {
            enabled: value.enabled,
            recovery_codes: value.recovery_codes,
        }
    }
}
