use crate::{
    Clock, EntitlementService, IdentityError, PagedResult, PasswordHasher, PatchField,
    ResourceInfo, StoredPage, permission_matrix, validate_email, validate_name, validate_password,
};
use chrono::{DateTime, Utc};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

mod read_models;

mod commands;

mod repository;
pub use repository::{UserReader, UserRepository};
mod service;

pub mod model;

use service::enabled_by_default;

pub use model::User;

pub use commands::{
    AddUserRole, CreateUser, DeleteUsers, NewUserMutation, PatchUser, RenameUser,
    UserPatchMutation, UserResourceAccess, UserResourceAccessInput,
};

pub use read_models::{
    UserDetails, UserPasswordContext, UserResourceAccessDetails, UserSearchItemDetails,
};

pub use service::{UserMutationService, UserReadService};
