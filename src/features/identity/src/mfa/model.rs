use super::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

#[derive(Clone, PartialEq, Eq)]
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

#[derive(Clone, PartialEq, Eq)]
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

macro_rules! database_string_enum {
    ($(#[$metadata:meta])* pub enum $name:ident { $($(#[$variant_metadata:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$metadata])*
        pub enum $name {
            $($(#[$variant_metadata])* $variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_database_str(self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant)),+
                }
            }

            #[must_use]
            pub fn from_database_str(value: &str) -> Option<Self> {
                match value {
                    $(stringify!($variant) => Some(Self::$variant)),+,
                    _ => None,
                }
            }
        }
    };
}

database_string_enum! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum MfaPolicy {
        #[default]
        Optional,
        RequiredForAdministrators,
        RequiredForAllUsers,
    }
}

impl MfaPolicy {
    #[must_use]
    pub const fn at_least(self, minimum: Self) -> Self {
        match (self, minimum) {
            (Self::RequiredForAllUsers, _) | (_, Self::RequiredForAllUsers) => {
                Self::RequiredForAllUsers
            }
            (Self::RequiredForAdministrators, _) | (_, Self::RequiredForAdministrators) => {
                Self::RequiredForAdministrators
            }
            _ => Self::Optional,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MfaConfiguration {
    pub policy: MfaPolicy,
    pub challenge_lifetime: Duration,
    pub setup_lifetime: Duration,
    pub maximum_failed_attempts: i32,
    pub recovery_code_count: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub struct TotpSetup {
    pub secret: String,
    pub otp_auth_uri: String,
    pub expires_at: DateTime<Utc>,
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

impl std::fmt::Debug for TotpSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TotpSetup")
            .field("expires_at", &self.expires_at)
            .field("secret", &"[REDACTED]")
            .field("otp_auth_uri", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for MfaSetupSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MfaSetupSession")
            .field("user_id", &self.user_id)
            .field("expires_at", &self.expires_at)
            .field("id", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for MfaChallenge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MfaChallenge")
            .field("user_id", &self.user_id)
            .field("expires_at", &self.expires_at)
            .field("id", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}
