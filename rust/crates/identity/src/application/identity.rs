use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use citadel_domain::{
    ActorId, AuthenticatedPrincipalType, PermissionLevel, ResourceType, SpecificPermission,
};
use email_address::EmailAddress;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::{
    ActorPrincipal, AuthorizationSnapshot, MAX_NAME_CHARS, PermissionGrant, SYSTEM_ACTOR_ID,
    ServiceAccountCredential, ServiceAccountLastUsedTracker, User, UserAuthentication,
};

pub const MINIMUM_PASSWORD_CHARACTERS: usize = 15;
pub const MAXIMUM_PASSWORD_CHARACTERS: usize = 128;
pub const MAXIMUM_SESSIONS_PER_USER: i64 = 10;

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("authentication is required")]
    Unauthenticated,
    #[error("the authenticated principal is not allowed to perform this operation")]
    Forbidden,
    #[error("Citadel setup is required")]
    SetupRequired,
    #[error("Citadel setup is already complete")]
    SetupAlreadyComplete,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("one or more validation errors occurred")]
    FieldValidation(std::collections::BTreeMap<String, Vec<String>>),
    #[error("resource conflict: {0}")]
    Conflict(String),
    #[error("resource conflict: {message}")]
    TypedConflict {
        problem_type: &'static str,
        message: String,
    },
    #[error("resource was not found")]
    NotFound,
    #[error("{0}")]
    ResourceNotFound(&'static str),
    #[error("license capability '{0}' is unavailable")]
    LicenseRequired(&'static str),
    #[error("identity storage failed: {0}")]
    Storage(String),
    #[error("identity credential processing failed")]
    Credential,
    #[error("external operation failed: {0}")]
    External(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeCitadelRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email_or_name: String,
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LoginNextStep {
    Completed,
    VerifyMfa,
    EnrollMfa,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub access_token: Option<String>,
    pub next_step: LoginNextStep,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatusResponse {
    pub requires_setup: bool,
}

#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PreparedSession {
    pub session: NewSession,
    pub tokens: SessionTokens,
    pub expected_password_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub metadata: SessionMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSessionRecord {
    pub id: Uuid,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub subject_id: Uuid,
    pub actor_id: ActorId,
    pub principal_type: AuthenticatedPrincipalType,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub automation_run_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedBearer {
    pub principal: ActorPrincipal,
    pub automation_run_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub session_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub trait IdentityStore: Send + Sync {
    fn setup_required(&self) -> BoxFuture<'_, Result<bool, IdentityError>>;

    fn initialize_administrator<'a>(
        &'a self,
        administrator: &'a User,
        mode: citadel_domain::SetupInitializationMode,
    ) -> BoxFuture<'a, Result<UserAuthentication, IdentityError>>;

    fn find_user_for_login<'a>(
        &'a self,
        identifier: &'a str,
    ) -> BoxFuture<'a, Result<Option<UserAuthentication>, IdentityError>>;

    fn load_user_by_id(
        &self,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<UserAuthentication>, IdentityError>>;

    fn create_session<'a>(
        &'a self,
        session: &'a NewSession,
        expected_password_hash: Option<&'a str>,
        maximum_sessions: i64,
    ) -> BoxFuture<'a, Result<(), IdentityError>>;

    fn load_session_user(
        &self,
        session_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<UserAuthentication>, IdentityError>>;

    fn touch_session<'a>(
        &'a self,
        session_id: Uuid,
        now: DateTime<Utc>,
        metadata: &'a SessionMetadata,
    ) -> BoxFuture<'a, Result<bool, IdentityError>>;

    fn delete_session(&self, session_id: Uuid) -> BoxFuture<'_, Result<(), IdentityError>>;

    fn active_session_id(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<Uuid>, IdentityError>>;

    fn list_active_sessions(
        &self,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Vec<UserSessionRecord>, IdentityError>>;

    fn delete_owned_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        current_session_id: Option<Uuid>,
        actor_id: ActorId,
        revoked_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, IdentityError>>;

    fn delete_other_sessions(
        &self,
        user_id: Uuid,
        current_session_id: Uuid,
        now: DateTime<Utc>,
        actor_id: ActorId,
    ) -> BoxFuture<'_, Result<Option<i64>, IdentityError>>;

    fn load_principal(
        &self,
        actor_id: ActorId,
        expected_subject_id: Uuid,
        principal_type: AuthenticatedPrincipalType,
        credential_id: Option<Uuid>,
    ) -> BoxFuture<'_, Result<Option<ActorPrincipal>, IdentityError>>;

    fn load_run_as_principal(
        &self,
        actor_id: ActorId,
    ) -> BoxFuture<'_, Result<Option<ActorPrincipal>, IdentityError>>;

    fn load_service_account_credential(
        &self,
        credential_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<ServiceAccountCredential>, IdentityError>>;

    fn authorization_snapshot(
        &self,
        actor_id: ActorId,
    ) -> BoxFuture<'_, Result<AuthorizationSnapshot, IdentityError>>;

    fn resource_permission<'a>(
        &'a self,
        actor_id: ActorId,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> BoxFuture<'a, Result<Option<PermissionGrant>, IdentityError>>;
}

pub trait PasswordHasher: Send + Sync {
    fn hash(&self, password: &str) -> Result<String, IdentityError>;
    fn verify(&self, password: &str, encoded_hash: &str) -> bool;
    fn dummy_hash(&self) -> String;
}

pub trait SessionTokenCodec: Send + Sync {
    fn encode_access(&self, claims: &AccessTokenClaims) -> Result<String, IdentityError>;
    fn decode_access(&self, token: &str) -> Result<AccessTokenClaims, IdentityError>;
    fn encode_refresh(&self, claims: &RefreshTokenClaims) -> Result<String, IdentityError>;
    fn decode_refresh(&self, token: &str) -> Result<RefreshTokenClaims, IdentityError>;
}

pub trait ServiceAccountTokenCodec: Send + Sync {
    fn issue(&self, credential_id: Uuid) -> Result<(String, [u8; 32]), IdentityError>;
    fn parse_and_hash(&self, token: &str) -> Option<(Uuid, [u8; 32])>;
}

pub trait EntitlementService: Send + Sync {
    fn custom_access_control_enabled(&self) -> BoxFuture<'_, Result<bool, IdentityError>>;
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

pub struct IdentityService {
    store: Arc<dyn IdentityStore>,
    passwords: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn SessionTokenCodec>,
    service_account_tokens: Arc<dyn ServiceAccountTokenCodec>,
    entitlements: Arc<dyn EntitlementService>,
    clock: Arc<dyn Clock>,
    access_token_lifetime: Duration,
    refresh_token_lifetime: Duration,
    credential_workers: Arc<Semaphore>,
    last_used_tracker: Arc<dyn ServiceAccountLastUsedTracker>,
}

impl IdentityService {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        store: Arc<dyn IdentityStore>,
        passwords: Arc<dyn PasswordHasher>,
        tokens: Arc<dyn SessionTokenCodec>,
        service_account_tokens: Arc<dyn ServiceAccountTokenCodec>,
        entitlements: Arc<dyn EntitlementService>,
        clock: Arc<dyn Clock>,
        last_used_tracker: Arc<dyn ServiceAccountLastUsedTracker>,
        access_token_lifetime: Duration,
        refresh_token_lifetime: Duration,
    ) -> Self {
        Self {
            store,
            passwords,
            tokens,
            service_account_tokens,
            entitlements,
            clock,
            access_token_lifetime,
            refresh_token_lifetime,
            credential_workers: Arc::new(Semaphore::new(2)),
            last_used_tracker,
        }
    }

    pub async fn setup_status(&self) -> Result<SetupStatusResponse, IdentityError> {
        Ok(SetupStatusResponse {
            requires_setup: self.store.setup_required().await?,
        })
    }

    pub async fn initialize(
        &self,
        request: InitializeCitadelRequest,
        metadata: SessionMetadata,
    ) -> Result<(LoginResponse, SessionTokens), IdentityError> {
        let user = self.initialize_user(request).await?;
        let session = self.issue_session(&user, metadata).await?;
        Ok((
            LoginResponse {
                access_token: Some(session.access_token.clone()),
                next_step: LoginNextStep::Completed,
            },
            session,
        ))
    }

    pub async fn initialize_user(
        &self,
        request: InitializeCitadelRequest,
    ) -> Result<UserAuthentication, IdentityError> {
        self.initialize_user_in_mode(
            request,
            citadel_domain::SetupInitializationMode::Interactive,
        )
        .await
    }

    pub async fn initialize_user_in_mode(
        &self,
        request: InitializeCitadelRequest,
        mode: citadel_domain::SetupInitializationMode,
    ) -> Result<UserAuthentication, IdentityError> {
        validate_name(&request.name)?;
        validate_email(&request.email)?;
        validate_password(&request.password, Some(&request.name), Some(&request.email))?;
        let now = self.clock.now();
        let administrator = User::new(
            request.name.trim().to_owned(),
            request.email.trim().to_ascii_lowercase(),
            Some(self.hash_password(request.password).await?),
            ActorId::new(Uuid::now_v7()),
            ActorId::new(SYSTEM_ACTOR_ID),
            now,
        );
        self.store
            .initialize_administrator(&administrator, mode)
            .await
    }

    pub async fn login(
        &self,
        request: LoginRequest,
        metadata: SessionMetadata,
    ) -> Result<(LoginResponse, SessionTokens), IdentityError> {
        let user = self.authenticate_local(request).await?;
        let session = self.issue_session(&user, metadata).await?;
        Ok((
            LoginResponse {
                access_token: Some(session.access_token.clone()),
                next_step: LoginNextStep::Completed,
            },
            session,
        ))
    }

    pub async fn authenticate_local(
        &self,
        request: LoginRequest,
    ) -> Result<UserAuthentication, IdentityError> {
        if request.email_or_name.trim().is_empty()
            || request.password.chars().count() > MAXIMUM_PASSWORD_CHARACTERS
        {
            return Err(IdentityError::InvalidCredentials);
        }
        let user = self
            .store
            .find_user_for_login(request.email_or_name.trim())
            .await?;
        let hash = user
            .as_ref()
            .and_then(|user| user.password_hash.clone())
            .unwrap_or_else(|| self.passwords.dummy_hash());
        let password_valid = self.verify_password(request.password, hash).await?;
        let user = user
            .filter(|user| user.enabled && user.password_hash.is_some())
            .ok_or(IdentityError::InvalidCredentials)?;
        if !password_valid {
            return Err(IdentityError::InvalidCredentials);
        }
        Ok(user)
    }

    pub async fn current_local_user(
        &self,
        user_id: Uuid,
        password: &str,
    ) -> Result<UserAuthentication, IdentityError> {
        let user = self
            .store
            .load_user_by_id(user_id)
            .await?
            .filter(|user| user.enabled)
            .ok_or(IdentityError::NotFound)?;
        let Some(password_hash) = user.password_hash.clone() else {
            return Err(IdentityError::Validation(
                "This account does not have a local password credential.".to_owned(),
            ));
        };
        if password.chars().count() > MAXIMUM_PASSWORD_CHARACTERS
            || !self
                .verify_password(password.to_owned(), password_hash)
                .await?
        {
            return Err(IdentityError::Validation(
                "Current password is incorrect.".to_owned(),
            ));
        }
        Ok(user)
    }

    pub async fn load_user_for_authentication(
        &self,
        user_id: Uuid,
    ) -> Result<UserAuthentication, IdentityError> {
        self.store
            .load_user_by_id(user_id)
            .await?
            .filter(|user| user.enabled)
            .ok_or(IdentityError::InvalidCredentials)
    }

    pub async fn load_user_by_id(
        &self,
        user_id: Uuid,
    ) -> Result<UserAuthentication, IdentityError> {
        self.store
            .load_user_by_id(user_id)
            .await?
            .ok_or(IdentityError::NotFound)
    }

    pub async fn refresh(
        &self,
        refresh_token: &str,
        metadata: SessionMetadata,
    ) -> Result<SessionTokens, IdentityError> {
        let claims = self
            .tokens
            .decode_refresh(refresh_token)
            .map_err(|_| IdentityError::InvalidCredentials)?;
        let now = self.clock.now();
        if claims.expires_at <= now {
            return Err(IdentityError::InvalidCredentials);
        }
        let user = self
            .store
            .load_session_user(claims.session_id, now)
            .await?
            .filter(|user| user.enabled)
            .ok_or(IdentityError::InvalidCredentials)?;
        if !self
            .store
            .touch_session(claims.session_id, now, &metadata)
            .await?
        {
            return Err(IdentityError::InvalidCredentials);
        }
        let access = self.tokens.encode_access(&AccessTokenClaims {
            subject_id: user.user_id,
            actor_id: user.actor_id,
            principal_type: AuthenticatedPrincipalType::User,
            issued_at: now,
            expires_at: now + self.access_token_lifetime,
            automation_run_id: None,
        })?;
        Ok(SessionTokens {
            access_token: access,
            refresh_token: refresh_token.to_owned(),
            refresh_expires_at: claims.expires_at,
        })
    }

    pub async fn logout(&self, refresh_token: Option<&str>) -> Result<(), IdentityError> {
        if let Some(token) = refresh_token
            && let Ok(claims) = self.tokens.decode_refresh(token)
        {
            self.store.delete_session(claims.session_id).await?;
        }
        Ok(())
    }

    pub async fn resolve_current_session(
        &self,
        user_id: Uuid,
        refresh_token: Option<&str>,
    ) -> Result<Option<Uuid>, IdentityError> {
        let Some(refresh_token) = refresh_token else {
            return Ok(None);
        };
        let Ok(claims) = self.tokens.decode_refresh(refresh_token) else {
            return Ok(None);
        };
        let now = self.clock.now();
        if claims.expires_at <= now {
            return Ok(None);
        }
        self.store
            .active_session_id(claims.session_id, user_id, now)
            .await
    }

    pub async fn list_active_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserSessionRecord>, IdentityError> {
        self.store
            .list_active_sessions(user_id, self.clock.now())
            .await
    }

    pub async fn revoke_owned_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        current_session_id: Option<Uuid>,
        actor_id: ActorId,
        revoked_at: DateTime<Utc>,
    ) -> Result<bool, IdentityError> {
        self.store
            .delete_owned_session(
                session_id,
                user_id,
                current_session_id,
                actor_id,
                revoked_at,
            )
            .await
    }

    pub async fn revoke_other_sessions(
        &self,
        user_id: Uuid,
        current_session_id: Uuid,
        actor_id: ActorId,
    ) -> Result<Option<i64>, IdentityError> {
        self.store
            .delete_other_sessions(user_id, current_session_id, self.clock.now(), actor_id)
            .await
    }

    pub async fn authenticate_bearer(&self, token: &str) -> Result<ActorPrincipal, IdentityError> {
        Ok(self.authenticate_bearer_context(token).await?.principal)
    }

    pub async fn authenticate_bearer_context(
        &self,
        token: &str,
    ) -> Result<AuthenticatedBearer, IdentityError> {
        if token.starts_with("cit_sa_") {
            return Ok(AuthenticatedBearer {
                principal: self.authenticate_service_account(token).await?,
                automation_run_id: None,
            });
        }
        let claims = self
            .tokens
            .decode_access(token)
            .map_err(|_| IdentityError::InvalidCredentials)?;
        if claims.expires_at <= self.clock.now() {
            return Err(IdentityError::InvalidCredentials);
        }
        if claims.principal_type == AuthenticatedPrincipalType::ServiceAccount
            && claims.automation_run_id.is_none()
        {
            return Err(IdentityError::InvalidCredentials);
        }
        let principal = self
            .store
            .load_principal(
                claims.actor_id,
                claims.subject_id,
                claims.principal_type,
                None,
            )
            .await?
            .ok_or(IdentityError::InvalidCredentials)?;
        Ok(AuthenticatedBearer {
            principal,
            automation_run_id: claims.automation_run_id,
        })
    }

    pub async fn ensure_run_as_allowed(
        &self,
        caller: &ActorPrincipal,
        run_as_actor_id: ActorId,
    ) -> Result<ActorPrincipal, IdentityError> {
        let target = self
            .store
            .load_run_as_principal(run_as_actor_id)
            .await?
            .ok_or_else(|| {
                IdentityError::Validation(
                    "The run-as identity is unavailable or disabled.".to_owned(),
                )
            })?;
        if target.principal_type == AuthenticatedPrincipalType::ServiceAccount
            && !self.entitlements.custom_access_control_enabled().await?
        {
            return Err(IdentityError::LicenseRequired("CustomAccessControl"));
        }
        if caller.is_administrator() || caller.actor_id == run_as_actor_id {
            return Ok(target);
        }
        if target.principal_type != AuthenticatedPrincipalType::ServiceAccount {
            return Err(IdentityError::Forbidden);
        }
        self.authorize_resource(
            caller,
            ResourceType::ServiceAccount,
            target.subject_id,
            PermissionLevel::Read,
            Some(SpecificPermission::Use),
        )
        .await?;
        Ok(target)
    }

    pub async fn execution_principal(
        &self,
        actor_id: ActorId,
    ) -> Result<ActorPrincipal, IdentityError> {
        let principal = self
            .store
            .load_run_as_principal(actor_id)
            .await?
            .ok_or_else(|| {
                IdentityError::Validation(
                    "The run-as identity is unavailable or disabled.".to_owned(),
                )
            })?;
        if principal.principal_type == AuthenticatedPrincipalType::ServiceAccount
            && !self.entitlements.custom_access_control_enabled().await?
        {
            return Err(IdentityError::LicenseRequired("CustomAccessControl"));
        }
        Ok(principal)
    }

    pub async fn issue_automation_access_token(
        &self,
        run_as_actor_id: ActorId,
        automation_run_id: Uuid,
        lifetime: Duration,
    ) -> Result<String, IdentityError> {
        let target = self
            .store
            .load_run_as_principal(run_as_actor_id)
            .await?
            .ok_or_else(|| {
                IdentityError::Validation(
                    "The run-as identity is unavailable or disabled.".to_owned(),
                )
            })?;
        if target.principal_type == AuthenticatedPrincipalType::ServiceAccount
            && !self.entitlements.custom_access_control_enabled().await?
        {
            return Err(IdentityError::LicenseRequired("CustomAccessControl"));
        }
        let issued_at = self.clock.now();
        self.tokens.encode_access(&AccessTokenClaims {
            subject_id: target.subject_id,
            actor_id: target.actor_id,
            principal_type: target.principal_type,
            issued_at,
            expires_at: issued_at + lifetime,
            automation_run_id: Some(automation_run_id),
        })
    }

    async fn authenticate_service_account(
        &self,
        token: &str,
    ) -> Result<ActorPrincipal, IdentityError> {
        if !self.entitlements.custom_access_control_enabled().await? {
            return Err(IdentityError::InvalidCredentials);
        }
        let (credential_id, supplied_hash) = self
            .service_account_tokens
            .parse_and_hash(token)
            .ok_or(IdentityError::InvalidCredentials)?;
        let credential = self
            .store
            .load_service_account_credential(credential_id)
            .await?
            .filter(|credential| service_account_credential_is_usable(credential, self.clock.now()))
            .ok_or(IdentityError::InvalidCredentials)?;
        if credential.secret_hash.ct_eq(&supplied_hash).unwrap_u8() != 1 {
            return Err(IdentityError::InvalidCredentials);
        }
        let principal = ActorPrincipal {
            subject_id: credential.service_account_id,
            actor_id: credential.actor_id,
            name: credential.name,
            principal_type: AuthenticatedPrincipalType::ServiceAccount,
            credential_id: Some(credential.credential_id),
            roles: credential.roles,
        };
        self.last_used_tracker
            .track(credential_id, self.clock.now());
        Ok(principal)
    }

    pub async fn authorize(
        &self,
        principal: &ActorPrincipal,
        resource_type: ResourceType,
        level: PermissionLevel,
        specific: Option<SpecificPermission>,
    ) -> Result<(), IdentityError> {
        let snapshot = self
            .store
            .authorization_snapshot(principal.actor_id)
            .await?;
        if snapshot.permits(resource_type, level, specific) {
            Ok(())
        } else {
            Err(IdentityError::Forbidden)
        }
    }

    pub async fn authorization_snapshot(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<AuthorizationSnapshot, IdentityError> {
        self.store.authorization_snapshot(principal.actor_id).await
    }

    pub async fn global_permission(
        &self,
        principal: &ActorPrincipal,
        resource_type: ResourceType,
    ) -> Result<Option<PermissionGrant>, IdentityError> {
        let snapshot = self
            .store
            .authorization_snapshot(principal.actor_id)
            .await?;
        Ok(snapshot
            .enabled
            .then(|| {
                snapshot
                    .direct_and_team_permissions
                    .into_iter()
                    .find(|grant| grant.resource_type == resource_type)
            })
            .flatten())
    }

    pub async fn permission_for_resource(
        &self,
        principal: &ActorPrincipal,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> Result<Option<PermissionGrant>, IdentityError> {
        self.store
            .resource_permission(principal.actor_id, resource_type, resource_id)
            .await
    }

    pub async fn authorize_resource(
        &self,
        principal: &ActorPrincipal,
        resource_type: ResourceType,
        resource_id: Uuid,
        level: PermissionLevel,
        specific: Option<SpecificPermission>,
    ) -> Result<(), IdentityError> {
        let permission = self
            .store
            .resource_permission(principal.actor_id, resource_type, resource_id)
            .await?;
        if permission.is_some_and(|grant| {
            grant.level.grants(level)
                && specific.is_none_or(|permission| grant.has_specific(permission))
        }) {
            Ok(())
        } else {
            Err(IdentityError::Forbidden)
        }
    }

    pub async fn issue_session(
        &self,
        user: &UserAuthentication,
        metadata: SessionMetadata,
    ) -> Result<SessionTokens, IdentityError> {
        let prepared = self.prepare_session(user, metadata)?;
        self.persist_prepared_session(&prepared).await?;
        Ok(prepared.tokens)
    }

    pub fn prepare_session(
        &self,
        user: &UserAuthentication,
        metadata: SessionMetadata,
    ) -> Result<PreparedSession, IdentityError> {
        let now = self.clock.now();
        let expires_at = now + self.refresh_token_lifetime;
        let session_id = Uuid::now_v7();
        let access_token = self.tokens.encode_access(&AccessTokenClaims {
            subject_id: user.user_id,
            actor_id: user.actor_id,
            principal_type: AuthenticatedPrincipalType::User,
            issued_at: now,
            expires_at: now + self.access_token_lifetime,
            automation_run_id: None,
        })?;
        let refresh_token = self.tokens.encode_refresh(&RefreshTokenClaims {
            session_id,
            issued_at: now,
            expires_at,
        })?;
        Ok(PreparedSession {
            session: NewSession {
                id: session_id,
                user_id: user.user_id,
                created_at: now,
                expires_at,
                metadata,
            },
            tokens: SessionTokens {
                access_token,
                refresh_token,
                refresh_expires_at: expires_at,
            },
            expected_password_hash: user.password_hash.clone(),
        })
    }

    pub async fn persist_prepared_session(
        &self,
        prepared: &PreparedSession,
    ) -> Result<(), IdentityError> {
        self.store
            .create_session(
                &prepared.session,
                prepared.expected_password_hash.as_deref(),
                MAXIMUM_SESSIONS_PER_USER,
            )
            .await
    }

    pub(crate) async fn hash_password(&self, password: String) -> Result<String, IdentityError> {
        let password = zeroize::Zeroizing::new(password);
        let permit = Arc::clone(&self.credential_workers)
            .acquire_owned()
            .await
            .map_err(|_| IdentityError::Credential)?;
        let passwords = Arc::clone(&self.passwords);
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            passwords.hash(&password)
        })
        .await
        .map_err(|_| IdentityError::Credential)?
    }

    pub(crate) async fn verify_password(
        &self,
        password: String,
        encoded_hash: String,
    ) -> Result<bool, IdentityError> {
        let permit = Arc::clone(&self.credential_workers)
            .acquire_owned()
            .await
            .map_err(|_| IdentityError::Credential)?;
        let passwords = Arc::clone(&self.passwords);
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            passwords.verify(&password, &encoded_hash)
        })
        .await
        .map_err(|_| IdentityError::Credential)
    }
}

fn service_account_credential_is_usable(
    credential: &ServiceAccountCredential,
    now: DateTime<Utc>,
) -> bool {
    credential.enabled
        && credential.revoked_at.is_none()
        && credential.archived_at.is_none()
        && credential
            .expires_at
            .is_none_or(|expires_at| expires_at > now)
}

pub fn validate_name(name: &str) -> Result<(), IdentityError> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > MAX_NAME_CHARS
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(IdentityError::Validation(
            "Name must use 1 to 100 letters, numbers, hyphens, or underscores.".to_owned(),
        ));
    }
    Ok(())
}

pub fn validate_email(email: &str) -> Result<(), IdentityError> {
    if email.trim().parse::<EmailAddress>().is_err() {
        return Err(IdentityError::Validation(
            "Email must be a valid address.".to_owned(),
        ));
    }
    Ok(())
}

pub fn validate_password(
    password: &str,
    user_name: Option<&str>,
    email: Option<&str>,
) -> Result<(), IdentityError> {
    let count = password.chars().count();
    if !(MINIMUM_PASSWORD_CHARACTERS..=MAXIMUM_PASSWORD_CHARACTERS).contains(&count) {
        return Err(IdentityError::Validation(format!(
            "Password must contain {MINIMUM_PASSWORD_CHARACTERS} to {MAXIMUM_PASSWORD_CHARACTERS} characters."
        )));
    }
    const BLOCKED: [&str; 7] = [
        "admin",
        "admin123",
        "password",
        "password123",
        "letmein",
        "citadel",
        "citadel123",
    ];
    if BLOCKED
        .iter()
        .any(|blocked| password.eq_ignore_ascii_case(blocked))
        || user_name.is_some_and(|name| password.eq_ignore_ascii_case(name))
        || email.is_some_and(|address| {
            password.eq_ignore_ascii_case(address)
                || address
                    .split_once('@')
                    .is_some_and(|(local, _)| password.eq_ignore_ascii_case(local))
        })
    {
        return Err(IdentityError::Validation(
            "Choose a less predictable password.".to_owned(),
        ));
    }
    Ok(())
}

#[must_use]
pub fn token_digest(secret: &[u8]) -> [u8; 32] {
    Sha256::digest(secret).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_policy_rejects_short_and_identity_values() {
        assert!(validate_password("short", None, None).is_err());
        assert!(
            validate_password(
                "owner@example.test",
                Some("owner"),
                Some("owner@example.test")
            )
            .is_err()
        );
        assert!(validate_password("correct-horse-battery-staple", None, None).is_ok());
    }

    #[test]
    fn service_account_status_prevents_suspended_credentials() {
        let now = Utc::now();
        let mut credential = ServiceAccountCredential {
            credential_id: Uuid::now_v7(),
            service_account_id: Uuid::now_v7(),
            actor_id: ActorId::new(Uuid::now_v7()),
            name: "ci".to_owned(),
            secret_hash: [0; 32],
            expires_at: None,
            revoked_at: None,
            archived_at: None,
            enabled: true,
            roles: Vec::new(),
        };
        assert!(service_account_credential_is_usable(&credential, now));
        credential.enabled = false;
        assert!(!service_account_credential_is_usable(&credential, now));
    }
}
