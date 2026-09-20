use crate::api::resources::common::enabled_by_default;
use citadel_identity::{PatchField, UserDateTimeFormat, UserTheme};
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsersFilter {
    #[serde(alias = "Page")]
    #[serde(default = "first_page")]
    pub(crate) page: i64,
    #[serde(alias = "PageSize")]
    #[serde(default = "default_page_size")]
    pub(crate) page_size: i64,
    #[serde(alias = "Name")]
    pub(crate) name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UserSearchFilter {
    #[serde(alias = "Query")]
    pub(crate) query: String,
    #[serde(alias = "Limit")]
    #[serde(default = "default_search_limit")]
    pub(crate) limit: i64,
}

const fn default_page_size() -> i64 {
    50
}

const fn first_page() -> i64 {
    1
}

const fn default_search_limit() -> i64 {
    20
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchUserPreferencesRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub time_zone: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<crate::api::resources::vocabulary::UserDateTimeFormatSchema>, required = false)]
    pub date_time_format: PatchField<UserDateTimeFormat>,
    #[serde(default)]
    #[schema(value_type = Option<crate::api::resources::vocabulary::UserThemeSchema>, required = false)]
    pub theme: PatchField<UserTheme>,
}

impl From<PatchUserPreferencesRequest> for citadel_identity::PatchUserPreferences {
    fn from(value: PatchUserPreferencesRequest) -> Self {
        Self {
            time_zone: value.time_zone,
            date_time_format: value.date_time_format,
            theme: value.theme,
        }
    }
}

impl From<citadel_identity::PatchUserPreferences> for PatchUserPreferencesRequest {
    fn from(value: citadel_identity::PatchUserPreferences) -> Self {
        Self {
            time_zone: value.time_zone,
            date_time_format: value.date_time_format,
            theme: value.theme,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResourceAccessInput {
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<UserResourceAccessInput> for citadel_identity::UserResourceAccessInput {
    fn from(value: UserResourceAccessInput) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::UserResourceAccessInput> for UserResourceAccessInput {
    fn from(value: citadel_identity::UserResourceAccessInput) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(default = "enabled_by_default")]
    pub is_enabled: bool,
    #[serde(default)]
    pub team_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default)]
    pub resource_accesses: Vec<UserResourceAccessInput>,
}

impl From<CreateUserRequest> for citadel_identity::CreateUser {
    fn from(value: CreateUserRequest) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::CreateUser> for CreateUserRequest {
    fn from(value: citadel_identity::CreateUser) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchUserRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub email: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub password: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub is_enabled: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<Uuid>>, required = false)]
    pub team_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<Uuid>>, required = false)]
    pub role_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<UserResourceAccessInput>>, required = false)]
    pub resource_accesses: PatchField<Vec<UserResourceAccessInput>>,
}

impl From<PatchUserRequest> for citadel_identity::PatchUser {
    fn from(value: PatchUserRequest) -> Self {
        Self {
            email: value.email,
            password: value.password,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::PatchUser> for PatchUserRequest {
    fn from(value: citadel_identity::PatchUser) -> Self {
        Self {
            email: value.email,
            password: value.password,
            is_enabled: value.is_enabled,
            team_ids: value.team_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameUserRequest {
    pub id: Uuid,
    pub name: String,
}

impl From<RenameUserRequest> for citadel_identity::RenameUser {
    fn from(value: RenameUserRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::RenameUser> for RenameUserRequest {
    fn from(value: citadel_identity::RenameUser) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddUserRoleRequest {
    pub role_id: Uuid,
}

impl From<AddUserRoleRequest> for citadel_identity::AddUserRole {
    fn from(value: AddUserRoleRequest) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

impl From<citadel_identity::AddUserRole> for AddUserRoleRequest {
    fn from(value: citadel_identity::AddUserRole) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResourceAccessRequest {
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<UserResourceAccessRequest> for citadel_identity::UserResourceAccess {
    fn from(value: UserResourceAccessRequest) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::UserResourceAccess> for UserResourceAccessRequest {
    fn from(value: citadel_identity::UserResourceAccess) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUsersRequest {
    pub ids: Vec<Uuid>,
}

impl From<DeleteUsersRequest> for citadel_identity::DeleteUsers {
    fn from(value: DeleteUsersRequest) -> Self {
        Self { ids: value.ids }
    }
}

impl From<citadel_identity::DeleteUsers> for DeleteUsersRequest {
    fn from(value: citadel_identity::DeleteUsers) -> Self {
        Self { ids: value.ids }
    }
}
