use serde::{Deserialize, Serialize};

/// Installation mechanisms are qualified independently of release verification.
/// Declaring a kind alone never qualifies an update or recovery path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallationKind {
    DockerCompose,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoreIdentity {
    pub version: String,
    pub image: String,
    pub catalog_sha256: String,
    pub architecture: String,
}
