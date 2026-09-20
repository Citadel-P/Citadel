use crate::{
    Clock, EntitlementService, IdentityError, PagedResult, PatchField, ResourceInfo,
    ServiceAccountTokenCodec, StoredPage, permission_matrix, validate_name,
};
use chrono::{DateTime, Duration, Utc};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

mod read_models;

mod commands;

mod repository;
pub use repository::ServiceAccountRepository;
mod service;

pub mod model;

use service::enabled_by_default;

pub use model::{ServiceAccountCredential, ServiceAccountResourceAccess};

pub use commands::{
    AddServiceAccountResourceAccess, AddServiceAccountRole, ArchiveServiceAccounts,
    CreateServiceAccount, CreateServiceAccountToken, NewServiceAccount, NewServiceAccountToken,
    RenameServiceAccount, UpdateServiceAccount,
};

pub use read_models::{
    CreatedServiceAccountTokenDetails, RunAsActorUsageDetails, ServiceAccountDetails,
    ServiceAccountLimitsDetails, ServiceAccountTokenDetails,
};

pub use service::{
    DEFAULT_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS, MAXIMUM_ACTIVE_SERVICE_ACCOUNT_TOKENS,
    MAXIMUM_SERVICE_ACCOUNT_DESCRIPTION_CHARS, MAXIMUM_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS,
    ServiceAccountService,
};

pub mod usage;
pub use usage::{
    NoopServiceAccountLastUsedTracker, ServiceAccountLastUsedStore, ServiceAccountLastUsedTracker,
};
