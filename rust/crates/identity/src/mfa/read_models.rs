use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandatoryMfaSetupDetails {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandatoryMfaSetupCompleteDetails {
    pub access_token: String,
    pub recovery_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MfaVerificationDetails {
    pub access_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaSetupDetails {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaStatusDetails {
    pub enabled: bool,
    pub remaining_recovery_codes: i32,
    pub policy: MfaPolicy,
    pub can_disable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaRecoveryCodesDetails {
    pub enabled: bool,
    pub recovery_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum BrowserAuthenticationAction {
    Completed(SessionTokens),
    VerifyMfa {
        challenge_id: Uuid,
        expires_at: DateTime<Utc>,
    },
    EnrollMfa {
        setup_session_id: Uuid,
        expires_at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone)]
pub struct BrowserAuthenticationResult {
    pub response: LoginOutcome,
    pub action: BrowserAuthenticationAction,
}
