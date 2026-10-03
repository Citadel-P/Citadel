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
    pub resource_accesses: Option<Vec<ResourceAccessDetails>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchItemDetails {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct UserPasswordContext {
    pub name: String,
    pub email: String,
}
