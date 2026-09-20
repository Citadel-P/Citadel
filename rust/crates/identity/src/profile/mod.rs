use crate::UserSessionRecord;
use crate::{
    ActorPrincipal, AuthorizationSnapshot, Clock, IdentityError, IdentityService,
    MAXIMUM_PASSWORD_CHARACTERS, PatchField, User, validate_name, validate_password,
};
use chrono::{DateTime, Utc};
use citadel_primitives::{PermissionLevel, ResourceType};

use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

mod read_models;

mod commands;

mod repository;
pub use repository::ProfileRepository;
mod service;

pub mod model;

pub use model::{UserDateTimeFormat, UserPreferences, UserTheme};

pub use commands::{
    ChangeCurrentPassword, PatchUserPreferences, UpdateCurrentProfile, UserPreferencesUpdate,
};

pub use read_models::{
    CurrentProfileAuthenticationDetails, CurrentProfileAuthenticationType,
    CurrentProfileAuthorizationDetails, CurrentProfileDetails, CurrentProfileRecord,
    PasswordChangeOutcome, ProfileResourceInfo, ResourceCapabilities,
    RevokeOtherProfileSessionsDetails, UserPreferencesDetails, UserSessionSummaryDetails,
    UserSessionsDetails,
};

pub use service::ProfileService;
