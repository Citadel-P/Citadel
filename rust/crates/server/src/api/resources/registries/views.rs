use crate::api::resources::{platforms::views::ResourceCapabilitiesView, tags::views::TagSummary};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistriesResponse {
    pub(crate) registries: Vec<AuthorizedRegistryView>,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorizedRegistryView {
    #[serde(flatten)]
    pub(crate) registry: RegistryView,
    pub(crate) capabilities: ResourceCapabilitiesView,
    pub(crate) is_default: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistryConfigResponse {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) registry_host: String,
    pub(crate) status: crate::api::resources::registries::views::RegistryStatus,
    pub(crate) description: String,
    pub(crate) configuration: Value,
    pub(crate) tags: Vec<crate::api::resources::tags::views::TagSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum RegistryStatus {
    Active,
    Disabled,
    Deprecated,
}

impl From<RegistryStatus> for citadel_registries::RegistryStatus {
    fn from(value: RegistryStatus) -> Self {
        match value {
            RegistryStatus::Active => Self::Active,
            RegistryStatus::Disabled => Self::Disabled,
            RegistryStatus::Deprecated => Self::Deprecated,
        }
    }
}

impl From<citadel_registries::RegistryStatus> for RegistryStatus {
    fn from(value: citadel_registries::RegistryStatus) -> Self {
        match value {
            citadel_registries::RegistryStatus::Active => Self::Active,
            citadel_registries::RegistryStatus::Disabled => Self::Disabled,
            citadel_registries::RegistryStatus::Deprecated => Self::Deprecated,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryView {
    pub id: Uuid,
    pub created_by_actor_id: Uuid,
    pub name: String,
    pub status: RegistryStatus,
    pub description: Option<String>,
    pub registry_host: String,
    #[serde(rename = "type")]
    pub registry_type: String,
    #[serde(skip_serializing)]
    pub configuration: Value,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<TagSummary>,
}

impl From<RegistryView> for citadel_registries::RegistryDetails {
    fn from(value: RegistryView) -> Self {
        Self {
            id: value.id,
            created_by_actor_id: value.created_by_actor_id,
            name: value.name,
            status: value.status.into(),
            description: value.description,
            registry_host: value.registry_host,
            registry_type: value.registry_type,
            configuration: value.configuration,
            created_at: value.created_at,
            tags: value.tags.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<citadel_registries::RegistryDetails> for RegistryView {
    fn from(value: citadel_registries::RegistryDetails) -> Self {
        Self {
            id: value.id,
            created_by_actor_id: value.created_by_actor_id,
            name: value.name,
            status: value.status.into(),
            description: value.description,
            registry_host: value.registry_host,
            registry_type: value.registry_type,
            configuration: value.configuration,
            created_at: value.created_at,
            tags: value.tags.into_iter().map(|item| item.into()).collect(),
        }
    }
}
