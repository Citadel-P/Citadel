use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewImageDeletionInput {
    pub platform_id: Uuid,
    pub ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageUsageQuery {
    pub docker_node_id: Option<String>,
    #[serde(default)]
    pub offset: u32,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CheckImageUpdateInput {
    pub platform_id: Uuid,
    pub docker_image_id: String,
    pub docker_node_id: Option<String>,
    pub registry_id: Uuid,
    pub reference: String,
}
