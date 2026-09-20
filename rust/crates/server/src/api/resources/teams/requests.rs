use citadel_identity::PatchField;
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TeamsFilter {
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
pub(crate) struct TeamSearchFilter {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamResourceAccessInput {
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<TeamResourceAccessInput> for citadel_identity::TeamResourceAccessInput {
    fn from(value: TeamResourceAccessInput) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::TeamResourceAccessInput> for TeamResourceAccessInput {
    fn from(value: citadel_identity::TeamResourceAccessInput) -> Self {
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
pub struct CreateTeamRequest {
    pub name: String,
    #[serde(default)]
    pub user_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default)]
    pub resource_accesses: Vec<TeamResourceAccessInput>,
}

impl From<CreateTeamRequest> for citadel_identity::CreateTeam {
    fn from(value: CreateTeamRequest) -> Self {
        Self {
            name: value.name,
            user_ids: value.user_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::CreateTeam> for CreateTeamRequest {
    fn from(value: citadel_identity::CreateTeam) -> Self {
        Self {
            name: value.name,
            user_ids: value.user_ids,
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
pub struct PatchTeamRequest {
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub is_enabled: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<Uuid>>, required = false)]
    pub user_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<Uuid>>, required = false)]
    pub role_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<TeamResourceAccessInput>>, required = false)]
    pub resource_accesses: PatchField<Vec<TeamResourceAccessInput>>,
}

impl From<PatchTeamRequest> for citadel_identity::PatchTeam {
    fn from(value: PatchTeamRequest) -> Self {
        Self {
            is_enabled: value.is_enabled,
            user_ids: value.user_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::PatchTeam> for PatchTeamRequest {
    fn from(value: citadel_identity::PatchTeam) -> Self {
        Self {
            is_enabled: value.is_enabled,
            user_ids: value.user_ids,
            role_ids: value.role_ids,
            resource_accesses: value
                .resource_accesses
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameTeamRequest {
    pub id: Uuid,
    pub name: String,
}

impl From<RenameTeamRequest> for citadel_identity::RenameTeam {
    fn from(value: RenameTeamRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::RenameTeam> for RenameTeamRequest {
    fn from(value: citadel_identity::RenameTeam) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddTeamRoleRequest {
    pub role_id: Uuid,
}

impl From<AddTeamRoleRequest> for citadel_identity::AddTeamRole {
    fn from(value: AddTeamRoleRequest) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

impl From<citadel_identity::AddTeamRole> for AddTeamRoleRequest {
    fn from(value: citadel_identity::AddTeamRole) -> Self {
        Self {
            role_id: value.role_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddTeamMemberRequest {
    pub member_actor_id: Uuid,
}

impl From<AddTeamMemberRequest> for citadel_identity::AddTeamMember {
    fn from(value: AddTeamMemberRequest) -> Self {
        Self {
            member_actor_id: value.member_actor_id,
        }
    }
}

impl From<citadel_identity::AddTeamMember> for AddTeamMemberRequest {
    fn from(value: citadel_identity::AddTeamMember) -> Self {
        Self {
            member_actor_id: value.member_actor_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTeamsRequest {
    pub ids: Vec<Uuid>,
}

impl From<DeleteTeamsRequest> for citadel_identity::DeleteTeams {
    fn from(value: DeleteTeamsRequest) -> Self {
        Self { ids: value.ids }
    }
}

impl From<citadel_identity::DeleteTeams> for DeleteTeamsRequest {
    fn from(value: citadel_identity::DeleteTeams) -> Self {
        Self { ids: value.ids }
    }
}
