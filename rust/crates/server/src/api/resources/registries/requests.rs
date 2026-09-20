use crate::api::resources::{metadata_patch::MetadataPatch, registries::views::RegistryStatus};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;
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
