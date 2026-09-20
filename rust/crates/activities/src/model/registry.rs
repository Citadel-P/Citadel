use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegistryActivitySnapshot {
    #[serde(rename = "Id")]
    pub id: Uuid,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "RegistryHost")]
    pub registry_host: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "Configuration")]
    pub configuration: Value,
}
