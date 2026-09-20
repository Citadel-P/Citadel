use super::*;
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceAccountCredential {
    pub credential_id: Uuid,
    pub service_account_id: Uuid,
    pub actor_id: ActorId,
    pub name: String,
    pub secret_hash: [u8; 32],
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountResourceAccess {
    pub id: Option<Uuid>,
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    #[serde(default)]
    pub resource_name: Option<String>,
    pub permission_level: PermissionLevel,
    #[serde(default)]
    pub specific_permissions: Vec<SpecificPermission>,
}
