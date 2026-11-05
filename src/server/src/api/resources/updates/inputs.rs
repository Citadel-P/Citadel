use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewUpdateInput {
    #[serde(default)]
    pub database: bool,
    pub version: String,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApproveUpdateInput {
    pub review_id: Uuid,
    pub request_id: Uuid,
}
