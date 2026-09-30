use super::*;
use citadel_primitives::AutoUpdateState;
use uuid::Uuid;

/// Deployment state and related data, independent of HTTP and caller permissions.
#[derive(Debug, Clone, PartialEq)]
pub struct Deployment {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub platform_id: Uuid,
    pub status: crate::DeploymentStatus,
    pub control_state: citadel_primitives::ResourceControlState,
    pub row_version: i64,
    pub auto_update_state: Option<AutoUpdateState>,
    pub spec: DeploymentSpec,
    pub audit: citadel_primitives::AuditMetadata,

    // Related data populated by full resource reads.
    pub platform_status: citadel_primitives::PlatformStatus,
    pub platform_name: Option<String>,
    pub image_name: Option<String>,
    pub image_id: Option<Uuid>,
    pub container_id: Option<Uuid>,
    pub docker_container_id: Option<String>,
    pub docker_image_id: Option<String>,
    pub tags: Vec<citadel_tags::TagSummary>,
    pub latest_activity: Option<citadel_activities::ActivitySummary>,
}
#[derive(Debug, Clone, thiserror::Error)]
pub enum DeploymentError {
    #[error("{0}")]
    Validation(String),
    #[error("Deployment was not found.")]
    NotFound,
    #[error("This operation is not authorized.")]
    Forbidden,
    #[error("License capability '{0}' is unavailable.")]
    LicenseRequired(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("Deployment runtime is unavailable: {0}")]
    Runtime(String),
    #[error("Deployment persistence failed: {0}")]
    Storage(String),
    #[error("Deployment operation was cancelled.")]
    Cancelled,
}
