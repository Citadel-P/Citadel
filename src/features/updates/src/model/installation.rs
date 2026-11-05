use super::CoreIdentity;
use uuid::Uuid;

/// Live source facts used to build and revalidate an administrator's review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationSnapshot {
    pub instance_id: Uuid,
    pub source: CoreIdentity,
    pub registration_id: Uuid,
    pub configuration_sha256: String,
    pub checkpoint_directory: String,
    pub agent_versions: Vec<String>,
}
