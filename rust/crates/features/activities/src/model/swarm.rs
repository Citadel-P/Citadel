use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SwarmServiceActivitySnapshot {
    #[serde(rename = "Id")]
    pub id: Uuid,
    #[serde(rename = "PlatformId")]
    pub platform_id: Uuid,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "DockerName")]
    pub docker_name: String,
    #[serde(rename = "DockerServiceId")]
    pub docker_service_id: Option<String>,
    #[serde(rename = "Spec")]
    pub spec: Value,
}
