use super::*;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUser {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(default = "enabled_by_default")]
    pub is_enabled: bool,
    #[serde(default)]
    pub team_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default)]
    pub resource_accesses: Vec<ResourceAccessInput>,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchUser {
    #[serde(default)]
    pub email: PatchField<String>,
    #[serde(default)]
    pub password: PatchField<String>,
    #[serde(default)]
    pub is_enabled: PatchField<bool>,
    #[serde(default)]
    pub team_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    pub role_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    pub resource_accesses: PatchField<Vec<ResourceAccessInput>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameUser {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddUserRole {
    pub role_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUsers {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct NewUserMutation {
    pub id: Uuid,
    pub actor_id: ActorId,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub is_enabled: bool,
    pub created_by_actor_id: ActorId,
    pub created_at: DateTime<Utc>,
    pub team_ids: Vec<Uuid>,
    pub role_ids: Vec<Uuid>,
    pub resource_accesses: Vec<ResourceAccessInput>,
}

#[derive(Debug, Clone, Default)]
pub struct UserPatchMutation {
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub is_enabled: Option<bool>,
    pub team_ids: Option<Vec<Uuid>>,
    pub role_ids: Option<Vec<Uuid>>,
    pub resource_accesses: Option<Vec<ResourceAccessInput>>,
}

impl std::fmt::Debug for CreateUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateUser")
            .field("name", &self.name)
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .field("is_enabled", &self.is_enabled)
            .field("team_ids", &self.team_ids)
            .field("role_ids", &self.role_ids)
            .field("resource_accesses", &self.resource_accesses)
            .finish()
    }
}

impl std::fmt::Debug for PatchUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PatchUser")
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .field("is_enabled", &self.is_enabled)
            .field("team_ids", &self.team_ids)
            .field("role_ids", &self.role_ids)
            .field("resource_accesses", &self.resource_accesses)
            .finish()
    }
}
