use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildArgSpec {
    pub name: String,
    pub value: Option<String>,
}

impl From<citadel_builds::BuildArgSpec> for BuildArgSpec {
    fn from(value: citadel_builds::BuildArgSpec) -> Self {
        Self {
            name: value.name,
            value: value.value,
        }
    }
}

impl From<BuildArgSpec> for citadel_builds::BuildArgSpec {
    fn from(value: BuildArgSpec) -> Self {
        Self {
            name: value.name,
            value: value.value,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildSecretSpec {
    pub id: String,
    pub secret_id: Uuid,
}

impl From<citadel_builds::BuildSecretSpec> for BuildSecretSpec {
    fn from(value: citadel_builds::BuildSecretSpec) -> Self {
        Self {
            id: value.id,
            secret_id: value.secret_id,
        }
    }
}

impl From<BuildSecretSpec> for citadel_builds::BuildSecretSpec {
    fn from(value: BuildSecretSpec) -> Self {
        Self {
            id: value.id,
            secret_id: value.secret_id,
        }
    }
}
