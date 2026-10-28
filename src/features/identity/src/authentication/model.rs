use super::*;
use citadel_primitives::ActorId;
use serde::Serialize;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAuthentication {
    pub user_id: Uuid,
    pub actor_id: ActorId,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub enabled: bool,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SetupInitializationMode {
    Interactive,
    Unattended,
}

#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PreparedSession {
    pub session: NewSession,
    pub tokens: SessionTokens,
    pub expected_password_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    #[serde(default)]
    pub session_id: Option<Uuid>,
    pub subject_id: Uuid,
    pub actor_id: ActorId,
    pub principal_type: AuthenticatedPrincipalType,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub automation_run_id: Option<Uuid>,
    #[serde(default)]
    pub automation_attempt_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub session_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Stored once for the installation, independently of personal preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallationSettings {
    pub initialized: bool,
    pub update_checks_enabled: bool,
    pub mfa_policy: crate::MfaPolicy,
}

/// Deployment restrictions remain authoritative over database choices.
#[derive(Debug, Clone, Copy)]
pub struct InstallationPolicy {
    pub update_checks_allowed: bool,
    pub minimum_mfa_policy: crate::MfaPolicy,
}

impl Default for InstallationPolicy {
    fn default() -> Self {
        Self {
            update_checks_allowed: true,
            minimum_mfa_policy: crate::MfaPolicy::Optional,
        }
    }
}
