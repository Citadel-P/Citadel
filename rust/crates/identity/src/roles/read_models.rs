use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionDetails {
    pub resource_type: ResourceType,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleDetails {
    pub id: Uuid,
    pub name: String,
    pub role_type: RoleType,
    pub permissions: Vec<RolePermissionDetails>,
}
