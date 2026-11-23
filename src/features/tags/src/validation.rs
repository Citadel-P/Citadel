use super::*;

impl TaggableResourceType {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Deployment => "Deployment",
            Self::Stack => "Stack",
            Self::Platform => "Platform",
            Self::GitRepository => "GitRepository",
            Self::Registry => "Registry",
            Self::Script => "Script",
            Self::BackupPolicy => "BackupPolicy",
            Self::Build => "Build",
            Self::BuildAgentPool => "BuildAgentPool",
            Self::SwarmService => "SwarmService",
        }
    }
    #[must_use]
    pub const fn permission_type(self) -> citadel_primitives::ResourceType {
        match self {
            Self::Deployment => citadel_primitives::ResourceType::Deployment,
            Self::Stack => citadel_primitives::ResourceType::Stack,
            Self::Platform => citadel_primitives::ResourceType::Platform,
            Self::GitRepository => citadel_primitives::ResourceType::GitRepository,
            Self::Registry => citadel_primitives::ResourceType::Registry,
            Self::Script => citadel_primitives::ResourceType::Script,
            Self::BackupPolicy => citadel_primitives::ResourceType::BackupPolicy,
            Self::Build => citadel_primitives::ResourceType::Build,
            Self::BuildAgentPool => citadel_primitives::ResourceType::BuildAgentPool,
            Self::SwarmService => citadel_primitives::ResourceType::SwarmService,
        }
    }
}

impl NewTag {
    pub fn validate(&mut self) -> Result<(), TagError> {
        validate_tag_name(&self.name)?;
        self.name = self.name.trim().to_owned();
        self.color = normalize_color(&self.color)?;
        Ok(())
    }
}

pub fn normalize_color(color: &str) -> Result<String, TagError> {
    let value = color.trim().to_ascii_uppercase();
    if value.len() != 7
        || !value.starts_with('#')
        || !value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(TagError::Validation(
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
