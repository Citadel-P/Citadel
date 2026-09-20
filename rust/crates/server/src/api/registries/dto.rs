use crate::api::metadata_patch::MetadataPatch;

use crate::api::tags::dto::TagSummary;

use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};

use serde_json::Value;

use uuid::Uuid;

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

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewRegistry {
    pub name: String,
    pub registry_host: String,
    pub status: RegistryStatus,
    pub configuration: Value,
    pub description: Option<String>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl From<NewRegistry> for citadel_registries::NewRegistry {
    fn from(value: NewRegistry) -> Self {
        Self {
            name: value.name,
            registry_host: value.registry_host,
            status: value.status.into(),
            configuration: value.configuration,
            description: value.description,
            tag_ids: value.tag_ids,
        }
    }
}

impl From<citadel_registries::NewRegistry> for NewRegistry {
    fn from(value: citadel_registries::NewRegistry) -> Self {
        Self {
            name: value.name,
            registry_host: value.registry_host,
            status: value.status.into(),
            configuration: value.configuration,
            description: value.description,
            tag_ids: value.tag_ids,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPatch {
    pub name: Option<String>,
    pub registry_host: Option<String>,
    pub status: Option<RegistryStatus>,
    #[serde(default)]
    #[schema(value_type = Option<Value>, required = false)]
    pub configuration: MetadataPatch<Value>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub description: MetadataPatch<String>,
    pub tag_ids: Option<Vec<Uuid>>,
}

impl From<RegistryPatch> for citadel_registries::RegistryPatch {
    fn from(value: RegistryPatch) -> Self {
        Self {
            name: value.name,
            registry_host: value.registry_host,
            status: value.status.map(|item| item.into()),
            configuration: value.configuration.into(),
            description: value.description.into(),
            tag_ids: value.tag_ids,
        }
    }
}

impl From<citadel_registries::RegistryPatch> for RegistryPatch {
    fn from(value: citadel_registries::RegistryPatch) -> Self {
        Self {
            name: value.name,
            registry_host: value.registry_host,
            status: value.status.map(|item| item.into()),
            configuration: value.configuration.into(),
            description: value.description.into(),
            tag_ids: value.tag_ids,
        }
    }
}
