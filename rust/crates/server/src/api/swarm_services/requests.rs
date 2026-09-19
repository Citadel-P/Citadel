use super::spec::*;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateSwarmServiceInput {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    #[schema(nullable)]
    pub tag_ids: Vec<Uuid>,
    #[serde(default)]
    pub duplicate_source: Option<SwarmServiceDuplicateSource>,
}
impl From<citadel_swarm_services::CreateSwarmService> for CreateSwarmServiceInput {
    fn from(value: citadel_swarm_services::CreateSwarmService) -> Self {
        Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            spec: value.spec.into(),
            tag_ids: value.tag_ids,
            duplicate_source: value.duplicate_source.map(|item| item.into()),
        }
    }
}
impl From<CreateSwarmServiceInput> for citadel_swarm_services::CreateSwarmService {
    fn from(value: CreateSwarmServiceInput) -> Self {
        Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            spec: value.spec.into(),
            tag_ids: value.tag_ids,
            duplicate_source: value.duplicate_source.map(|item| item.into()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSwarmServiceInput {
    pub spec: SwarmServiceSpec,
    pub row_version: i64,
}
impl From<citadel_swarm_services::UpdateSwarmService> for UpdateSwarmServiceInput {
    fn from(value: citadel_swarm_services::UpdateSwarmService) -> Self {
        Self {
            spec: value.spec.into(),
            row_version: value.row_version,
        }
    }
}
impl From<UpdateSwarmServiceInput> for citadel_swarm_services::UpdateSwarmService {
    fn from(value: UpdateSwarmServiceInput) -> Self {
        Self {
            spec: value.spec.into(),
            row_version: value.row_version,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameSwarmServiceInput {
    pub id: Uuid,
    pub name: String,
}
impl From<citadel_swarm_services::RenameSwarmService> for RenameSwarmServiceInput {
    fn from(value: citadel_swarm_services::RenameSwarmService) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}
impl From<RenameSwarmServiceInput> for citadel_swarm_services::RenameSwarmService {
    fn from(value: RenameSwarmServiceInput) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScaleSwarmServiceInput {
    pub replicas: i32,
}
impl From<citadel_swarm_services::ScaleSwarmService> for ScaleSwarmServiceInput {
    fn from(value: citadel_swarm_services::ScaleSwarmService) -> Self {
        Self {
            replicas: value.replicas,
        }
    }
}
impl From<ScaleSwarmServiceInput> for citadel_swarm_services::ScaleSwarmService {
    fn from(value: ScaleSwarmServiceInput) -> Self {
        Self {
            replicas: value.replicas,
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdoptSwarmServiceInput {
    pub name: String,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    pub preview_fingerprint: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    #[schema(nullable)]
    pub tag_ids: Vec<Uuid>,
}
impl From<citadel_swarm_services::adoption::AdoptSwarmService> for AdoptSwarmServiceInput {
    fn from(value: citadel_swarm_services::adoption::AdoptSwarmService) -> Self {
        Self {
            name: value.name,
            description: value.description,
            spec: value.spec.into(),
            preview_fingerprint: value.preview_fingerprint,
            tag_ids: value.tag_ids,
        }
    }
}
impl From<AdoptSwarmServiceInput> for citadel_swarm_services::adoption::AdoptSwarmService {
    fn from(value: AdoptSwarmServiceInput) -> Self {
        Self {
            name: value.name,
            description: value.description,
            spec: value.spec.into(),
            preview_fingerprint: value.preview_fingerprint,
            tag_ids: value.tag_ids,
        }
    }
}

pub(super) fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}
