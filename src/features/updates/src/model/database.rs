use crate::UpdateError;
use serde::{Deserialize, Serialize};

/// Explicitly reviewed PostgreSQL engine replacement. Core and Agent targets
/// remain unchanged; a scheduled Core update never implicitly upgrades PostgreSQL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatabaseUpdatePlan {
    pub source_version: u32,
    pub target_version: u32,
    pub target_image: String,
}

impl DatabaseUpdatePlan {
    pub fn validate(&self) -> Result<(), UpdateError> {
        use citadel_primitives::postgres::require_supported;
        if require_supported(self.source_version).is_err()
            || require_supported(self.target_version).is_err()
            || self.source_version >= self.target_version
            || !self
                .target_image
                .strip_prefix("postgres@sha256:")
                .is_some_and(crate::is_sha256)
        {
            return Err(UpdateError::Conflict(
                "This PostgreSQL upgrade is not supported by this build.".into(),
            ));
        }
        Ok(())
    }
    pub fn target_display(&self) -> String {
        format!(
            "{}.{}",
            self.target_version / 10000,
            self.target_version % 10000
        )
    }
}
