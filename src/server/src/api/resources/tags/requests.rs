use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplaceResourceTagsInput {
    #[serde(default)]
    pub(crate) tag_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewTag {
    pub name: String,
    pub color: String,
}

impl From<NewTag> for citadel_tags::NewTag {
    fn from(value: NewTag) -> Self {
        Self {
            name: value.name,
            color: value.color,
        }
    }
}

impl From<citadel_tags::NewTag> for NewTag {
    fn from(value: citadel_tags::NewTag) -> Self {
        Self {
            name: value.name,
            color: value.color,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagPatch {
    pub name: Option<String>,
    pub color: Option<String>,
}

impl From<TagPatch> for citadel_tags::TagPatch {
    fn from(value: TagPatch) -> Self {
        Self {
            name: value.name,
            color: value.color,
        }
    }
}

impl From<citadel_tags::TagPatch> for TagPatch {
    fn from(value: citadel_tags::TagPatch) -> Self {
        Self {
            name: value.name,
            color: value.color,
        }
    }
}
