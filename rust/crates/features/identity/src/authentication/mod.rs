use crate::ActorPrincipal;
use crate::AuthenticatedPrincipalType;
use crate::AuthorizationSnapshot;
use crate::IdentityError;
use crate::MAX_NAME_CHARS;
use crate::PermissionGrant;
use crate::SYSTEM_ACTOR_ID;
use crate::ServiceAccountCredential;
use crate::ServiceAccountLastUsedTracker;
use crate::User;
use chrono::{DateTime, Duration, Utc};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use email_address::EmailAddress;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;
use uuid::Uuid;

mod read_models;

mod commands;

mod repository;
pub use repository::{
    Clock, EntitlementService, IdentityStore, PasswordHasher, ServiceAccountTokenCodec,
    SessionTokenCodec,
};
mod service;

pub mod model;

pub use model::{
    AccessTokenClaims, PreparedSession, RefreshTokenClaims, SessionMetadata,
    SetupInitializationMode, UserAuthentication,
};

pub use commands::{InitializeCitadel, Login, NewSession};

pub use read_models::{
    AuthenticatedBearer, LoginNextStep, LoginOutcome, SessionTokens, SetupStatus, UserSessionRecord,
};

pub use service::{
    IdentityService, MAXIMUM_PASSWORD_CHARACTERS, MAXIMUM_SESSIONS_PER_USER,
    MINIMUM_PASSWORD_CHARACTERS, SystemClock, token_digest, validate_email, validate_name,
    validate_password,
};
