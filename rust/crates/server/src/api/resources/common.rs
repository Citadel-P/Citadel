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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[schema(as = identity::application::profile::ResourceCapabilities)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

impl From<ResourceCapabilities> for citadel_identity::ResourceCapabilities {
    fn from(value: ResourceCapabilities) -> Self {
        Self {
            can_read: value.can_read,
            can_write: value.can_write,
            can_execute: value.can_execute,
        }
    }
}

impl From<citadel_identity::ResourceCapabilities> for ResourceCapabilities {
    fn from(value: citadel_identity::ResourceCapabilities) -> Self {
        Self {
            can_read: value.can_read,
            can_write: value.can_write,
            can_execute: value.can_execute,
        }
    }
}
