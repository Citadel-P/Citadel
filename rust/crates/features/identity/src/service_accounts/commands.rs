use super::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceAccount {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "enabled_by_default")]
    pub is_enabled: bool,
    #[serde(default)]
    pub team_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
    #[serde(default)]
    pub resource_accesses: Vec<ServiceAccountResourceAccess>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateServiceAccount {
    #[serde(default)]
    pub description: PatchField<String>,
    #[serde(default)]
    pub is_enabled: PatchField<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameServiceAccount {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddServiceAccountRole {
    pub role_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddServiceAccountResourceAccess {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveServiceAccounts {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceAccountToken {
    pub name: String,
    pub expires_at_utc: Option<DateTime<Utc>>,
    #[serde(default)]
    pub never_expires: bool,
}

#[derive(Debug, Clone)]
pub struct NewServiceAccount {
    pub id: Uuid,
    pub actor_id: ActorId,
    pub name: String,
    pub description: Option<String>,
    pub is_enabled: bool,
    pub created_by_actor_id: ActorId,
    pub created_at: DateTime<Utc>,
    pub team_ids: Vec<Uuid>,
    pub role_ids: Vec<Uuid>,
    pub resource_accesses: Vec<ServiceAccountResourceAccess>,
}

#[derive(Debug, Clone)]
pub struct NewServiceAccountToken {
    pub id: Uuid,
    pub service_account_id: Uuid,
    pub name: String,
    pub secret_hash: [u8; 32],
    pub expires_at_utc: Option<DateTime<Utc>>,
    pub created_by_actor_id: ActorId,
    pub created_at_utc: DateTime<Utc>,
}
