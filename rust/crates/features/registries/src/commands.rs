use super::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRegistry {
    pub name: String,
    pub registry_host: String,
    pub status: RegistryStatus,
    pub configuration: Value,
    pub description: Option<String>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPatch {
    pub name: Option<String>,
    pub registry_host: Option<String>,
    pub status: Option<RegistryStatus>,
    #[serde(default)]
    pub configuration: PatchField<Value>,
    #[serde(default)]
    pub description: PatchField<String>,
    pub tag_ids: Option<Vec<Uuid>>,
}
