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
        let source_major = self.source_version / 10000;
        let target_major = self.target_version / 10000;
        if source_major != 18
            || target_major != 18
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
