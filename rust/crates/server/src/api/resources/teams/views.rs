use crate::api::resources::{
    capabilities::ResourceCapabilities,
    common::{PagedResult, ResourceInfo},
};
use citadel_identity::ActorType;
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TeamsResponse {
    pub(crate) paged_result: PagedResult<TeamView>,
    pub(crate) capabilities: ResourceCapabilities,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamView {
    pub id: Uuid,
    pub name: String,
    #[schema(value_type = crate::api::resources::vocabulary::ActorIdSchema)]
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub total_members: i32,
    #[schema(required = true)]
    pub users: Option<Vec<ResourceInfo>>,
    #[schema(required = true)]
    pub roles: Option<Vec<ResourceInfo>>,
    #[schema(required = true)]
    pub resource_accesses: Option<Vec<TeamResourceAccessView>>,
    #[schema(required = true)]
    pub members: Option<Vec<TeamMemberView>>,
}

impl From<TeamView> for citadel_identity::TeamDetails {
    fn from(value: TeamView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            total_members: value.total_members,
            users: value
                .users
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            roles: value
                .roles
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            members: value
                .members
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::TeamDetails> for TeamView {
    fn from(value: citadel_identity::TeamDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            actor_id: value.actor_id,
            is_enabled: value.is_enabled,
            total_members: value.total_members,
            users: value
                .users
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            roles: value
                .roles
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            members: value
                .members
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberView {
    #[schema(value_type = crate::api::resources::vocabulary::ActorIdSchema)]
    pub actor_id: ActorId,
    pub resource_id: Uuid,
    pub name: String,
    #[schema(value_type = crate::api::resources::vocabulary::ActorTypeSchema)]
    pub principal_type: ActorType,
}

impl From<TeamMemberView> for citadel_identity::TeamMemberDetails {
    fn from(value: TeamMemberView) -> Self {
        Self {
            actor_id: value.actor_id,
            resource_id: value.resource_id,
            name: value.name,
            principal_type: value.principal_type,
        }
    }
}

impl From<citadel_identity::TeamMemberDetails> for TeamMemberView {
    fn from(value: citadel_identity::TeamMemberDetails) -> Self {
        Self {
            actor_id: value.actor_id,
            resource_id: value.resource_id,
            name: value.name,
            principal_type: value.principal_type,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSearchItemView {
    pub id: Uuid,
    pub name: String,
}

impl From<TeamSearchItemView> for citadel_identity::TeamSearchItemDetails {
    fn from(value: TeamSearchItemView) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::TeamSearchItemDetails> for TeamSearchItemView {
    fn from(value: citadel_identity::TeamSearchItemDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamResourceAccessView {
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    #[schema(required = true)]
    pub resource_name: Option<String>,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[schema(value_type = Option<Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>>, required = true)]
    pub specific_permissions: Option<Vec<SpecificPermission>>,
    #[schema(required = true)]
    pub id: Option<Uuid>,
}

impl From<TeamResourceAccessView> for citadel_identity::TeamResourceAccessDetails {
    fn from(value: TeamResourceAccessView) -> Self {
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

impl From<citadel_identity::TeamResourceAccessDetails> for TeamResourceAccessView {
    fn from(value: citadel_identity::TeamResourceAccessDetails) -> Self {
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
