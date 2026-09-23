use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformActivitySnapshot {
    #[serde(rename = "Id")]
    pub id: Uuid,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Address")]
    pub address: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "ConnectorType")]
    pub connector_type: String,
    #[serde(rename = "NetworkCount")]
    pub network_count: i32,
    #[serde(rename = "VolumeCount")]
    pub volume_count: i32,
    #[serde(rename = "ImageCount")]
    pub image_count: i64,
    #[serde(rename = "CpuCount")]
    pub cpu_count: i64,
    #[serde(rename = "MemTotal")]
    pub mem_total: i64,
    #[serde(rename = "ServerVersion")]
    pub server_version: Option<String>,
    #[serde(rename = "AgentVersion")]
    pub agent_version: Option<String>,
    #[serde(rename = "PlatformDescriptor")]
    pub platform_descriptor: Value,
}
