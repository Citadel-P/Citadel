use crate::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildProjectConfiguration {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub git_repository_id: Uuid,
    pub branch: Option<String>,
    pub context_path: Option<String>,
    pub dockerfile_path: Option<String>,
    pub target: Option<String>,
    pub build_args: Option<Vec<BuildArgSpec>>,
    pub build_secrets: Option<Vec<BuildSecretSpec>>,
    pub platform_id: Option<Uuid>,
    pub registry_id: Uuid,
    pub image_repository: String,
    pub tag_templates: Option<Vec<String>>,
    pub webhook: Option<Value>,
    pub timeout_seconds: Option<i32>,
    pub retention_run_count: Option<i32>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
    #[serde(default = "platform_builder")]
    pub builder_kind: String,
    pub build_agent_pool_id: Option<Uuid>,
}

fn platform_builder() -> String {
    "Platform".to_owned()
}

impl BuildProjectConfiguration {
    pub fn validate(&mut self) -> Result<(), BuildError> {
        citadel_git::repositories::webhooks::validate_webhook(self.webhook.as_ref())
            .map_err(|error| BuildError::Validation(error.to_string()))?;
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() || self.name.chars().count() > 128 {
            return Err(BuildError::Validation(
                "Build Project name must contain between 1 and 128 characters.".to_owned(),
            ));
        }
        self.description = self
            .description
            .take()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if self
            .description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 600)
        {
            return Err(BuildError::Validation(
                "Build Project description cannot exceed 600 characters.".to_owned(),
            ));
        }
        if self.git_repository_id.is_nil() || self.registry_id.is_nil() {
            return Err(BuildError::Validation(
                "Git Repository and Registry are required.".to_owned(),
            ));
        }
        match self.builder_kind.as_str() {
            "Platform"
                if self.platform_id.is_some_and(|id| !id.is_nil())
                    && self.build_agent_pool_id.is_none() => {}
            "BuildAgentPool"
                if self.platform_id.is_none()
                    && self.build_agent_pool_id.is_some_and(|id| !id.is_nil()) => {}
            _ => {
                return Err(BuildError::Validation(
                    "Select exactly one valid Build execution target.".to_owned(),
                ));
            }
        }
        let branch = normalize_required(self.branch.take(), "main")?;
        if !valid_git_branch(&branch) {
            return Err(BuildError::Validation(
                "Git branch name is invalid.".to_owned(),
            ));
        }
        self.branch = Some(branch);
        self.context_path = Some(normalize_path(self.context_path.take(), ".")?);
        self.dockerfile_path = Some(normalize_path(self.dockerfile_path.take(), "Dockerfile")?);
        self.image_repository = self
            .image_repository
            .trim()
            .trim_start_matches('/')
            .to_owned();
        if self.image_repository.is_empty() || self.image_repository.len() > 512 {
            return Err(BuildError::Validation(
                "Image repository is required and cannot exceed 512 bytes.".to_owned(),
            ));
        }
        let timeout = self.timeout_seconds.unwrap_or(1_800);
        if !(60..=86_400).contains(&timeout) {
            return Err(BuildError::Validation(
                "Build timeout must be between 60 and 86400 seconds.".to_owned(),
            ));
        }
        let retention = self.retention_run_count.unwrap_or(20);
        if !(1..=1_000).contains(&retention) {
            return Err(BuildError::Validation(
                "Build retention must keep between 1 and 1000 runs.".to_owned(),
            ));
        }
        self.timeout_seconds = Some(timeout);
        self.retention_run_count = Some(retention);
        let mut tags = self
            .tag_templates
            .take()
            .unwrap_or_else(|| vec!["{branch}-{shortSha}".to_owned()]);
        tags = tags
            .into_iter()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .collect();
        tags.sort();
        tags.dedup();
        if tags.is_empty() {
            return Err(BuildError::Validation(
                "At least one image tag template is required.".to_owned(),
            ));
        }
        self.tag_templates = Some(tags);
        let mut secret_ids = HashSet::new();
        let build_secrets = self.build_secrets.get_or_insert_with(Vec::new);
        if build_secrets.len() > 64 {
            return Err(BuildError::Validation(
                "A Build cannot reference more than 64 Secrets.".to_owned(),
            ));
        }
        for secret in build_secrets {
            secret.id = secret.id.trim().to_owned();
            if secret.id.is_empty()
                || !secret
                    .id
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
            {
                return Err(BuildError::Validation(format!(
                    "BuildKit secret id '{}' is invalid.",
                    secret.id
                )));
            }
            if secret.secret_id.is_nil() {
                return Err(BuildError::Validation(format!(
                    "BuildKit secret '{}' must reference a Citadel Secret.",
                    secret.id
                )));
            }
            if !secret_ids.insert(secret.id.to_ascii_lowercase()) {
                return Err(BuildError::Validation(format!(
                    "BuildKit secret id '{}' is mapped more than once.",
                    secret.id
                )));
            }
        }
        let build_args = self.build_args.get_or_insert_with(Vec::new);
        if build_args.len() > 256 {
            return Err(BuildError::Validation(
                "A Build cannot define more than 256 build arguments.".to_owned(),
            ));
        }
        let mut argument_names = HashSet::new();
        for argument in build_args {
            argument.name = argument.name.trim().to_owned();
            if argument.name.is_empty()
                || argument.name.len() > 256
                || !argument_names.insert(argument.name.to_ascii_lowercase())
            {
                return Err(BuildError::Validation(
                    "Build argument names must be non-empty and unique.".to_owned(),
                ));
            }
            if argument
                .value
                .as_ref()
                .is_some_and(|value| value.len() > 16 * 1024)
            {
                return Err(BuildError::Validation(
                    "A Build argument value cannot exceed 16 KiB.".to_owned(),
                ));
            }
        }
        if self.tag_ids.len() > 100 {
            return Err(BuildError::Validation(
                "A Build cannot have more than 100 Tags.".to_owned(),
            ));
        }
        self.tag_ids.sort_unstable();
        self.tag_ids.dedup();
        Ok(())
    }
}

impl From<&BuildProject> for BuildProjectConfiguration {
    fn from(value: &BuildProject) -> Self {
        Self {
            name: value.name.clone(),
            description: value.description.clone(),
            enabled: value.enabled,
            git_repository_id: value.git_repository_id,
            branch: Some(value.branch.clone()),
            context_path: Some(value.context_path.clone()),
            dockerfile_path: Some(value.dockerfile_path.clone()),
            target: value.target.clone(),
            build_args: Some(value.build_args.clone()),
            build_secrets: Some(value.build_secrets.clone()),
            platform_id: value.platform_id,
            registry_id: value.registry_id,
            image_repository: value.image_repository.clone(),
            tag_templates: Some(value.tag_templates.clone()),
            webhook: value.webhook.clone(),
            timeout_seconds: Some(value.timeout_seconds),
            retention_run_count: Some(value.retention_run_count),
            tag_ids: Vec::new(),
            builder_kind: value.builder_kind.clone(),
            build_agent_pool_id: value.build_agent_pool_id,
        }
    }
}
