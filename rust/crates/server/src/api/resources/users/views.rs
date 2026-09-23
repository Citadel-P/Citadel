use crate::api::resources::{
    capabilities::ResourceCapabilities,
    common::{PagedResult, ResourceInfo},
};
use citadel_identity::{UserDateTimeFormat, UserTheme};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsersResponse {
    pub(crate) paged_result: PagedResult<UserView>,
    pub(crate) capabilities: ResourceCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferencesView {
    pub time_zone: Option<String>,
    #[schema(value_type = crate::api::resources::vocabulary::UserDateTimeFormatSchema)]
    pub date_time_format: UserDateTimeFormat,
    #[schema(value_type = crate::api::resources::vocabulary::UserThemeSchema)]
    pub theme: UserTheme,
    pub is_persisted: bool,
}

impl From<UserPreferencesView> for citadel_identity::UserPreferencesDetails {
    fn from(value: UserPreferencesView) -> Self {
        Self {
            time_zone: value.time_zone,
            date_time_format: value.date_time_format,
            theme: value.theme,
            is_persisted: value.is_persisted,
        }
    }
}

impl From<citadel_identity::UserPreferencesDetails> for UserPreferencesView {
    fn from(value: citadel_identity::UserPreferencesDetails) -> Self {
        Self {
            time_zone: value.time_zone,
            date_time_format: value.date_time_format,
            theme: value.theme,
            is_persisted: value.is_persisted,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[schema(value_type = crate::api::resources::vocabulary::ActorIdSchema)]
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub teams: Option<Vec<ResourceInfo>>,
    pub roles: Option<Vec<ResourceInfo>>,
    pub resource_accesses: Option<Vec<UserResourceAccessView>>,
}

impl From<UserView> for citadel_identity::UserDetails {
    fn from(value: UserView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            email: value.email,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            teams: value
                .teams
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            roles: value
                .roles
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::UserDetails> for UserView {
    fn from(value: citadel_identity::UserDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            email: value.email,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            teams: value
                .teams
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            roles: value
                .roles
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchItemView {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

impl From<UserSearchItemView> for citadel_identity::UserSearchItemDetails {
    fn from(value: UserSearchItemView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            email: value.email,
        }
    }
}

impl From<citadel_identity::UserSearchItemDetails> for UserSearchItemView {
    fn from(value: citadel_identity::UserSearchItemDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            email: value.email,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResourceAccessView {
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub resource_name: Option<String>,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[schema(value_type = Option<Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>>)]
    pub specific_permissions: Option<Vec<SpecificPermission>>,
    pub id: Option<Uuid>,
}

impl From<UserResourceAccessView> for citadel_identity::UserResourceAccessDetails {
    fn from(value: UserResourceAccessView) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
            id: value.id,
        }
    }
}

impl From<citadel_identity::UserResourceAccessDetails> for UserResourceAccessView {
    fn from(value: citadel_identity::UserResourceAccessDetails) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
            id: value.id,
        }
    }
}
