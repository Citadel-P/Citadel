use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use citadel_domain::{ActivityEvent, ActivityEventInfo, ActorId, MfaPolicy};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::{
    ActorPrincipal, Clock, IdentityError, IdentityService, InitializeCitadelRequest, LoginNextStep,
    LoginRequest, LoginResponse, MfaChallenge, MfaSetupSession, PreparedSession, SessionMetadata,
    SessionTokens, UserAuthentication, UserMfaRecoveryCode, UserMfaSettings,
};

const ISSUER: &str = "Citadel";
const INVALID_CODE: &str = "Invalid or expired verification code.";

#[derive(Debug, Clone, Copy)]
pub struct MfaConfiguration {
    pub policy: MfaPolicy,
    pub challenge_lifetime: Duration,
    pub setup_lifetime: Duration,
    pub maximum_failed_attempts: i32,
    pub recovery_code_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotpSetup {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MfaCredentialAcceptance {
    Totp {
        matched_time_step: i64,
        expected_protected_secret: String,
    },
    RecoveryCode {
        code_hash: String,
    },
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandatoryMfaSetupView {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandatoryMfaSetupCompleteView {
    pub access_token: String,
    pub recovery_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MfaVerificationView {
    pub access_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaSetupView {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaStatusView {
    pub enabled: bool,
    pub remaining_recovery_codes: i32,
    pub policy: MfaPolicy,
    pub can_disable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMfaRecoveryCodesView {
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
    pub response: LoginResponse,
    pub action: BrowserAuthenticationAction,
}

#[derive(Clone)]
pub struct MfaService {
    store: Arc<dyn MfaStore>,
    identity: Arc<IdentityService>,
    totp: Arc<dyn TotpService>,
    protector: Arc<dyn SecretProtector>,
    recovery_codes: Arc<dyn RecoveryCodeService>,
    clock: Arc<dyn Clock>,
    configuration: MfaConfiguration,
}

impl MfaService {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        store: Arc<dyn MfaStore>,
        identity: Arc<IdentityService>,
        totp: Arc<dyn TotpService>,
        protector: Arc<dyn SecretProtector>,
        recovery_codes: Arc<dyn RecoveryCodeService>,
        clock: Arc<dyn Clock>,
        configuration: MfaConfiguration,
    ) -> Self {
        Self {
            store,
            identity,
            totp,
            protector,
            recovery_codes,
            clock,
            configuration,
        }
    }

    pub async fn initialize(
        &self,
        request: InitializeCitadelRequest,
        metadata: SessionMetadata,
    ) -> Result<BrowserAuthenticationResult, IdentityError> {
        let user = self.identity.initialize_user(request).await?;
        self.complete_password_authentication(user, metadata).await
    }

    pub async fn login(
        &self,
        request: LoginRequest,
        metadata: SessionMetadata,
    ) -> Result<BrowserAuthenticationResult, IdentityError> {
        let user = self.identity.authenticate_local(request).await?;
        self.complete_password_authentication(user, metadata).await
    }

    pub async fn profile_status(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<ProfileMfaStatusView, IdentityError> {
        let user = self
            .identity
            .load_user_for_authentication(principal.subject_id)
            .await?;
        let enabled = self.store.settings(user.user_id).await?.is_some();
        let remaining_recovery_codes = if enabled {
            self.store.count_unused_recovery_codes(user.user_id).await?
        } else {
            0
        };
        Ok(ProfileMfaStatusView {
            enabled,
            remaining_recovery_codes,
            policy: self.configuration.policy,
            can_disable: !self.requires_mfa(&user),
        })
    }

    pub async fn start_profile_setup(
        &self,
        principal: &ActorPrincipal,
        input: StartProfileMfaSetupInput,
    ) -> Result<ProfileMfaSetupView, IdentityError> {
        validate_password_input(&input.password)?;
        let user = self
            .identity
            .current_local_user(principal.subject_id, &input.password)
            .await?;
        if self.store.settings(user.user_id).await?.is_some() {
            return Err(IdentityError::Conflict(
                "Two-factor authentication is already enabled.".to_owned(),
            ));
        }
        let (session, setup) = self.create_setup_session(&user)?;
        self.store.start_setup(session, self.clock.now()).await?;
        Ok(ProfileMfaSetupView {
            secret: setup.secret,
            otp_auth_uri: setup.otp_auth_uri,
            expires_at: setup.expires_at,
        })
    }

    pub async fn confirm_profile_setup(
        &self,
        principal: &ActorPrincipal,
        input: ConfirmProfileMfaSetupInput,
        refresh_token: Option<&str>,
    ) -> Result<ProfileMfaRecoveryCodesView, IdentityError> {
        validate_totp_code(&input.code)?;
        let now = self.clock.now();
        let user = self
            .identity
            .load_user_for_authentication(principal.subject_id)
            .await?;
        if user.password_hash.is_none() {
            return Err(no_local_password());
        }
        let session = self
            .store
            .active_setup_session(user.user_id, now)
            .await?
            .ok_or_else(invalid_setup)?;
        let secret = self.unprotect_secret(&session.protected_totp_secret)?;
        let secret = std::str::from_utf8(&secret).map_err(|_| invalid_code())?;
        let matched_time_step = self
            .totp
            .verify(secret, &input.code, now)
            .ok_or_else(invalid_code)?;
        let (plaintext, hashed) = self.create_recovery_codes(user.user_id, now)?;
        let settings = UserMfaSettings {
            user_id: user.user_id,
            protected_totp_secret: session.protected_totp_secret.clone(),
            last_accepted_time_step: Some(matched_time_step),
            enabled_at: now,
            created_at: now,
        };
        let current_session_id = self
            .identity
            .resolve_current_session(user.user_id, refresh_token)
            .await?;
        let activity = user_activity(
            &user,
            user.actor_id,
            ActivityEventInfo::user_mfa_enabled(),
            now,
        )?;
        if !self
            .store
            .confirm_profile_enrollment(ProfileEnrollmentCommit {
                setup_session_id: session.id,
                settings,
                recovery_codes: hashed,
                current_session_id,
                activity,
                now,
            })
            .await?
        {
            return Err(invalid_setup());
        }
        Ok(ProfileMfaRecoveryCodesView {
            enabled: true,
            recovery_codes: plaintext,
        })
    }

    pub async fn mandatory_setup(
        &self,
        setup_session_id: Uuid,
    ) -> Result<MandatoryMfaSetupView, IdentityError> {
        let now = self.clock.now();
        let session = self
            .store
            .setup_session(setup_session_id, now)
            .await?
            .ok_or(IdentityError::Unauthenticated)?;
        let user = self
            .identity
            .load_user_for_authentication(session.user_id)
            .await
            .map_err(|_| IdentityError::Unauthenticated)?;
        let secret = self
            .unprotect_secret(&session.protected_totp_secret)
            .map_err(|_| IdentityError::Unauthenticated)?;
        let secret = std::str::from_utf8(&secret).map_err(|_| IdentityError::Unauthenticated)?;
        Ok(MandatoryMfaSetupView {
            secret: secret.to_owned(),
            otp_auth_uri: self.totp.otp_auth_uri(ISSUER, account_name(&user), secret),
            expires_at: session.expires_at,
        })
    }

    pub async fn confirm_mandatory_setup(
        &self,
        setup_session_id: Uuid,
        input: ConfirmMandatoryMfaSetupInput,
        metadata: SessionMetadata,
    ) -> Result<(MandatoryMfaSetupCompleteView, SessionTokens), IdentityError> {
        validate_totp_code(&input.code)?;
        let now = self.clock.now();
        let session = self
            .store
            .setup_session(setup_session_id, now)
            .await?
            .ok_or(IdentityError::Unauthenticated)?;
        let user = self
            .identity
            .load_user_for_authentication(session.user_id)
            .await
            .map_err(|_| IdentityError::Unauthenticated)?;
        let secret = self.unprotect_secret(&session.protected_totp_secret)?;
        let secret = std::str::from_utf8(&secret).map_err(|_| invalid_code())?;
        let matched_time_step = self
            .totp
            .verify(secret, &input.code, now)
            .ok_or_else(invalid_code)?;
        let (plaintext, hashed) = self.create_recovery_codes(user.user_id, now)?;
        let prepared_session = self.identity.prepare_session(&user, metadata)?;
        let tokens = prepared_session.tokens.clone();
        let activity = user_activity(
            &user,
            user.actor_id,
            ActivityEventInfo::user_mfa_enabled(),
            now,
        )?;
        if !self
            .store
            .confirm_mandatory_enrollment(MandatoryEnrollmentCommit {
                setup_session_id: session.id,
                settings: UserMfaSettings {
                    user_id: user.user_id,
                    protected_totp_secret: session.protected_totp_secret,
                    last_accepted_time_step: Some(matched_time_step),
                    enabled_at: now,
                    created_at: now,
                },
                recovery_codes: hashed,
                prepared_session,
                activity,
                now,
            })
            .await?
        {
            return Err(IdentityError::Unauthenticated);
        }
        Ok((
            MandatoryMfaSetupCompleteView {
                access_token: tokens.access_token.clone(),
                recovery_codes: plaintext,
            },
            tokens,
        ))
    }

    pub async fn verify_challenge(
        &self,
        challenge_id: Uuid,
        input: MfaVerificationInput,
        metadata: SessionMetadata,
    ) -> Result<(MfaVerificationView, SessionTokens), IdentityError> {
        validate_exactly_one(&input.code, &input.recovery_code)?;
        if let Some(code) = &input.code {
            validate_totp_code(code)?;
        }
        if input
            .recovery_code
            .as_ref()
            .is_some_and(|code| code.chars().count() > 64)
        {
            return Err(invalid_code());
        }
        let now = self.clock.now();
        let challenge = self
            .store
            .challenge(
                challenge_id,
                now,
                self.configuration.maximum_failed_attempts,
            )
            .await?
            .ok_or_else(invalid_code)?;
        let user = self
            .identity
            .load_user_for_authentication(challenge.user_id)
            .await
            .map_err(|_| invalid_code())?;
        let settings = self
            .store
            .settings(user.user_id)
            .await?
            .ok_or_else(invalid_code)?;
        let (acceptance, used_recovery_code) = if let Some(code) = input.code {
            let secret = self.unprotect_secret(&settings.protected_totp_secret)?;
            let secret = std::str::from_utf8(&secret).map_err(|_| invalid_code())?;
            let Some(matched_time_step) = self.totp.verify(secret, &code, now) else {
                self.record_failed_attempt(challenge.id, &user, now).await?;
                return Err(invalid_code());
            };
            (
                MfaCredentialAcceptance::Totp {
                    matched_time_step,
                    expected_protected_secret: settings.protected_totp_secret,
                },
                false,
            )
        } else {
            let normalized = self
                .recovery_codes
                .normalize(input.recovery_code.as_deref().unwrap_or_default());
            if normalized.is_empty() {
                self.record_failed_attempt(challenge.id, &user, now).await?;
                return Err(invalid_code());
            }
            (
                MfaCredentialAcceptance::RecoveryCode {
                    code_hash: self.recovery_codes.hash(&normalized),
                },
                true,
            )
        };
        let prepared_session = self.identity.prepare_session(&user, metadata)?;
        let tokens = prepared_session.tokens.clone();
        let activity = used_recovery_code
            .then(|| {
                user_activity(
                    &user,
                    user.actor_id,
                    ActivityEventInfo::user_mfa_recovery_code_used(),
                    now,
                )
            })
            .transpose()?;
        if !self
            .store
            .complete_challenge(ChallengeCompletionCommit {
                challenge_id: challenge.id,
                user_id: user.user_id,
                acceptance,
                prepared_session,
                activity,
                maximum_failed_attempts: self.configuration.maximum_failed_attempts,
                now,
            })
            .await?
        {
            self.record_failed_attempt(challenge.id, &user, now).await?;
            return Err(invalid_code());
        }
        Ok((
            MfaVerificationView {
                access_token: tokens.access_token.clone(),
            },
            tokens,
        ))
    }

    pub async fn disable_profile_mfa(
        &self,
        principal: &ActorPrincipal,
        input: DisableProfileMfaInput,
        refresh_token: Option<&str>,
    ) -> Result<(), IdentityError> {
        validate_password_input(&input.password)?;
        validate_exactly_one(&input.code, &input.recovery_code)?;
        let user = self
            .identity
            .current_local_user(principal.subject_id, &input.password)
            .await?;
        if self.requires_mfa(&user) {
            return Err(IdentityError::Validation(
                "Two-factor authentication is required by policy.".to_owned(),
            ));
        }
        let settings = self
            .store
            .settings(user.user_id)
            .await?
            .ok_or(IdentityError::NotFound)?;
        let now = self.clock.now();
        let acceptance =
            self.credential_acceptance(&settings, input.code, input.recovery_code, now)?;
        let current_session_id = self
            .identity
            .resolve_current_session(user.user_id, refresh_token)
            .await?;
        let activity = user_activity(
            &user,
            user.actor_id,
            ActivityEventInfo::user_mfa_disabled(),
            now,
        )?;
        if !self
            .store
            .disable(DisableMfaCommit {
                user_id: user.user_id,
                acceptance,
                current_session_id,
                activity,
                now,
            })
            .await?
        {
            return Err(invalid_code());
        }
        Ok(())
    }

    pub async fn regenerate_recovery_codes(
        &self,
        principal: &ActorPrincipal,
        input: RegenerateProfileMfaRecoveryCodesInput,
    ) -> Result<ProfileMfaRecoveryCodesView, IdentityError> {
        validate_password_input(&input.password)?;
        validate_totp_code(&input.code)?;
        let user = self
            .identity
            .current_local_user(principal.subject_id, &input.password)
            .await?;
        let settings = self
            .store
            .settings(user.user_id)
            .await?
            .ok_or(IdentityError::NotFound)?;
        let now = self.clock.now();
        let secret = self.unprotect_secret(&settings.protected_totp_secret)?;
        let secret = std::str::from_utf8(&secret).map_err(|_| invalid_code())?;
        let matched_time_step = self
            .totp
            .verify(secret, &input.code, now)
            .ok_or_else(invalid_code)?;
        let (plaintext, hashed) = self.create_recovery_codes(user.user_id, now)?;
        let activity = user_activity(
            &user,
            user.actor_id,
            ActivityEventInfo::user_mfa_recovery_codes_regenerated(),
            now,
        )?;
        if !self
            .store
            .regenerate_recovery_codes(RegenerateRecoveryCodesCommit {
                user_id: user.user_id,
                expected_protected_secret: settings.protected_totp_secret,
                matched_time_step,
                recovery_codes: hashed,
                activity,
            })
            .await?
        {
            return Err(invalid_code());
        }
        Ok(ProfileMfaRecoveryCodesView {
            enabled: true,
            recovery_codes: plaintext,
        })
    }

    pub async fn reset_user_mfa(
        &self,
        user_id: Uuid,
        actor_id: ActorId,
    ) -> Result<(), IdentityError> {
        let user = self.identity.load_user_by_id(user_id).await?;
        let activity = user_activity(
            &user,
            actor_id,
            ActivityEventInfo::user_mfa_reset_by_administrator(user.user_id),
            self.clock.now(),
        )?;
        self.store.reset(ResetMfaCommit { user_id, activity }).await
    }

    async fn complete_password_authentication(
        &self,
        user: UserAuthentication,
        metadata: SessionMetadata,
    ) -> Result<BrowserAuthenticationResult, IdentityError> {
        let now = self.clock.now();
        if self.store.settings(user.user_id).await?.is_some() {
            let challenge = MfaChallenge {
                id: Uuid::now_v7(),
                user_id: user.user_id,
                expires_at: now + self.configuration.challenge_lifetime,
                failed_attempts: 0,
                consumed_at: None,
                created_at: now,
            };
            self.store.start_challenge(challenge.clone(), now).await?;
            return Ok(BrowserAuthenticationResult {
                response: LoginResponse {
                    access_token: None,
                    next_step: LoginNextStep::VerifyMfa,
                },
                action: BrowserAuthenticationAction::VerifyMfa {
                    challenge_id: challenge.id,
                    expires_at: challenge.expires_at,
                },
            });
        }
        if self.requires_mfa(&user) {
            let (session, _) = self.create_setup_session(&user)?;
            self.store.start_setup(session.clone(), now).await?;
            return Ok(BrowserAuthenticationResult {
                response: LoginResponse {
                    access_token: None,
                    next_step: LoginNextStep::EnrollMfa,
                },
                action: BrowserAuthenticationAction::EnrollMfa {
                    setup_session_id: session.id,
                    expires_at: session.expires_at,
                },
            });
        }
        let session = self.identity.issue_session(&user, metadata).await?;
        Ok(BrowserAuthenticationResult {
            response: LoginResponse {
                access_token: Some(session.access_token.clone()),
                next_step: LoginNextStep::Completed,
            },
            action: BrowserAuthenticationAction::Completed(session),
        })
    }

    fn requires_mfa(&self, user: &UserAuthentication) -> bool {
        if user.password_hash.is_none() {
            return false;
        }
        match self.configuration.policy {
            MfaPolicy::Optional => false,
            MfaPolicy::RequiredForAdministrators => user
                .roles
                .iter()
                .any(|role| role.eq_ignore_ascii_case("Admin")),
            MfaPolicy::RequiredForAllUsers => true,
        }
    }

    fn create_setup_session(
        &self,
        user: &UserAuthentication,
    ) -> Result<(MfaSetupSession, TotpSetup), IdentityError> {
        let now = self.clock.now();
        let expires_at = now + self.configuration.setup_lifetime;
        let setup = self
            .totp
            .create_setup(ISSUER, account_name(user), expires_at)?;
        let protected_totp_secret = self.protector.protect(setup.secret.as_bytes())?;
        Ok((
            MfaSetupSession {
                id: Uuid::now_v7(),
                user_id: user.user_id,
                protected_totp_secret,
                expires_at,
                consumed_at: None,
                created_at: now,
            },
            setup,
        ))
    }

    fn create_recovery_codes(
        &self,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(Vec<String>, Vec<UserMfaRecoveryCode>), IdentityError> {
        let plaintext = self
            .recovery_codes
            .generate(self.configuration.recovery_code_count)?;
        let hashed = plaintext
            .iter()
            .map(|code| UserMfaRecoveryCode {
                id: Uuid::now_v7(),
                user_id,
                code_hash: self
                    .recovery_codes
                    .hash(&self.recovery_codes.normalize(code)),
                used_at: None,
                created_at: now,
            })
            .collect();
        Ok((plaintext, hashed))
    }

    fn unprotect_secret(&self, envelope: &str) -> Result<Zeroizing<Vec<u8>>, IdentityError> {
        self.protector
            .unprotect(envelope)
            .map_err(|_| invalid_code())
    }

    fn credential_acceptance(
        &self,
        settings: &UserMfaSettings,
        code: Option<String>,
        recovery_code: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<MfaCredentialAcceptance, IdentityError> {
        if let Some(code) = code {
            validate_totp_code(&code)?;
            let secret = self.protector.unprotect(&settings.protected_totp_secret)?;
            let secret = std::str::from_utf8(&secret).map_err(|_| invalid_code())?;
            let matched_time_step = self
                .totp
                .verify(secret, &code, now)
                .ok_or_else(invalid_code)?;
            return Ok(MfaCredentialAcceptance::Totp {
                matched_time_step,
                expected_protected_secret: settings.protected_totp_secret.clone(),
            });
        }
        let recovery_code = recovery_code.ok_or_else(invalid_code)?;
        if recovery_code.chars().count() > 64 {
            return Err(invalid_code());
        }
        let normalized = self.recovery_codes.normalize(&recovery_code);
        if normalized.is_empty() {
            return Err(invalid_code());
        }
        Ok(MfaCredentialAcceptance::RecoveryCode {
            code_hash: self.recovery_codes.hash(&normalized),
        })
    }

    async fn record_failed_attempt(
        &self,
        challenge_id: Uuid,
        user: &UserAuthentication,
        now: DateTime<Utc>,
    ) -> Result<(), IdentityError> {
        let activity = user_activity(
            user,
            user.actor_id,
            ActivityEventInfo::user_mfa_verification_failed(),
            now,
        )?;
        let _ = self
            .store
            .record_challenge_failure(
                challenge_id,
                self.configuration.maximum_failed_attempts,
                now,
                activity,
            )
            .await?;
        Ok(())
    }
}

fn user_activity(
    user: &UserAuthentication,
    actor_id: ActorId,
    info: ActivityEventInfo,
    now: DateTime<Utc>,
) -> Result<ActivityEvent, IdentityError> {
    ActivityEvent::new_user_event(user.user_id, user.name.clone(), actor_id, info, now)
        .map_err(|error| IdentityError::Storage(format!("Invalid MFA Activity: {error}")))
}

fn account_name(user: &UserAuthentication) -> &str {
    if user.email.trim().is_empty() {
        &user.name
    } else {
        &user.email
    }
}

fn validate_password_input(password: &str) -> Result<(), IdentityError> {
    if !(6..=128).contains(&password.chars().count()) {
        return Err(IdentityError::Validation(
            "Password must contain 6 to 128 characters.".to_owned(),
        ));
    }
    Ok(())
}

fn validate_totp_code(code: &str) -> Result<(), IdentityError> {
    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid_code());
    }
    Ok(())
}

fn validate_exactly_one(
    code: &Option<String>,
    recovery_code: &Option<String>,
) -> Result<(), IdentityError> {
    let has_code = code.as_ref().is_some_and(|value| !value.trim().is_empty());
    let has_recovery = recovery_code
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    if has_code == has_recovery {
        return Err(IdentityError::Validation(
            "Exactly one of Code or RecoveryCode is required.".to_owned(),
        ));
    }
    Ok(())
}

fn invalid_code() -> IdentityError {
    IdentityError::Validation(INVALID_CODE.to_owned())
}

fn invalid_setup() -> IdentityError {
    IdentityError::Validation("Invalid or expired setup session.".to_owned())
}

fn no_local_password() -> IdentityError {
    IdentityError::Validation("This account does not have a local password credential.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_applies_only_to_local_users_and_matches_administrator_roles() {
        let local_admin = user(Some("hash"), &["Admin"]);
        let local_user = user(Some("hash"), &["Viewer"]);
        let oidc_admin = user(None, &["Admin"]);

        assert!(requires(MfaPolicy::RequiredForAdministrators, &local_admin));
        assert!(!requires(MfaPolicy::RequiredForAdministrators, &local_user));
        assert!(!requires(MfaPolicy::RequiredForAdministrators, &oidc_admin));
        assert!(requires(MfaPolicy::RequiredForAllUsers, &local_user));
        assert!(!requires(MfaPolicy::RequiredForAllUsers, &oidc_admin));
    }

    #[test]
    fn credential_input_requires_exactly_one_bounded_value() {
        assert!(validate_exactly_one(&Some("123456".into()), &None).is_ok());
        assert!(validate_exactly_one(&None, &Some("ABCD".into())).is_ok());
        assert!(validate_exactly_one(&None, &None).is_err());
        assert!(validate_exactly_one(&Some("123456".into()), &Some("ABCD".into())).is_err());
        assert!(validate_totp_code("123456").is_ok());
        assert!(validate_totp_code("12345x").is_err());
    }

    fn requires(policy: MfaPolicy, user: &UserAuthentication) -> bool {
        user.password_hash.is_some()
            && match policy {
                MfaPolicy::Optional => false,
                MfaPolicy::RequiredForAdministrators => user
                    .roles
                    .iter()
                    .any(|role| role.eq_ignore_ascii_case("Admin")),
                MfaPolicy::RequiredForAllUsers => true,
            }
    }

    fn user(password_hash: Option<&str>, roles: &[&str]) -> UserAuthentication {
        UserAuthentication {
            user_id: Uuid::now_v7(),
            actor_id: ActorId::new(Uuid::now_v7()),
            name: "owner".into(),
            email: "owner@example.test".into(),
            password_hash: password_hash.map(str::to_owned),
            enabled: true,
            roles: roles.iter().map(|role| (*role).to_owned()).collect(),
        }
    }
}
