use crate::ActorPrincipal;
use crate::AuthorizationSnapshot;
use crate::Clock;
use crate::IdentityError;
use crate::IdentityService;
use crate::MAXIMUM_PASSWORD_CHARACTERS;
use crate::PatchField;
use crate::User;
use crate::UserSessionRecord;
use crate::validate_name;
use crate::validate_password;
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

pub use model::{
    UserAppearance, UserContentLayout, UserDateTimeFormat, UserPreferences, UserTheme,
    UserThemeColor, UserUiDensity, UserUiFont, UserUiRadius,
};

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
