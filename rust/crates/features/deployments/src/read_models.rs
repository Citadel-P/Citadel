//! Transport-neutral read results and supporting filters.
//! Repository implementations populate these types; server adapters map them to HTTP views.

use crate::DeploymentSpec;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct DeploymentConfig {
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
}

impl From<&crate::Deployment> for DeploymentConfig {
    fn from(value: &crate::Deployment) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            platform_id: value.platform_id,
            description: value.description.clone(),
            spec: value.spec.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateSource {
    pub resource_type: String,
    pub resource_id: Uuid,
    pub resource_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct DeploymentFilter {
    pub tags: Vec<String>,
    pub platform_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateWarning {
    pub code: String,
    pub message: String,
    pub field_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeploymentDuplicateDraft {
    pub draft: DeploymentDraft,
    pub warnings: Vec<DuplicateWarning>,
}

#[derive(Debug, Clone)]
pub struct DeploymentDraft {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
    pub tag_ids: Vec<Uuid>,
    pub duplicate_source: DuplicateSource,
}
