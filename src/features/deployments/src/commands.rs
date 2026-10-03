use crate::DeploymentSpec;
use crate::DuplicateSource;
use citadel_primitives::PatchField;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateDeployment {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
    pub tag_ids: Vec<Uuid>,
    pub duplicate_source: Option<DuplicateSource>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateDeploymentMetadata {
    pub description: PatchField<String>,
}
