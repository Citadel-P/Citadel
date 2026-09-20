use crate::{
    Clock, EntitlementService, IdentityError, PatchField, permission_matrix, validate_name,
};
use chrono::{DateTime, Utc};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};

use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use uuid::Uuid;

mod read_models;
pub use read_models::{RoleDetails, RolePermissionDetails};
mod commands;
pub use commands::{
    CreateRole, DeleteRoles, NewRoleMutation, PatchRolePermissions, RenameRole, RolePermissionInput,
};
mod repository;
pub use repository::{RoleReader, RoleRepository};
mod service;
pub use service::{RoleMutationService, RoleReadService, role_permissions_expand};

pub mod model;
pub use model::{ADMIN_ROLE_ID, Role, RoleMutationError, RolePermission, RoleType};
