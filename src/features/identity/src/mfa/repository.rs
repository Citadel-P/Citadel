use super::*;

pub trait TotpService: Send + Sync {
    fn create_setup(
        &self,
        issuer: &str,
        account_name: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<TotpSetup, IdentityError>;

    fn verify(&self, secret: &str, code: &str, now: DateTime<Utc>) -> Option<i64>;

    fn otp_auth_uri(&self, issuer: &str, account_name: &str, secret: &str) -> String;
}

pub trait SecretProtector: Send + Sync {
    fn protect(&self, plaintext: &[u8]) -> Result<String, IdentityError>;
    fn unprotect(&self, envelope: &str) -> Result<Zeroizing<Vec<u8>>, IdentityError>;
}

pub trait RecoveryCodeService: Send + Sync {
    fn generate(&self, count: usize) -> Result<Vec<String>, IdentityError>;
    fn normalize(&self, code: &str) -> String;
    fn hash(&self, normalized_code: &str) -> String;
    fn verify(&self, normalized_code: &str, expected_hash: &str) -> bool;
}

pub trait MfaStore: Send + Sync {
    fn settings(
        &self,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<UserMfaSettings>, IdentityError>>;

    fn count_unused_recovery_codes(
        &self,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<i32, IdentityError>>;

    fn start_setup(
        &self,
        session: MfaSetupSession,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>>;

    fn start_challenge(
        &self,
        challenge: MfaChallenge,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>>;

    fn setup_session(
        &self,
        session_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<MfaSetupSession>, IdentityError>>;

    fn active_setup_session(
        &self,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<MfaSetupSession>, IdentityError>>;

    fn challenge(
        &self,
        challenge_id: Uuid,
        now: DateTime<Utc>,
        maximum_failed_attempts: i32,
    ) -> BoxFuture<'_, Result<Option<MfaChallenge>, IdentityError>>;

    fn confirm_profile_enrollment(
        &self,
        commit: ProfileEnrollmentCommit,
    ) -> BoxFuture<'_, Result<bool, IdentityError>>;

    fn confirm_mandatory_enrollment(
        &self,
        commit: MandatoryEnrollmentCommit,
    ) -> BoxFuture<'_, Result<bool, IdentityError>>;

    fn complete_challenge(
        &self,
        commit: ChallengeCompletionCommit,
    ) -> BoxFuture<'_, Result<bool, IdentityError>>;

    fn record_challenge_failure(
        &self,
        challenge_id: Uuid,
        maximum_failed_attempts: i32,
        now: DateTime<Utc>,
        activity: ActivityEvent,
    ) -> BoxFuture<'_, Result<bool, IdentityError>>;

    fn disable(&self, commit: DisableMfaCommit) -> BoxFuture<'_, Result<bool, IdentityError>>;

    fn regenerate_recovery_codes(
        &self,
        commit: RegenerateRecoveryCodesCommit,
    ) -> BoxFuture<'_, Result<bool, IdentityError>>;

    fn reset(&self, commit: ResetMfaCommit) -> BoxFuture<'_, Result<(), IdentityError>>;
}
