use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaggableResourceType {
    Deployment,
    Stack,
    Platform,
    GitRepository,
    Registry,
    AutomationAction,
    BackupPolicy,
    Build,
    BuildAgentPool,
    SwarmService,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub color: String,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub usage_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}
