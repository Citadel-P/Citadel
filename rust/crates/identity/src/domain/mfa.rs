use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMfaSettings {
    pub user_id: Uuid,
    pub protected_totp_secret: String,
    pub last_accepted_time_step: Option<i64>,
    pub enabled_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMfaRecoveryCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MfaSetupSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub protected_totp_secret: String,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl MfaSetupSession {
    #[must_use]
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.consumed_at.is_none() && self.expires_at > now
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MfaChallenge {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub failed_attempts: i32,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl MfaChallenge {
    #[must_use]
    pub fn is_active(&self, now: DateTime<Utc>, maximum_failed_attempts: i32) -> bool {
        self.consumed_at.is_none()
            && self.expires_at > now
            && self.failed_attempts < maximum_failed_attempts
    }
}
