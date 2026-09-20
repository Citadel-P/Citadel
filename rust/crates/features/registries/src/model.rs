use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryStatus {
    Active,
    Disabled,
    Deprecated,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryDetails {
    pub id: Uuid,
    pub created_by_actor_id: Uuid,
    pub name: String,
    pub status: RegistryStatus,
    pub description: Option<String>,
    pub registry_host: String,
    #[serde(rename = "type")]
    pub registry_type: String,
    #[serde(skip_serializing)]
    pub configuration: Value,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<TagSummary>,
}
