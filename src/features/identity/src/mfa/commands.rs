use super::*;

#[derive(Clone)]
pub struct ProfileEnrollmentCommit {
    pub setup_session_id: Uuid,
    pub settings: UserMfaSettings,
    pub recovery_codes: Vec<UserMfaRecoveryCode>,
    pub current_session_id: Option<Uuid>,
    pub activity: ActivityEvent,
    pub now: DateTime<Utc>,
}

#[derive(Clone)]
pub struct MandatoryEnrollmentCommit {
    pub setup_session_id: Uuid,
    pub settings: UserMfaSettings,
    pub recovery_codes: Vec<UserMfaRecoveryCode>,
    pub prepared_session: PreparedSession,
    pub activity: ActivityEvent,
    pub now: DateTime<Utc>,
}

#[derive(Clone)]
pub struct ChallengeCompletionCommit {
    pub challenge_id: Uuid,
    pub user_id: Uuid,
    pub acceptance: MfaCredentialAcceptance,
    pub prepared_session: PreparedSession,
    pub activity: Option<ActivityEvent>,
    pub maximum_failed_attempts: i32,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DisableMfaCommit {
    pub user_id: Uuid,
    pub acceptance: MfaCredentialAcceptance,
    pub current_session_id: Option<Uuid>,
    pub activity: ActivityEvent,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RegenerateRecoveryCodesCommit {
    pub user_id: Uuid,
    pub expected_protected_secret: String,
    pub matched_time_step: i64,
    pub recovery_codes: Vec<UserMfaRecoveryCode>,
    pub activity: ActivityEvent,
}

#[derive(Debug, Clone)]
pub struct ResetMfaCommit {
    pub user_id: Uuid,
    pub activity: ActivityEvent,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MfaVerificationInput {
    pub code: Option<String>,
    pub recovery_code: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmMandatoryMfaSetupInput {
    pub code: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartProfileMfaSetupInput {
    pub password: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmProfileMfaSetupInput {
    pub code: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisableProfileMfaInput {
    pub password: String,
    pub code: Option<String>,
    pub recovery_code: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegenerateProfileMfaRecoveryCodesInput {
    pub password: String,
    pub code: String,
}

impl std::fmt::Debug for MfaVerificationInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MfaVerificationInput")
            .field("code", &"[REDACTED]")
            .field("recovery_code", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for ConfirmMandatoryMfaSetupInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmMandatoryMfaSetupInput")
            .field("code", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for StartProfileMfaSetupInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StartProfileMfaSetupInput")
            .field("password", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for ConfirmProfileMfaSetupInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmProfileMfaSetupInput")
            .field("code", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for DisableProfileMfaInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DisableProfileMfaInput")
            .field("password", &"[REDACTED]")
            .field("code", &"[REDACTED]")
            .field("recovery_code", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for RegenerateProfileMfaRecoveryCodesInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegenerateProfileMfaRecoveryCodesInput")
            .field("password", &"[REDACTED]")
            .field("code", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for ProfileEnrollmentCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileEnrollmentCommit")
            .field("now", &self.now)
            .field("setup_session_id", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for MandatoryEnrollmentCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MandatoryEnrollmentCommit")
            .field("now", &self.now)
            .field("setup_session_id", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ChallengeCompletionCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChallengeCompletionCommit")
            .field("user_id", &self.user_id)
            .field("now", &self.now)
            .field("challenge_id", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}
