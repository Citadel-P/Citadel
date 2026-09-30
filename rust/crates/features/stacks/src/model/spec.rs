use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};

use serde_json::Value;

use crate::*;
use uuid::Uuid;

pub fn normalize_project_name(name: &str, id: Uuid) -> String {
    let value = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if value.is_empty() {
        format!("stack-{}", id.simple())
    } else {
        value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackSource {
    #[serde(alias = "webEditor", alias = "webeditor")]
    WebEditor,
    #[serde(alias = "git")]
    Git,
}

impl StackSource {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WebEditor => "WebEditor",
            Self::Git => "Git",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackUpdateBehavior {
    #[serde(alias = "disabled")]
    Disabled,
    #[serde(alias = "notify")]
    Notify,
    #[serde(alias = "serviceAutoDeploy", alias = "serviceautodeploy")]
    ServiceAutoDeploy,
    #[serde(alias = "stackAutoDeploy", alias = "stackautodeploy")]
    StackAutoDeploy,
}

citadel_primitives::status_enum! {
    pub enum StackReleaseStatus {
        Unknown,
        Created,
        Applying,
        Healthy,
        Pending,
        Paused,
        Degraded,
        Failed,
        Stopped,
        TimedOut,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackOrchestrationMode {
    DockerCompose,
    DockerSwarm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackDriftMode {
    Disabled,
    DetectOnly,
    AutoFix,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackDriftPolicy {
    pub mode: StackDriftMode,
    pub alert_on_drift: bool,
    pub mark_degraded: bool,
    pub auto_start_stopped_containers: bool,
    pub auto_resume_paused_containers: bool,
    pub remove_extra_containers: bool,
}

impl Default for StackDriftPolicy {
    fn default() -> Self {
        Self {
            mode: StackDriftMode::DetectOnly,
            alert_on_drift: true,
            mark_degraded: true,
            auto_start_stopped_containers: false,
            auto_resume_paused_containers: false,
            remove_extra_containers: false,
        }
    }
}

impl StackDriftPolicy {
    #[must_use]
    pub fn normalized(mut self) -> Self {
        if self.mode == StackDriftMode::Disabled {
            self.alert_on_drift = false;
            self.mark_degraded = false;
            self.auto_start_stopped_containers = false;
            self.auto_resume_paused_containers = false;
            self.remove_extra_containers = false;
        }
        self
    }

    pub fn to_storage_value(&self) -> Result<Value, StackError> {
        let mut value = serde_json::to_value(self).map_err(json_storage)?;
        rename_object_keys(&mut value, true);
        Ok(value)
    }

    pub fn from_storage_value(mut value: Value) -> Result<Self, StackError> {
        rename_object_keys(&mut value, false);
        serde_json::from_value(value).map_err(json_storage)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StackCommand {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default = "default_command_path")]
    pub path: String,
}

pub(crate) fn default_command_path() -> String {
    "./".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StackBuildImageBinding {
    pub service_name: String,
    pub build_project_id: Uuid,
    #[serde(default)]
    pub redeploy_on_build: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_image_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_build_run_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_image_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_build_run_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<DateTime<Utc>>,
}

impl StackBuildImageBinding {
    #[must_use]
    pub fn without_provenance(mut self) -> Self {
        self.resolved_image_reference = None;
        self.resolved_digest = None;
        self.resolved_build_run_id = None;
        self.applied_image_reference = None;
        self.applied_digest = None;
        self.applied_build_run_id = None;
        self.applied_at = None;
        self
    }

    pub fn record_applied(
        &mut self,
        resolved: &ResolvedStackBuildImageBinding,
        applied_at: DateTime<Utc>,
    ) {
        self.resolved_image_reference = Some(resolved.image_reference.clone());
        self.resolved_digest = resolved.digest.clone();
        self.resolved_build_run_id = resolved.build_run_id;
        self.applied_image_reference = Some(resolved.image_reference.clone());
        self.applied_digest = resolved.digest.clone();
        self.applied_build_run_id = resolved.build_run_id;
        self.applied_at = Some(applied_at);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StackWebhookConfig {
    #[serde(flatten)]
    pub config: citadel_primitives::WebhookConfig,
    #[serde(default)]
    pub force_deploy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "$type")]
// Stack specs are request/configuration objects, not hot-path event values.
// Boxing the Git fields would add heap indirection to every parse and clone.
#[allow(clippy::large_enum_variant)]
pub enum StackSpec {
    WebEditor {
        #[serde(rename = "composeFile")]
        compose_file: String,
        #[serde(rename = "updateBehavior", default = "disabled_update")]
        update_behavior: StackUpdateBehavior,
        #[serde(flatten)]
        common: StackSpecCommon,
    },
    Git {
        #[serde(rename = "gitRepoId")]
        git_repo_id: Uuid,
        branch: String,
        #[serde(rename = "commitSha", default, skip_serializing_if = "Option::is_none")]
        commit_sha: Option<String>,
        #[serde(rename = "updateBehavior", default = "disabled_update")]
        update_behavior: StackUpdateBehavior,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        webhook: Option<StackWebhookConfig>,
        #[serde(rename = "composePaths", default)]
        compose_paths: Vec<String>,
        #[serde(
            rename = "workingDirectory",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        working_directory: Option<String>,
        #[serde(rename = "composeEnvFilesFromRepo", default)]
        compose_env_files_from_repo: Vec<String>,
        #[serde(rename = "watchPaths", default)]
        watch_paths: Vec<String>,
        #[serde(rename = "additionalEnvFileFromRepo", default)]
        additional_env_file_from_repo: Vec<String>,
        #[serde(flatten)]
        common: StackSpecCommon,
    },
}

const fn disabled_update() -> StackUpdateBehavior {
    StackUpdateBehavior::Disabled
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StackSpecCommon {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_deploy: Option<StackCommand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_deploy: Option<StackCommand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<Uuid>,
    #[serde(default = "default_destroy_before_deploy")]
    pub destroy_before_deploy: bool,
    #[serde(default)]
    pub build_image_bindings: Vec<StackBuildImageBinding>,
}

const fn default_destroy_before_deploy() -> bool {
    true
}

impl StackSpec {
    #[must_use]
    pub const fn source(&self) -> StackSource {
        match self {
            Self::WebEditor { .. } => StackSource::WebEditor,
            Self::Git { .. } => StackSource::Git,
        }
    }

    #[must_use]
    pub const fn common(&self) -> &StackSpecCommon {
        match self {
            Self::WebEditor { common, .. } | Self::Git { common, .. } => common,
        }
    }

    pub fn common_mut(&mut self) -> &mut StackSpecCommon {
        match self {
            Self::WebEditor { common, .. } | Self::Git { common, .. } => common,
        }
    }

    #[must_use]
    pub fn compose_file(&self) -> Option<&str> {
        match self {
            Self::WebEditor { compose_file, .. } => Some(compose_file),
            Self::Git { .. } => None,
        }
    }

    pub fn validate(&self) -> Result<(), StackError> {
        if let Some(project) = self.common().project_name.as_deref() {
            validate_project_name(project)?;
        }
        validate_relative_optional(self.common().env_file_path.as_deref(), "Environment file")?;
        for command in [
            self.common().pre_deploy.as_ref(),
            self.common().post_deploy.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_relative(&command.path, "Command path")?;
            if command.commands.len() > 100
                || command.commands.iter().any(|value| value.len() > 16_384)
            {
                return Err(validation("Stack commands exceed the supported limits."));
            }
        }
        if self.common().build_image_bindings.len() > 100 {
            return Err(validation(
                "A Stack cannot contain more than 100 build bindings.",
            ));
        }
        let mut services = std::collections::HashSet::new();
        if self.common().build_image_bindings.iter().any(|binding| {
            binding.service_name.trim().is_empty()
                || binding.build_project_id.is_nil()
                || !services.insert(binding.service_name.to_ascii_lowercase())
        }) {
            return Err(validation(
                "Stack build bindings require unique Service names.",
            ));
        }
        match self {
            Self::WebEditor { compose_file, .. } => {
                if compose_file.trim().is_empty() || compose_file.len() > 2 * 1024 * 1024 {
                    return Err(validation(
                        "The Compose file is required and must not exceed 2 MiB.",
                    ));
                }
            }
            Self::Git {
                git_repo_id,
                branch,
                compose_paths,
                working_directory,
                compose_env_files_from_repo,
                watch_paths,
                additional_env_file_from_repo,
                webhook,
                ..
            } => {
                if git_repo_id.is_nil() || branch.trim().is_empty() || branch.len() > 255 {
                    return Err(validation("A Git repository and branch are required."));
                }
                if compose_paths.is_empty() || compose_paths.len() > 20 {
                    return Err(validation("Git Stacks require 1 to 20 Compose paths."));
                }
                for path in compose_paths
                    .iter()
                    .chain(compose_env_files_from_repo)
                    .chain(watch_paths)
                    .chain(additional_env_file_from_repo)
                {
                    validate_relative(path, "Repository path")?;
                }
                validate_relative_optional(working_directory.as_deref(), "Working directory")?;
                if let Some(webhook) = webhook {
                    webhook.config.validate().map_err(validation)?;
                }
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn for_create(mut self) -> Self {
        self.common_mut().build_image_bindings = self
            .common()
            .build_image_bindings
            .clone()
            .into_iter()
            .map(StackBuildImageBinding::without_provenance)
            .collect();
        self
    }

    pub fn to_storage_value(&self) -> Result<Value, StackError> {
        let api = serde_json::to_value(self).map_err(json_storage)?;
        Ok(pascalize_stack_spec(api))
    }

    pub fn from_storage_value(value: Value) -> Result<Self, StackError> {
        serde_json::from_value(camelize_stack_spec(value)).map_err(json_storage)
    }
}

pub(crate) fn pascalize_stack_spec(mut value: Value) -> Value {
    rename_object_keys(&mut value, true);
    value
}

pub(crate) fn camelize_stack_spec(mut value: Value) -> Value {
    rename_object_keys(&mut value, false);
    value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum StackUpdateState {
    WebEditor {
        #[serde(rename = "recreateStackOnNewImageState")]
        recreate_stack_on_new_image_state: RecreateStackOnNewImageState,
    },
    Git {
        #[serde(rename = "recreateStackOnNewImageState")]
        recreate_stack_on_new_image_state: RecreateStackOnNewImageState,
        #[serde(rename = "recreateStackOnNewCommitState")]
        recreate_stack_on_new_commit_state: RecreateStackOnNewCommitState,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecreateStackOnNewImageState {
    #[serde(default)]
    pub auto_update_states: Vec<ImageUpdateState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecreateStackOnNewCommitState {
    pub current_commit_sha: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_commit_sha: Option<String>,
    pub last_checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUpdateState {
    pub service_name: String,
    pub image_name: String,
    pub current_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_digest: Option<String>,
    pub last_checked_at: DateTime<Utc>,
    pub update_available: bool,
}

impl StackUpdateState {
    #[must_use]
    pub fn new(spec: &StackSpec) -> Self {
        match spec {
            StackSpec::WebEditor { .. } => Self::WebEditor {
                recreate_stack_on_new_image_state: RecreateStackOnNewImageState::default(),
            },
            StackSpec::Git { commit_sha, .. } => Self::Git {
                recreate_stack_on_new_image_state: RecreateStackOnNewImageState::default(),
                recreate_stack_on_new_commit_state: RecreateStackOnNewCommitState {
                    current_commit_sha: commit_sha.clone().unwrap_or_default(),
                    remote_commit_sha: None,
                    last_checked_at: DateTime::<Utc>::MIN_UTC,
                },
            },
        }
    }

    pub fn to_storage_value(&self) -> Result<Value, StackError> {
        let mut value = serde_json::to_value(self).map_err(json_storage)?;
        rename_object_keys(&mut value, true);
        Ok(value)
    }

    pub fn from_storage_value(mut value: Value) -> Result<Self, StackError> {
        rename_object_keys(&mut value, false);
        serde_json::from_value(value).map_err(json_storage)
    }

    /// Records the immutable Git revision that produced the successfully applied release.
    /// Returns `false` when this is not a Git-backed Stack state.
    pub fn record_applied_commit(
        &mut self,
        resolved_commit_sha: &str,
        observed_at: DateTime<Utc>,
    ) -> bool {
        let Self::Git {
            recreate_stack_on_new_commit_state,
            ..
        } = self
        else {
            return false;
        };
        recreate_stack_on_new_commit_state.current_commit_sha = resolved_commit_sha.to_owned();
        recreate_stack_on_new_commit_state.remote_commit_sha = None;
        recreate_stack_on_new_commit_state.last_checked_at = observed_at;
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBindingSnapshot {
    pub name: String,
    pub kind: String,
    pub scope: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_delivery_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
}

impl ResourceBindingSnapshot {
    pub fn list_to_storage_value(values: &[Self]) -> Result<Value, StackError> {
        let mut value = serde_json::to_value(values).map_err(json_storage)?;
        rename_object_keys(&mut value, true);
        Ok(value)
    }

    pub fn list_from_storage_value(mut value: Value) -> Result<Vec<Self>, StackError> {
        rename_object_keys(&mut value, false);
        serde_json::from_value(value).map_err(json_storage)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackReleaseSource {
    pub source_type: StackSource,
    pub git_repository_id: Option<Uuid>,
    pub git_repository_name: Option<String>,
    pub branch: Option<String>,
    pub requested_commit_sha: Option<String>,
    pub resolved_commit_sha: String,
    #[serde(default)]
    pub compose_paths: Vec<String>,
    #[serde(default)]
    pub env_file_paths: Vec<String>,
    pub git_repository_url: Option<String>,
    pub working_directory: Option<String>,
    pub watch_paths: Option<Vec<String>>,
    pub compose_env_files_from_repo: Option<Vec<String>>,
    pub compose_digest: Option<String>,
}

impl StackReleaseSource {
    pub fn to_storage_value(&self) -> Result<Value, StackError> {
        let mut value = serde_json::to_value(self).map_err(json_storage)?;
        rename_object_keys(&mut value, true);
        Ok(value)
    }

    pub fn from_storage_value(mut value: Value) -> Result<Self, StackError> {
        rename_object_keys(&mut value, false);
        serde_json::from_value(value).map_err(json_storage)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StackError {
    #[error("{0}")]
    Validation(String),
    #[error("Stack was not found.")]
    NotFound,
    #[error("This operation is not authorized.")]
    Forbidden,
    #[error("License capability '{0}' is unavailable.")]
    LicenseRequired(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("Stack runtime is unavailable: {0}")]
    Runtime(String),
    #[error("Stack runtime rejected the operation: {0}")]
    RuntimeRejected(String),
    #[error("Stack persistence failed: {0}")]
    Storage(String),
    #[error("Stack operation was cancelled.")]
    Cancelled,
}

pub(crate) fn validation(message: &str) -> StackError {
    StackError::Validation(message.to_owned())
}

pub(crate) fn json_storage(error: serde_json::Error) -> StackError {
    StackError::Storage(format!("invalid persisted Stack data: {error}"))
}

pub(crate) fn validate_project_name(value: &str) -> Result<(), StackError> {
    if value.is_empty()
        || value.len() > 63
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
        || !value
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
    {
        return Err(validation("The Stack project name is invalid."));
    }
    Ok(())
}

pub(crate) fn validate_relative_optional(
    value: Option<&str>,
    name: &str,
) -> Result<(), StackError> {
    if let Some(value) = value {
        validate_relative(value, name)?;
    }
    Ok(())
}

pub(crate) fn validate_relative(value: &str, name: &str) -> Result<(), StackError> {
    let normalized = value.replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.split('/').any(|segment| segment == "..")
        || normalized.contains('\0')
    {
        return Err(validation(&format!(
            "{name} must stay inside the Stack source directory."
        )));
    }
    Ok(())
}

pub(crate) fn normalize_name(name: &mut String) -> Result<(), StackError> {
    *name = name.trim().to_owned();
    if name.is_empty() || name.len() > 100 {
        return Err(validation(
            "Stack name is required and must not exceed 100 characters.",
        ));
    }
    Ok(())
}

pub(crate) fn normalize_description(value: &mut Option<String>) -> Result<(), StackError> {
    *value = citadel_primitives::normalization::optional_text(value.take());
    if value.as_ref().is_some_and(|item| item.len() > 2_000) {
        return Err(validation(
            "Stack description must not exceed 2000 characters.",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackAction {
    Start,
    Stop,
    Pause,
    Resume,
    Restart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackImportKind {
    ComposeProject,
    SwarmStack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectRuntimeService {
    pub name: String,

    pub image: Option<String>,
    pub container_count: usize,
    pub states: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackAdoptionIssue {
    pub code: String,
    pub message: String,
    pub severity: String,

    pub field_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectServiceComparison {
    pub name: String,
    pub runtime_container_count: usize,

    pub runtime_image: Option<String>,
    pub defined_in_source: bool,

    pub source_image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectImportValidation {
    pub services: Vec<ComposeProjectServiceComparison>,
    pub issues: Vec<StackAdoptionIssue>,
    pub preview_fingerprint: String,
    pub importable_sensitive_environment_names: Vec<String>,
    pub can_import_sensitive_environment_values: bool,
}

pub(crate) use citadel_primitives::merge_json;

pub(crate) fn rename_object_keys(value: &mut Value, pascal: bool) {
    use citadel_primitives::json_keys::{PropertyCase, map_property_keys};
    let case = if pascal {
        PropertyCase::Pascal
    } else {
        PropertyCase::Camel
    };
    *value = map_property_keys(value.take(), case, &[]);
}

pub(crate) fn normalize_tags(values: &[Uuid]) -> Vec<Uuid> {
    let mut values = values
        .iter()
        .copied()
        .filter(|value| !value.is_nil())
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_normalization_retains_stack_specific_limits() {
        let mut name = " stack project ".to_owned();
        normalize_name(&mut name).unwrap();
        assert_eq!(name, "stack project");
        assert!(normalize_name(&mut "a".repeat(101)).is_err());
        let mut description = Some(" \t ".to_owned());
        normalize_description(&mut description).unwrap();
        assert_eq!(description, None);
        assert!(normalize_description(&mut Some("é".repeat(1_000))).is_ok());
        assert!(normalize_description(&mut Some("é".repeat(1_001))).is_err());
    }

    #[test]
    fn successful_git_apply_advances_and_clears_the_update_state() {
        let previous = Utc::now() - chrono::Duration::hours(1);
        let observed = Utc::now();
        let mut state = StackUpdateState::Git {
            recreate_stack_on_new_image_state: RecreateStackOnNewImageState::default(),
            recreate_stack_on_new_commit_state: RecreateStackOnNewCommitState {
                current_commit_sha: "old".to_owned(),
                remote_commit_sha: Some("new".to_owned()),
                last_checked_at: previous,
            },
        };

        assert!(state.record_applied_commit("new", observed));
        let StackUpdateState::Git {
            recreate_stack_on_new_commit_state,
            ..
        } = state
        else {
            unreachable!();
        };
        assert_eq!(recreate_stack_on_new_commit_state.current_commit_sha, "new");
        assert!(
            recreate_stack_on_new_commit_state
                .remote_commit_sha
                .is_none()
        );
        assert_eq!(recreate_stack_on_new_commit_state.last_checked_at, observed);
    }

    #[test]
    fn web_editor_apply_does_not_create_git_update_state() {
        let mut state = StackUpdateState::WebEditor {
            recreate_stack_on_new_image_state: RecreateStackOnNewImageState::default(),
        };
        assert!(!state.record_applied_commit("commit", Utc::now()));
    }

    #[test]
    fn applied_build_binding_records_one_exact_artifact() {
        let project_id = Uuid::now_v7();
        let run_id = Uuid::now_v7();
        let applied_at = Utc::now();
        let mut binding = StackBuildImageBinding {
            service_name: "api".to_owned(),
            build_project_id: project_id,
            redeploy_on_build: true,
            resolved_image_reference: None,
            resolved_digest: None,
            resolved_build_run_id: None,
            applied_image_reference: None,
            applied_digest: None,
            applied_build_run_id: None,
            applied_at: None,
        };
        binding.record_applied(
            &ResolvedStackBuildImageBinding {
                service_name: "api".to_owned(),
                build_project_id: project_id,
                image_reference: "registry.test/api@sha256:abc".to_owned(),
                digest: Some("sha256:abc".to_owned()),
                build_run_id: Some(run_id),
            },
            applied_at,
        );
        assert_eq!(binding.applied_build_run_id, Some(run_id));
        assert_eq!(binding.resolved_build_run_id, Some(run_id));
        assert_eq!(binding.applied_at, Some(applied_at));
    }
}

#[cfg(test)]
mod status_tests {
    use super::StackReleaseStatus;

    #[test]
    fn release_statuses_roundtrip_and_unknown_storage_values_are_rejected() {
        for status in StackReleaseStatus::ALL {
            assert_eq!(
                status.as_str().parse::<StackReleaseStatus>().unwrap(),
                *status
            );
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
        for invalid in ["", "healthy", "Timedout", "Stopped ", "Invalid"] {
            assert!(invalid.parse::<StackReleaseStatus>().is_err());
        }
    }
}
