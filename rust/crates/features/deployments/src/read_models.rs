//! Transport-neutral read results and supporting filters.
//! Repository implementations populate these types; server adapters map them to HTTP views.

use crate::Deployment;
use crate::DeploymentSpec;
use crate::model::TagSummary;
use serde_json::Value;
use uuid::Uuid;
/// ACL-aware enriched read projection, obtained with one repository query.
#[derive(Debug, Clone, PartialEq)]
pub struct DeploymentDetails {
    pub deployment: Deployment,
    pub platform_status: String,
    pub platform_name: Option<String>,
    pub image_name: Option<String>,
    pub image_id: Option<Uuid>,
    pub container_id: Option<Uuid>,
    pub docker_container_id: Option<String>,
    pub docker_image_id: Option<String>,
    pub tags: Vec<TagSummary>,
    pub latest_activity: Option<Value>,
    pub effective_permission: citadel_primitives::EffectivePermission,
}
impl std::ops::Deref for DeploymentDetails {
    type Target = Deployment;
    fn deref(&self) -> &Self::Target {
        &self.deployment
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeploymentConfig {
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
}

impl From<&DeploymentDetails> for DeploymentConfig {
    fn from(value: &DeploymentDetails) -> Self {
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
