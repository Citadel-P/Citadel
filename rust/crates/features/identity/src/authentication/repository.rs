use super::*;

pub trait IdentityStore: Send + Sync {
    fn setup_required(&self) -> BoxFuture<'_, Result<bool, IdentityError>>;

    fn initialize_administrator<'a>(
        &'a self,
        administrator: &'a User,
        mode: crate::SetupInitializationMode,
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
