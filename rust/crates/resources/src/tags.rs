use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ResourceMetadataError, validate_name};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaggableResourceType {
    Deployment,
    Stack,
    Platform,
    GitRepository,
    Registry,
    AutomationAction,
    BackupPolicy,
    Build,
    BuildAgentPool,
    SwarmService,
}

impl TaggableResourceType {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Deployment => "Deployment",
            Self::Stack => "Stack",
            Self::Platform => "Platform",
            Self::GitRepository => "GitRepository",
            Self::Registry => "Registry",
            Self::AutomationAction => "AutomationAction",
            Self::BackupPolicy => "BackupPolicy",
            Self::Build => "Build",
            Self::BuildAgentPool => "BuildAgentPool",
            Self::SwarmService => "SwarmService",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagView {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub color: String,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub usage_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = resources::tags::TagSummary)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewTag {
    pub name: String,
    pub color: String,
}

impl NewTag {
    pub fn validate(&mut self) -> Result<(), ResourceMetadataError> {
        validate_name(&self.name, "Tag")?;
        self.name = self.name.trim().to_owned();
        self.color = normalize_color(&self.color)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagPatch {
    pub name: Option<String>,
    pub color: Option<String>,
}

pub fn normalize_color(color: &str) -> Result<String, ResourceMetadataError> {
    let value = color.trim().to_ascii_uppercase();
    if value.len() != 7
        || !value.starts_with('#')
        || !value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ResourceMetadataError::Validation(
            "Tag color must be a valid hex color in #RRGGBB format.".to_owned(),
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_normalization_matches_dotnet_domain() {
        let mut tag = NewTag {
            name: " Prod ".into(),
            color: "#ef4444".into(),
        };
        tag.validate().unwrap();
        assert_eq!(tag.name, "Prod");
        assert_eq!(tag.color, "#EF4444");
        assert!(normalize_color("red").is_err());
    }
}
