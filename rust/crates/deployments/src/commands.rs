use crate::{DeploymentSpec, DuplicateSource};
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
    pub description: FieldPatch<String>,
}

#[derive(Debug, Clone, Default)]
pub enum FieldPatch<T> {
    #[default]
    Unchanged,
    Set(T),
    Clear,
}
