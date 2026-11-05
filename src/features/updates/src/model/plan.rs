use crate::{CoreIdentity, UpdateError, is_sha256, official_image_digest, release_version};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdatePlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database: Option<super::DatabaseUpdatePlan>,
    pub id: Uuid,
    pub instance_id: Uuid,
    pub source: CoreIdentity,
    pub target: CoreIdentity,
    pub helper_image: String,
    pub configuration_sha256: String,
    pub registration_id: Uuid,
    pub reviewed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub checkpoint_directory: String,
    /// Sorted distinct published Agent versions covered by this review.
    pub agent_requirements: Vec<String>,
}

impl UpdatePlan {
    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), UpdateError> {
        let source = release_version(&self.source.version)?;
        let target = release_version(&self.target.version)?;
        if self.id.is_nil()
            || self.instance_id.is_nil()
            || self.registration_id.is_nil()
            || self.reviewed_at > now
            || now >= self.expires_at
            || self.expires_at - self.reviewed_at > chrono::Duration::minutes(15)
            || !official_image_digest(&self.source.image)
            || !official_image_digest(&self.target.image)
            || self.helper_image != self.source.image
            || !is_sha256(&self.configuration_sha256)
            || !is_sha256(&self.source.catalog_sha256)
            || !is_sha256(&self.target.catalog_sha256)
            || self.source.architecture != self.target.architecture
            || !matches!(self.source.architecture.as_str(), "amd64" | "arm64")
            || match &self.database {
                Some(database) => self.source != self.target || database.validate().is_err(),
                None => source >= target,
            }
            || source.pre.is_empty() != target.pre.is_empty()
            || !self.checkpoint_directory.starts_with('/')
            || self.checkpoint_directory.contains('\0')
            || self
                .checkpoint_directory
                .split('/')
                .any(|segment| matches!(segment, "." | ".."))
            || self.checkpoint_directory.len() > 4096
            || self.agent_requirements.len() > 4096
            || self
                .agent_requirements
                .iter()
                .any(|version| release_version(version).is_err())
            || self
                .agent_requirements
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(UpdateError::Conflict(
                "The update review is invalid or has expired. Review again.".into(),
            ));
        }
        Ok(())
    }
}
