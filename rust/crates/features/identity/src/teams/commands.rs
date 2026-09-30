use super::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeam {
    pub name: String,
    #[serde(default)]
    pub user_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default)]
    pub resource_accesses: Vec<ResourceAccessInput>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchTeam {
    #[serde(default)]
    pub is_enabled: PatchField<bool>,
    #[serde(default)]
    pub user_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    pub role_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    pub resource_accesses: PatchField<Vec<ResourceAccessInput>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameTeam {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTeamRole {
    pub role_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTeamMember {
    pub member_actor_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTeams {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct NewTeamMutation {
    pub id: Uuid,
    pub actor_id: ActorId,
    pub name: String,
    pub created_by_actor_id: ActorId,
    pub created_at: DateTime<Utc>,
    pub user_ids: Vec<Uuid>,
    pub role_ids: Vec<Uuid>,
    pub resource_accesses: Vec<ResourceAccessInput>,
}

#[derive(Debug, Clone, Default)]
pub struct TeamPatchMutation {
    pub is_enabled: Option<bool>,
    pub user_ids: Option<Vec<Uuid>>,
    pub role_ids: Option<Vec<Uuid>>,
    pub resource_accesses: Option<Vec<ResourceAccessInput>>,
}
