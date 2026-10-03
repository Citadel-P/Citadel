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
    pub subject_id: Uuid,
    pub actor_id: ActorId,
    pub principal_type: AuthenticatedPrincipalType,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub automation_run_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub session_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
