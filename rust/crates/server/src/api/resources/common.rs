use serde::Serialize;
use uuid::Uuid;

pub(crate) fn enabled_by_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInfo {
    pub id: Uuid,
    pub name: String,
    #[schema(required = true)]
    pub group: Option<String>,
}

impl From<ResourceInfo> for citadel_identity::ResourceInfo {
    fn from(value: ResourceInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
            group: value.group,
        }
    }
}

impl From<citadel_identity::ResourceInfo> for ResourceInfo {
    fn from(value: citadel_identity::ResourceInfo) -> Self {
        Self {
            id: value.id,
            name: value.name,
            group: value.group,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

impl<T, V: From<T>> From<citadel_identity::PagedResult<T>> for PagedResult<V> {
    fn from(value: citadel_identity::PagedResult<T>) -> Self {
        Self {
            items: value.items.into_iter().map(V::from).collect(),
            total_count: value.total_count,
            page: value.page,
            page_size: value.page_size,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateSourceInput {
    #[schema(value_type = crate::api::resources::vocabulary::ActivityResourceTypeSchema)]
    pub resource_type: citadel_activities::ActivityResourceType,
    pub resource_id: Uuid,
    pub resource_name: String,
}
