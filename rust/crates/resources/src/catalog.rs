use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{ResourceMetadataError, TagSummary};

#[derive(Debug, Clone, Default)]
pub enum MetadataPatch<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<'de, T> Deserialize<'de> for MetadataPatch<T>
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

impl<T: Clone> MetadataPatch<T> {
    #[must_use]
    pub fn merge_optional(&self, current: Option<&T>) -> Option<T> {
        match self {
            Self::Missing => current.cloned(),
            Self::Null => None,
            Self::Value(value) => Some(value.clone()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum RegistryStatus {
    Active,
    Disabled,
    Deprecated,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = resources::catalog::GitRepositorySyncMode)]
pub enum GitRepositorySyncMode {
    Manual,
    #[default]
    PullInterval,
}

impl GitRepositorySyncMode {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::PullInterval => "PullInterval",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RepoCommand {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default = "default_repo_command_path")]
    pub path: String,
}

fn default_repo_command_path() -> String {
    "./".to_owned()
}

impl RegistryStatus {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Disabled => "Disabled",
            Self::Deprecated => "Deprecated",
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryView {
    pub id: Uuid,
    pub created_by_actor_id: Uuid,
    pub name: String,
    pub status: RegistryStatus,
    pub description: Option<String>,
    pub registry_host: String,
    #[serde(rename = "type")]
    pub registry_type: String,
    #[serde(skip_serializing)]
    pub configuration: Value,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<TagSummary>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewRegistry {
    pub name: String,
    pub registry_host: String,
    pub status: RegistryStatus,
    pub configuration: Value,
    pub description: Option<String>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl NewRegistry {
    pub fn validate(&mut self) -> Result<(), ResourceMetadataError> {
        validate_name_identifier(&self.name, "Registry")?;
        self.name = self.name.trim().to_owned();
        validate_description(self.description.as_deref())?;
        let kind = registry_type(&self.configuration)?;
        validate_registry_configuration(&self.configuration, &kind)?;
        if kind == "DockerHub" {
            self.registry_host = "docker.io".to_owned();
        } else if kind == "GitHub" {
            self.registry_host = "ghcr.io".to_owned();
        } else {
            self.registry_host = self.registry_host.trim().trim_end_matches('/').to_owned();
        }
        validate_registry_host(&self.registry_host)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPatch {
    pub name: Option<String>,
    pub registry_host: Option<String>,
    pub status: Option<RegistryStatus>,
    #[serde(default)]
    #[schema(value_type = Option<Value>, required = false)]
    pub configuration: MetadataPatch<Value>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub description: MetadataPatch<String>,
    pub tag_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub default_branch: String,
    pub git_account_id: Option<Uuid>,
    pub sync_mode: GitRepositorySyncMode,
    pub sync_interval_minutes: Option<i32>,
    pub webhook: Option<Value>,
    pub on_clone: Option<RepoCommand>,
    pub on_pull: Option<RepoCommand>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    pub control_state: String,
    pub latest_activity_view: Option<Value>,
    pub tags: Vec<TagSummary>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewGitRepository {
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

impl NewGitRepository {
    pub fn validate(&mut self) -> Result<(), ResourceMetadataError> {
        validate_name_identifier(&self.name, "Git repository")?;
        validate_description(self.description.as_deref())?;
        if self.url.trim().is_empty() || self.url.len() > 2048 {
            return Err(ResourceMetadataError::Validation(
                "Git repository URL is required and cannot exceed 2048 characters.".to_owned(),
            ));
        }
        if self.default_branch.trim().is_empty() {
            return Err(ResourceMetadataError::Validation(
                "Default branch is required.".to_owned(),
            ));
        }
        self.sync_interval_minutes = match self.sync_mode {
            GitRepositorySyncMode::Manual => None,
            GitRepositorySyncMode::PullInterval => Some(
                self.sync_interval_minutes
                    .filter(|interval| *interval >= 1)
                    .ok_or_else(|| {
                        ResourceMetadataError::Validation(
                            "Pull interval must be at least one minute.".to_owned(),
                        )
                    })?,
            ),
        };
        validate_repo_command(self.on_clone.as_ref())?;
        validate_repo_command(self.on_pull.as_ref())?;
        validate_webhook(self.webhook.as_ref())?;
        self.name = self.name.trim().to_owned();
        self.url = normalize_git_url(&self.url);
        self.default_branch = self.default_branch.trim().to_owned();
        Ok(())
    }
}

pub fn validate_name_identifier(name: &str, resource: &str) -> Result<(), ResourceMetadataError> {
    let name = name.trim();
    if !(3..=64).contains(&name.len())
        || !name
            .bytes()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_'))
    {
        return Err(ResourceMetadataError::Validation(format!(
            "{resource} name must contain 3 to 64 letters, numbers, hyphens, or underscores."
        )));
    }
    Ok(())
}

fn validate_registry_host(host: &str) -> Result<(), ResourceMetadataError> {
    let (authority, path) = host
        .split_once('/')
        .map_or((host, None), |(host, path)| (host, Some(path)));
    let (hostname, port) = authority
        .rsplit_once(':')
        .filter(|(_, port)| port.bytes().all(|character| character.is_ascii_digit()))
        .map_or((authority, None), |(hostname, port)| (hostname, Some(port)));
    let valid_hostname = !hostname.is_empty()
        && hostname.len() <= 253
        && hostname.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|character| character.is_ascii_alphanumeric() || character == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        });
    let valid_port = port.is_none_or(|port| !port.is_empty() && port.len() <= 5);
    let valid_path = path.is_none_or(|path| {
        !path.is_empty()
            && path
                .bytes()
                .all(|character| character.is_ascii_alphanumeric() || b"._-/".contains(&character))
    });
    if !valid_hostname || !valid_port || !valid_path {
        return Err(ResourceMetadataError::Validation(
            "Registry host must be a valid host name such as ghcr.io.".to_owned(),
        ));
    }
    Ok(())
}

fn validate_registry_configuration(
    configuration: &Value,
    kind: &str,
) -> Result<(), ResourceMetadataError> {
    let string_length = |name: &str, minimum: usize| {
        configuration_value(configuration, name)
            .and_then(Value::as_str)
            .is_some_and(|value| value.len() >= minimum)
    };
    let enabled = |name: &str| {
        configuration_value(configuration, name)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    let valid = match kind {
        "DockerHub" => string_length("userName", 4) && string_length("pat", 10),
        "Azure" => string_length("userName", 3) && string_length("password", 6),
        "AWS" => {
            string_length("accessKey", 10)
                && string_length("secretAccessKey", 10)
                && string_length("region", 4)
        }
        "Gitlab" => {
            string_length("instanceUrl", 10)
                && string_length("userName", 10)
                && string_length("pat", 10)
        }
        "GitHub" => {
            !enabled("ghcrAuthEnabled")
                || (string_length("pat", 10) && string_length("nameSpace", 5))
        }
        "Custom" => {
            !enabled("authEnabled")
                || (string_length("userName", 3) && string_length("password", 6))
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(ResourceMetadataError::Validation(format!(
            "{kind} Registry credentials do not meet the required fields and minimum lengths."
        )))
    }
}

fn configuration_value<'a>(configuration: &'a Value, name: &str) -> Option<&'a Value> {
    configuration
        .as_object()?
        .iter()
        .find_map(|(key, value)| key.eq_ignore_ascii_case(name).then_some(value))
}

pub fn validate_webhook(webhook: Option<&Value>) -> Result<(), ResourceMetadataError> {
    let Some(webhook) = webhook else {
        return Ok(());
    };
    let enabled = configuration_value(webhook, "enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let provider = configuration_value(webhook, "provider")
        .and_then(Value::as_str)
        .unwrap_or("GitHub");
    let scheme = configuration_value(webhook, "authScheme")
        .and_then(Value::as_str)
        .unwrap_or("GitHubHmacSha256");
    let secret = configuration_value(webhook, "secret").and_then(Value::as_str);
    let branch = configuration_value(webhook, "branchFilter").and_then(Value::as_str);
    if secret.is_some_and(|value| value.len() > 256)
        || branch.is_some_and(|value| value.len() > 256)
    {
        return Err(ResourceMetadataError::Validation(
            "Webhook Secret and branch filter cannot exceed 256 characters.".to_owned(),
        ));
    }
    let valid_authentication = !enabled
        || matches!(
            (provider, scheme),
            ("GitHub", "GitHubHmacSha256") | ("GitLab", "GitLabSignedToken" | "GitLabLegacyToken")
        )
        || (provider == "Generic"
            && scheme == "BearerToken"
            && secret.is_some_and(|value| !value.trim().is_empty()));
    if valid_authentication {
        Ok(())
    } else {
        Err(ResourceMetadataError::Validation(
            "Webhook provider and authentication scheme are not compatible.".to_owned(),
        ))
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryPatch {
    pub name: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub description: MetadataPatch<String>,
    pub url: Option<String>,
    pub default_branch: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<Uuid>, required = false)]
    pub git_account_id: MetadataPatch<Uuid>,
    pub sync_mode: Option<GitRepositorySyncMode>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    pub sync_interval_minutes: MetadataPatch<i32>,
    #[serde(default)]
    #[schema(value_type = Option<Value>, required = false)]
    pub webhook: MetadataPatch<Value>,
    #[serde(default)]
    #[schema(value_type = Option<RepoCommand>, required = false)]
    pub on_clone: MetadataPatch<RepoCommand>,
    #[serde(default)]
    #[schema(value_type = Option<RepoCommand>, required = false)]
    pub on_pull: MetadataPatch<RepoCommand>,
    pub tag_ids: Option<Vec<Uuid>>,
}

const fn default_sync_interval() -> Option<i32> {
    Some(5)
}

fn validate_repo_command(command: Option<&RepoCommand>) -> Result<(), ResourceMetadataError> {
    if command.is_some_and(|command| {
        command.commands.len() > 100
            || command
                .commands
                .iter()
                .any(|value| value.is_empty() || value.len() > 4096)
            || command.path.is_empty()
            || command.path.len() > 1024
    }) {
        return Err(ResourceMetadataError::Validation(
            "Git repository commands exceed the supported limits.".to_owned(),
        ));
    }
    Ok(())
}

pub fn registry_type(configuration: &Value) -> Result<String, ResourceMetadataError> {
    configuration
        .get("$type")
        .or_else(|| configuration.get("type"))
        .and_then(Value::as_str)
        .filter(|kind| {
            matches!(
                *kind,
                "Custom" | "DockerHub" | "Azure" | "AWS" | "Gitlab" | "GitHub"
            )
        })
        .map(str::to_owned)
        .ok_or_else(|| {
            ResourceMetadataError::Validation(
                "Registry configuration must include a supported type discriminator.".to_owned(),
            )
        })
}

pub fn validate_description(description: Option<&str>) -> Result<(), ResourceMetadataError> {
    if description.is_some_and(|value| value.chars().count() > 600) {
        return Err(ResourceMetadataError::Validation(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_validation_normalizes_known_hosts_and_rejects_untyped_credentials() {
        let mut docker_hub = NewRegistry {
            name: " Docker-Hub ".into(),
            registry_host: "ignored.example".into(),
            status: RegistryStatus::Active,
            configuration: serde_json::json!({
                "$type":"DockerHub",
                "userName":"user",
                "pat":"0123456789"
            }),
            description: None,
            tag_ids: Vec::new(),
        };
        docker_hub.validate().unwrap();
        assert_eq!(docker_hub.name, "Docker-Hub");
        assert_eq!(docker_hub.registry_host, "docker.io");

        docker_hub.configuration = serde_json::json!({"Username":"user"});
        assert!(docker_hub.validate().is_err());
    }

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
