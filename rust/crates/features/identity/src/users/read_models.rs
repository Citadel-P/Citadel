use super::*;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDetails {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub teams: Option<Vec<ResourceInfo>>,
    pub roles: Option<Vec<ResourceInfo>>,
    pub resource_accesses: Option<Vec<UserResourceAccessDetails>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchItemDetails {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResourceAccessDetails {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub resource_name: Option<String>,
    pub permission_level: PermissionLevel,
    pub specific_permissions: Option<Vec<SpecificPermission>>,
    pub id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct UserPasswordContext {
    pub name: String,
    pub email: String,
}
