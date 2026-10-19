use super::*;

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandatoryMfaSetupDetails {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandatoryMfaSetupCompleteDetails {
    pub access_token: String,
    pub recovery_codes: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MfaVerificationDetails {
    pub access_token: String,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
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

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaRecoveryCodesDetails {
    pub enabled: bool,
    pub recovery_codes: Vec<String>,
}

/// Setup/challenge IDs become authentication cookies, so they are credentials.
#[derive(Clone)]
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

impl std::fmt::Debug for MandatoryMfaSetupDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MandatoryMfaSetupDetails")
            .field("secret", &"[REDACTED]")
            .field("otp_auth_uri", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl std::fmt::Debug for MandatoryMfaSetupCompleteDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MandatoryMfaSetupCompleteDetails")
            .field("access_token", &"[REDACTED]")
            .field("recovery_codes", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for MfaVerificationDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MfaVerificationDetails")
            .field("access_token", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for ProfileMfaSetupDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileMfaSetupDetails")
            .field("secret", &"[REDACTED]")
            .field("otp_auth_uri", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl std::fmt::Debug for ProfileMfaRecoveryCodesDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileMfaRecoveryCodesDetails")
            .field("enabled", &self.enabled)
            .field("recovery_codes", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for BrowserAuthenticationAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed(tokens) => f.debug_tuple("Completed").field(tokens).finish(),
            Self::VerifyMfa { expires_at, .. } => f
                .debug_struct("VerifyMfa")
                .field("challenge_id", &"[REDACTED]")
                .field("expires_at", expires_at)
                .finish(),
            Self::EnrollMfa { expires_at, .. } => f
                .debug_struct("EnrollMfa")
                .field("setup_session_id", &"[REDACTED]")
                .field("expires_at", expires_at)
                .finish(),
        }
    }
}
