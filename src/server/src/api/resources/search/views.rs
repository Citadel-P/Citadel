use citadel_primitives::ResourceType;
use serde::Serialize;
use uuid::Uuid;
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSearchParent {
    pub id: Uuid,
    #[schema(value_type = GlobalSearchResourceType)]
    pub resource_type: ResourceType,
    pub name: String,
}

impl From<GlobalSearchParent> for citadel_discovery::GlobalSearchParent {
    fn from(value: GlobalSearchParent) -> Self {
        Self {
            id: value.id,
            resource_type: value.resource_type,
            name: value.name,
        }
    }
}

impl From<citadel_discovery::GlobalSearchParent> for GlobalSearchParent {
    fn from(value: citadel_discovery::GlobalSearchParent) -> Self {
        Self {
            id: value.id,
            resource_type: value.resource_type,
            name: value.name,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GlobalSearchStatus {
    pub label: String,
    #[schema(value_type = SearchStatusTone)]
    pub tone: &'static str,
}

impl From<GlobalSearchStatus> for citadel_discovery::GlobalSearchStatus {
    fn from(value: GlobalSearchStatus) -> Self {
        Self {
            label: value.label,
            tone: value.tone,
        }
    }
}

impl From<citadel_discovery::GlobalSearchStatus> for GlobalSearchStatus {
    fn from(value: citadel_discovery::GlobalSearchStatus) -> Self {
        Self {
            label: value.label,
            tone: value.tone,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSearchItem {
    pub id: Uuid,
    #[schema(value_type = GlobalSearchResourceType)]
    pub resource_type: ResourceType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<GlobalSearchStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<GlobalSearchParent>,
}

impl From<GlobalSearchItem> for citadel_discovery::GlobalSearchItem {
    fn from(value: GlobalSearchItem) -> Self {
        Self {
            id: value.id,
            resource_type: value.resource_type,
            name: value.name,
            secondary_text: value.secondary_text,
            status: value.status.map(|item| item.into()),
            parent: value.parent.map(|item| item.into()),
        }
    }
}

impl From<citadel_discovery::GlobalSearchItem> for GlobalSearchItem {
    fn from(value: citadel_discovery::GlobalSearchItem) -> Self {
        Self {
            id: value.id,
            resource_type: value.resource_type,
            name: value.name,
            secondary_text: value.secondary_text,
            status: value.status.map(|item| item.into()),
            parent: value.parent.map(|item| item.into()),
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GlobalSearchGroup {
    #[schema(value_type = GlobalSearchCategory)]
    pub category: &'static str,
    pub items: Vec<GlobalSearchItem>,
}

impl From<GlobalSearchGroup> for citadel_discovery::GlobalSearchGroup {
    fn from(value: GlobalSearchGroup) -> Self {
        Self {
            category: value.category,
            items: value.items.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<citadel_discovery::GlobalSearchGroup> for GlobalSearchGroup {
    fn from(value: citadel_discovery::GlobalSearchGroup) -> Self {
        Self {
            category: value.category,
            items: value.items.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GlobalSearchResponse {
    pub query: String,
    pub groups: Vec<GlobalSearchGroup>,
}

impl From<GlobalSearchResponse> for citadel_discovery::GlobalSearchResults {
    fn from(value: GlobalSearchResponse) -> Self {
        Self {
            query: value.query,
            groups: value.groups.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<citadel_discovery::GlobalSearchResults> for GlobalSearchResponse {
    fn from(value: citadel_discovery::GlobalSearchResults) -> Self {
        Self {
            query: value.query,
            groups: value.groups.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub enum SearchStatusTone {
    Positive,
    Negative,
    Warning,
    Info,
    Neutral,
}

#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub enum GlobalSearchCategory {
    Platforms,
    Stacks,
    Deployments,
    Repositories,
    Registries,
    Automations,
    Backups,
    Builds,
    SwarmServices,
}

pub struct GlobalSearchResourceType;
impl utoipa::PartialSchema for GlobalSearchResourceType {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .enum_values(Some(
                citadel_discovery::search::supported_resource_types()
                    .map(|kind| serde_json::to_value(kind).expect("resource type serializes")),
            ))
            .into()
    }
}
impl utoipa::ToSchema for GlobalSearchResourceType {}
