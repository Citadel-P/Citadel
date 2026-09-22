use crate::api::resources::{metadata_patch::MetadataPatch, registries::views::RegistryStatus};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewRegistry {
    pub name: String,
    /// Required for custom registry addresses; inferred for Docker Hub and GitHub.
    #[serde(default)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::resources::registries::examples;

    #[test]
    fn known_registry_hosts_are_inferred_when_omitted_from_requests() {
        for (mut payload, expected) in [
            (examples::dockerhub(), "docker.io"),
            (examples::github(), "ghcr.io"),
        ] {
            payload.as_object_mut().unwrap().remove("registryHost");
            let request: NewRegistry = serde_json::from_value(payload).unwrap();
            let mut input: citadel_registries::NewRegistry = request.into();
            input.validate().unwrap();
            assert_eq!(input.registry_host, expected);
        }
    }

    #[test]
    fn other_registry_providers_still_require_a_host() {
        for mut payload in [
            examples::custom(),
            examples::azure(),
            examples::aws(),
            examples::gitlab(),
        ] {
            payload.as_object_mut().unwrap().remove("registryHost");
            let request: NewRegistry = serde_json::from_value(payload).unwrap();
            let mut input: citadel_registries::NewRegistry = request.into();
            assert!(input.validate().is_err());
        }
    }

    #[test]
    fn registry_request_schema_allows_an_omitted_host() {
        let schema =
            serde_json::to_value(<NewRegistry as utoipa::PartialSchema>::schema()).unwrap();
        assert!(
            !schema["required"]
                .as_array()
                .unwrap()
                .contains(&Value::from("registryHost"))
        );
    }
}
