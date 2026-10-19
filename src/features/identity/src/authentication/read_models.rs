use super::*;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginOutcome {
    pub access_token: Option<String>,
    pub next_step: LoginNextStep,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub requires_setup: bool,
    pub password_minimum_length: usize,
    pub password_maximum_length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LoginNextStep {
    Completed,
    VerifyMfa,
    EnrollMfa,
}

#[derive(Clone)]
pub struct SessionTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_expires_at: DateTime<Utc>,
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

#[derive(Debug, Clone)]
pub struct AuthenticatedBearer {
    pub principal: ActorPrincipal,
    pub automation_run_id: Option<Uuid>,
}

impl std::fmt::Debug for LoginOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginOutcome")
            .field("access_token", &"[REDACTED]")
            .field("next_step", &self.next_step)
            .finish()
    }
}

impl std::fmt::Debug for SessionTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionTokens")
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .field("refresh_expires_at", &self.refresh_expires_at)
            .finish()
    }
}
