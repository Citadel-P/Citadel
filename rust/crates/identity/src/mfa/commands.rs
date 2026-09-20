use super::*;

#[derive(Debug, Clone)]
pub struct ProfileEnrollmentCommit {
    pub setup_session_id: Uuid,
    pub settings: UserMfaSettings,
    pub recovery_codes: Vec<UserMfaRecoveryCode>,
    pub current_session_id: Option<Uuid>,
    pub activity: ActivityEvent,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MandatoryEnrollmentCommit {
    pub setup_session_id: Uuid,
    pub settings: UserMfaSettings,
    pub recovery_codes: Vec<UserMfaRecoveryCode>,
    pub prepared_session: PreparedSession,
    pub activity: ActivityEvent,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MfaVerificationInput {
    pub code: Option<String>,
    pub recovery_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmMandatoryMfaSetupInput {
    pub code: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartProfileMfaSetupInput {
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmProfileMfaSetupInput {
    pub code: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisableProfileMfaInput {
    pub password: String,
    pub code: Option<String>,
    pub recovery_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegenerateProfileMfaRecoveryCodesInput {
    pub password: String,
    pub code: String,
}
