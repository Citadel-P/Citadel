use citadel_activities::{ActivityEventType, ActivityFilter, ActivityResourceType};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivityFilterQuery {
    #[serde(alias = "ResourceId")]
    pub(crate) resource_id: Option<Uuid>,
    #[serde(alias = "ResourceType")]
    pub(crate) resource_type: Option<ActivityResourceType>,
    #[serde(alias = "EventType")]
    pub(crate) event_type: Option<ActivityEventType>,
    #[serde(alias = "Page")]
    pub(crate) page: Option<i32>,
    #[serde(alias = "PageSize")]
    pub(crate) page_size: Option<i32>,
}

impl From<ActivityFilterQuery> for ActivityFilter {
    fn from(value: ActivityFilterQuery) -> Self {
        Self {
            resource_id: value.resource_id,
            resource_type: value.resource_type,
            event_type: value.event_type,
            page: value.page,
            page_size: value.page_size,
        }
    }
}
