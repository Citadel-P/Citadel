use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivitySourceResource {
    #[serde(rename = "ResourceType")]
    pub resource_type: ActivityResourceType,
    #[serde(rename = "ResourceId")]
    pub resource_id: Uuid,
    #[serde(rename = "ResourceName")]
    pub resource_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "PascalCase")]
pub struct VolumeContentDownloaded {
    pub volume_name: String,
    pub path: String,
    pub is_directory: bool,
    pub file_name: String,
}
