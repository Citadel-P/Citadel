use crate::api::resources::common::DuplicateSourceInput;
use crate::api::resources::swarm_services::spec::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
    pub duplicate_source: Option<DuplicateSourceInput>,
}

impl TryFrom<citadel_swarm_services::CreateSwarmService> for CreateSwarmServiceInput {
    type Error = serde_json::Error;

    fn try_from(value: citadel_swarm_services::CreateSwarmService) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            spec: value.spec.try_into()?,
            tag_ids: value.tag_ids,
            duplicate_source: value.duplicate_source.map(TryInto::try_into).transpose()?,
        })
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

impl TryFrom<citadel_swarm_services::UpdateSwarmService> for UpdateSwarmServiceInput {
    type Error = serde_json::Error;

    fn try_from(value: citadel_swarm_services::UpdateSwarmService) -> Result<Self, Self::Error> {
        Ok(Self {
            spec: value.spec.try_into()?,
            row_version: value.row_version,
        })
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

impl TryFrom<citadel_swarm_services::adoption::AdoptSwarmService> for AdoptSwarmServiceInput {
    type Error = serde_json::Error;

    fn try_from(
        value: citadel_swarm_services::adoption::AdoptSwarmService,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            description: value.description,
            spec: value.spec.try_into()?,
            preview_fingerprint: value.preview_fingerprint,
            tag_ids: value.tag_ids,
        })
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

pub(crate) fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct ServiceMetadataInput {
    #[serde(deserialize_with = "deserialize_description")]
    pub(crate) description: Option<String>,
}

pub(super) fn deserialize_description<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    serde::Deserialize::deserialize(deserializer)
}
