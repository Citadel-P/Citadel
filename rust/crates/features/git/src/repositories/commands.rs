use super::*;
use webhooks::validate_webhook;
#[derive(Debug, Clone, Default)]
pub enum FieldPatch<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<'de, T> Deserialize<'de> for FieldPatch<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| value.map_or(Self::Null, Self::Value))
    }
}

impl<T: Clone> FieldPatch<T> {
    #[must_use]
    pub fn merge_optional(&self, current: Option<&T>) -> Option<T> {
        match self {
            Self::Missing => current.cloned(),
            Self::Null => None,
            Self::Value(value) => Some(value.clone()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGitRepository {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub default_branch: String,
    pub git_account_id: Option<Uuid>,
    #[serde(default)]
    pub sync_mode: GitRepositorySyncMode,
    #[serde(default = "default_sync_interval")]
    pub sync_interval_minutes: Option<i32>,
    pub webhook: Option<Value>,
    pub on_clone: Option<RepoCommand>,
    pub on_pull: Option<RepoCommand>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl CreateGitRepository {
    pub fn validate(&mut self) -> Result<(), GitRepositoryError> {
        validate_name_identifier(&self.name, "Git repository")?;
        validate_description(self.description.as_deref())?;
        if self.url.trim().is_empty() || self.url.len() > 2048 {
            return Err(GitRepositoryError::Validation(
                "Git repository URL is required and cannot exceed 2048 characters.".to_owned(),
            ));
        }
        if self.default_branch.trim().is_empty() {
            return Err(GitRepositoryError::Validation(
                "Default branch is required.".to_owned(),
            ));
        }
        self.sync_interval_minutes = match self.sync_mode {
            GitRepositorySyncMode::Manual => None,
            GitRepositorySyncMode::PullInterval => Some(
                self.sync_interval_minutes
                    .filter(|interval| *interval >= 1)
                    .ok_or_else(|| {
                        GitRepositoryError::Validation(
                            "Pull interval must be at least one minute.".to_owned(),
                        )
                    })?,
            ),
        };
        validate_repo_command(self.on_clone.as_ref())?;
        validate_repo_command(self.on_pull.as_ref())?;
        validate_webhook(self.webhook.as_ref())
            .map_err(|error| GitRepositoryError::Validation(error.to_string()))?;
        self.name = self.name.trim().to_owned();
        self.url = normalize_git_url(&self.url);
        self.default_branch = self.default_branch.trim().to_owned();
        Ok(())
    }
}

pub fn validate_name_identifier(name: &str, resource: &str) -> Result<(), GitRepositoryError> {
    let name = name.trim();
    if !(3..=64).contains(&name.len())
        || !name
            .bytes()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_'))
    {
        return Err(GitRepositoryError::Validation(format!(
            "{resource} name must contain 3 to 64 letters, numbers, hyphens, or underscores."
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryPatch {
    pub name: Option<String>,
    #[serde(default)]
    pub description: FieldPatch<String>,
    pub url: Option<String>,
    pub default_branch: Option<String>,
    #[serde(default)]
    pub git_account_id: FieldPatch<Uuid>,
    pub sync_mode: Option<GitRepositorySyncMode>,
    #[serde(default)]
    pub sync_interval_minutes: FieldPatch<i32>,
    #[serde(default)]
    pub webhook: FieldPatch<Value>,
    #[serde(default)]
    pub on_clone: FieldPatch<RepoCommand>,
    #[serde(default)]
    pub on_pull: FieldPatch<RepoCommand>,
    pub tag_ids: Option<Vec<Uuid>>,
}

fn validate_repo_command(command: Option<&RepoCommand>) -> Result<(), GitRepositoryError> {
    if command.is_some_and(|command| {
        command.commands.len() > 100
            || command
                .commands
                .iter()
                .any(|value| value.is_empty() || value.len() > 4096)
            || command.path.is_empty()
            || command.path.len() > 1024
    }) {
        return Err(GitRepositoryError::Validation(
            "Git repository commands exceed the supported limits.".to_owned(),
        ));
    }
    Ok(())
}

pub fn validate_description(description: Option<&str>) -> Result<(), GitRepositoryError> {
    if description.is_some_and(|value| value.chars().count() > 600) {
        return Err(GitRepositoryError::Validation(
            "Description cannot exceed 600 characters.".to_owned(),
        ));
    }
    Ok(())
}

#[must_use]
pub fn normalize_git_url(url: &str) -> String {
    let normalized = url.trim().trim_end_matches('/');
    normalized
        .strip_suffix(".git")
        .or_else(|| normalized.strip_suffix(".GIT"))
        .unwrap_or(normalized)
        .to_owned()
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitRepositoryMutationKind {
    Update,
    Metadata,
    Rename,
}

const fn default_sync_interval() -> Option<i32> {
    Some(5)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn git_urls_are_normalized_without_changing_the_repository_path() {
        assert_eq!(
            normalize_git_url(" https://git.example/team/repo.git/ "),
            "https://git.example/team/repo"
        );
        assert_eq!(
            normalize_git_url("https://git.example/team/repo.git"),
            "https://git.example/team/repo"
        );
    }
}
