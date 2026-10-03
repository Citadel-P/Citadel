use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub(crate) struct LookupQuery {
    #[serde(rename = "TargetResourceType", alias = "targetResourceType")]
    pub(crate) target: String,
    #[serde(rename = "SourceResourceType", alias = "sourceResourceType")]
    pub(crate) source: Option<String>,
    #[serde(rename = "SourceResourceId", alias = "sourceResourceId")]
    pub(crate) source_id: Option<Uuid>,
    #[serde(rename = "PlatformId", alias = "platformId")]
    pub(crate) platform_id: Option<Uuid>,
}
