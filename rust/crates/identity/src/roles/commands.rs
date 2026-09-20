use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionInput {
    pub resource_type: ResourceType,
    pub permission_level: PermissionLevel,
    pub specific_permissions: Option<Vec<SpecificPermission>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRole {
    pub name: String,
    pub permissions: Option<Vec<RolePermissionInput>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchRolePermissions {
    #[serde(default)]
    pub permissions: PatchField<Vec<RolePermissionInput>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameRole {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRoles {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct NewRoleMutation {
    pub id: Uuid,
    pub name: String,
    pub permissions: Vec<RolePermissionDetails>,
    pub changed_by_actor_id: ActorId,
    pub changed_at: DateTime<Utc>,
}
