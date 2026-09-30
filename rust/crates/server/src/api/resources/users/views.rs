use crate::api::resources::resource_access::ResourceAccessView;
use crate::api::resources::{
    capabilities::ResourceCapabilitiesView,
    common::{PagedResult, ResourceInfo},
};
use citadel_identity::{
    UserContentLayout, UserDateTimeFormat, UserTheme, UserThemeColor, UserUiDensity, UserUiFont,
    UserUiRadius,
};
use citadel_primitives::ActorId;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsersResponse {
    pub(crate) paged_result: PagedResult<UserView>,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferencesView {
    #[schema(required = true)]
    pub time_zone: Option<String>,
    #[schema(value_type = crate::api::resources::vocabulary::UserDateTimeFormatSchema)]
    pub date_time_format: UserDateTimeFormat,
    #[schema(value_type = crate::api::resources::vocabulary::UserThemeSchema)]
    pub theme: UserTheme,
    #[schema(value_type = crate::api::resources::vocabulary::UserThemeColorSchema)]
    pub theme_color: UserThemeColor,
    #[schema(value_type = crate::api::resources::vocabulary::UserUiFontSchema)]
    pub font: UserUiFont,
    #[schema(value_type = crate::api::resources::vocabulary::UserUiRadiusSchema)]
    pub radius: UserUiRadius,
    #[schema(value_type = crate::api::resources::vocabulary::UserContentLayoutSchema)]
    pub content_layout: UserContentLayout,
    #[schema(value_type = crate::api::resources::vocabulary::UserUiDensitySchema)]
    pub density: UserUiDensity,

    pub is_persisted: bool,
}

impl From<UserPreferencesView> for citadel_identity::UserPreferencesDetails {
    fn from(value: UserPreferencesView) -> Self {
        Self {
            time_zone: value.time_zone,
            date_time_format: value.date_time_format,
            theme: value.theme,
            theme_color: value.theme_color,
            font: value.font,
            radius: value.radius,
            content_layout: value.content_layout,
            density: value.density,

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
            theme_color: value.theme_color,
            font: value.font,
            radius: value.radius,
            content_layout: value.content_layout,
            density: value.density,

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
    #[schema(required = true)]
    pub teams: Option<Vec<ResourceInfo>>,
    #[schema(required = true)]
    pub roles: Option<Vec<ResourceInfo>>,
    #[schema(required = true)]
    pub resource_accesses: Option<Vec<ResourceAccessView>>,
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

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
