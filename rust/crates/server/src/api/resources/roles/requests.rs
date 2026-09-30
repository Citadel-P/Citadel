use citadel_primitives::PatchField;
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use serde::Deserialize;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionInput {
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[schema(value_type = Option<Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>>)]
    pub specific_permissions: Option<Vec<SpecificPermission>>,
}

impl From<RolePermissionInput> for citadel_identity::RolePermissionInput {
    fn from(value: RolePermissionInput) -> Self {
        Self {
            resource_type: value.resource_type,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::RolePermissionInput> for RolePermissionInput {
    fn from(value: citadel_identity::RolePermissionInput) -> Self {
        Self {
            resource_type: value.resource_type,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleRequest {
    pub name: String,
    pub permissions: Option<Vec<RolePermissionInput>>,
}

impl From<CreateRoleRequest> for citadel_identity::CreateRole {
    fn from(value: CreateRoleRequest) -> Self {
        Self {
            name: value.name,
            permissions: value
                .permissions
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::CreateRole> for CreateRoleRequest {
    fn from(value: citadel_identity::CreateRole) -> Self {
        Self {
            name: value.name,
            permissions: value
                .permissions
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchRolePermissionsRequest {
    #[serde(default)]
    #[schema(value_type = Option<Vec<RolePermissionInput>>, required = false)]
    pub permissions: PatchField<Vec<RolePermissionInput>>,
}

impl From<PatchRolePermissionsRequest> for citadel_identity::PatchRolePermissions {
    fn from(value: PatchRolePermissionsRequest) -> Self {
        Self {
            permissions: value
                .permissions
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

impl From<citadel_identity::PatchRolePermissions> for PatchRolePermissionsRequest {
    fn from(value: citadel_identity::PatchRolePermissions) -> Self {
        Self {
            permissions: value
                .permissions
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameRoleRequest {
    pub id: Uuid,
    pub name: String,
}

impl From<RenameRoleRequest> for citadel_identity::RenameRole {
    fn from(value: RenameRoleRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::RenameRole> for RenameRoleRequest {
    fn from(value: citadel_identity::RenameRole) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRolesRequest {
    pub ids: Vec<Uuid>,
}

impl From<DeleteRolesRequest> for citadel_identity::DeleteRoles {
    fn from(value: DeleteRolesRequest) -> Self {
        Self { ids: value.ids }
    }
}

impl From<citadel_identity::DeleteRoles> for DeleteRolesRequest {
    fn from(value: citadel_identity::DeleteRoles) -> Self {
        Self { ids: value.ids }
    }
}
