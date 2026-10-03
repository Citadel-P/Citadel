use super::*;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamDetails {
    pub id: Uuid,
    pub name: String,
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub total_members: i32,
    pub users: Option<Vec<ResourceInfo>>,
    pub roles: Option<Vec<ResourceInfo>>,
    pub resource_accesses: Option<Vec<ResourceAccessDetails>>,
    pub members: Option<Vec<TeamMemberDetails>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberDetails {
    pub actor_id: ActorId,
    pub resource_id: Uuid,
    pub name: String,
    pub principal_type: ActorType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSearchItemDetails {
    pub id: Uuid,
    pub name: String,
}
