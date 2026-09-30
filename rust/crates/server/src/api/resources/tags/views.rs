use crate::api::resources::platforms::views::ResourceCapabilitiesView;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TagsResponse {
    pub(crate) tags: Vec<AuthorizedTagView>,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorizedTagView {
    #[serde(flatten)]
    pub(crate) tag: TagView,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceTagsResponse {
    pub(crate) tags: Vec<TagSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagView {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub color: String,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub usage_count: i32,
}

impl From<TagView> for citadel_tags::Tag {
    fn from(value: TagView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            normalized_name: value.normalized_name,
            color: value.color,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            usage_count: value.usage_count,
        }
    }
}

impl From<citadel_tags::Tag> for TagView {
    fn from(value: citadel_tags::Tag) -> Self {
        Self {
            id: value.id,
            name: value.name,
            normalized_name: value.normalized_name,
            color: value.color,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            usage_count: value.usage_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = resources::tags::TagSummary)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

impl From<TagSummary> for citadel_tags::TagSummary {
    fn from(value: TagSummary) -> Self {
        Self {
            id: value.id,
            name: value.name,
            color: value.color,
        }
    }
}

impl From<citadel_tags::TagSummary> for TagSummary {
    fn from(value: citadel_tags::TagSummary) -> Self {
        Self {
            id: value.id,
            name: value.name,
            color: value.color,
        }
    }
}
