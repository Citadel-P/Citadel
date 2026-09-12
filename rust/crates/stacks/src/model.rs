use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

impl StackReleaseStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Created => "Created",
            Self::Applying => "Applying",
            Self::Healthy => "Healthy",
            Self::Pending => "Pending",
            Self::Paused => "Paused",
            Self::Degraded => "Degraded",
            Self::Failed => "Failed",
            Self::Stopped => "Stopped",
            Self::TimedOut => "TimedOut",
        }
    }

    pub fn parse(value: &str) -> Result<Self, StackError> {
        match value {
            "Unknown" => Ok(Self::Unknown),
            "Created" => Ok(Self::Created),
            "Applying" => Ok(Self::Applying),
            "Healthy" => Ok(Self::Healthy),
            "Pending" => Ok(Self::Pending),
            "Paused" => Ok(Self::Paused),
            "Degraded" => Ok(Self::Degraded),
            "Failed" => Ok(Self::Failed),
            "Stopped" => Ok(Self::Stopped),
            "TimedOut" => Ok(Self::TimedOut),
            other => Err(StackError::Storage(format!(
                "invalid persisted Stack release status '{other}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackOrchestrationMode {
    DockerCompose,
    DockerSwarm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum StackDriftMode {
    Disabled,
    DetectOnly,
    AutoFix,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StackCommand {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default = "default_command_path")]
    pub path: String,
}

fn default_command_path() -> String {
    "./".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStackBuildImageBinding {
    pub service_name: String,
    pub build_project_id: Uuid,
    pub image_reference: String,
    pub digest: Option<String>,
    pub build_run_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum WebhookProvider {
    GitHub,
    GitLab,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum WebhookAuthScheme {
    GitHubHmacSha256,
    GitLabSignedToken,
    GitLabLegacyToken,
    BearerToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StackWebhookConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_webhook_provider")]
    pub provider: WebhookProvider,
    #[serde(default = "default_webhook_auth")]
    pub auth_scheme: WebhookAuthScheme,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_filter: Option<String>,
    #[serde(default)]
    pub force_deploy: bool,
}

const fn default_webhook_provider() -> WebhookProvider {
    WebhookProvider::GitHub
}

const fn default_webhook_auth() -> WebhookAuthScheme {
    WebhookAuthScheme::GitHubHmacSha256
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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
                let webhook = webhook
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(json_storage)?;
                citadel_resources::validate_webhook(webhook.as_ref())
                    .map_err(|error| validation(&error.to_string()))?;
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

fn pascalize_stack_spec(mut value: Value) -> Value {
    rename_object_keys(&mut value, true);
    value
}

fn camelize_stack_spec(mut value: Value) -> Value {
    rename_object_keys(&mut value, false);
    value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecreateStackOnNewImageState {
    #[serde(default)]
    pub auto_update_states: Vec<ImageUpdateState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecreateStackOnNewCommitState {
    pub current_commit_sha: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_commit_sha: Option<String>,
    pub last_checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackReleaseView {
    pub id: Uuid,
    pub stack_id: Uuid,
    pub platform_id: Uuid,
    pub status: StackReleaseStatus,
    pub version: String,
    pub spec: StackSpec,
    pub source: Option<StackReleaseSource>,
    pub resource_bindings: Option<Vec<ResourceBindingSnapshot>>,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    pub actor_name: String,
    pub actor_type: String,
    pub platform_status: String,
    pub platform_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackReleasesView {
    pub releases: Vec<StackReleaseView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = stacks::model::TagSummary)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_delete: bool,
    pub can_view_logs: bool,
    pub can_inspect: bool,
    pub can_open_terminal: bool,
    pub can_pull: bool,
    pub can_apply: bool,
    pub can_view_resource_bindings: bool,
    pub can_view_releases: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[schema(as = stacks::model::ResourceCapabilities)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub stack_update_state: StackUpdateState,
    pub drift_policy: StackDriftPolicy,
    pub status: StackReleaseStatus,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    pub control_state: String,
    pub current_stack_release_id: Uuid,
    pub platform_type: String,
    pub platform_id: Option<Uuid>,
    pub version: Option<String>,
    pub spec: Option<StackSpec>,
    pub source: Option<StackReleaseSource>,
    pub resource_bindings: Option<Vec<ResourceBindingSnapshot>>,
    pub platform_status: String,
    pub platform_name: Option<String>,
    pub tags: Vec<TagSummary>,
    pub latest_activity_view: Option<Value>,
    pub capabilities: Option<StackCapabilities>,
    pub row_version: i64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StacksView {
    pub stacks: Vec<StackView>,
    pub capabilities: ResourceCapabilities,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackConfigView {
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub platform_type: String,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub spec: StackSpec,
    pub stack_update_state: StackUpdateState,
    pub drift_policy: StackDriftPolicy,
    pub row_version: i64,
}

impl From<StackView> for StackConfigView {
    fn from(value: StackView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            platform_id: value.platform_id.unwrap_or_default(),
            platform_type: value.platform_type,
            description: value.description,
            stack_source: value.stack_source,
            spec: value
                .spec
                .expect("Stack persistence always includes a current release"),
            stack_update_state: value.stack_update_state,
            drift_policy: value.drift_policy,
            row_version: value.row_version,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateStackInput {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub spec: StackSpec,
    pub drift_policy: Option<StackDriftPolicy>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
    #[serde(default)]
    pub duplicate_source: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchStackInput {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub platform_id: Option<Uuid>,
    #[serde(default, deserialize_with = "deserialize_present_nullable_string")]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub stack_source: Option<StackSource>,
    #[serde(default)]
    pub spec: Option<Value>,
    #[serde(default)]
    pub drift_policy: Option<StackDriftPolicy>,
    pub row_version: Option<i64>,
}

fn deserialize_present_nullable_string<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameStackInput {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyStackInput {
    pub id: Uuid,
    #[serde(default)]
    pub recreate: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RollbackStackInput {
    pub stack_id: Uuid,
    pub release_id: Uuid,
}

#[derive(Debug, Clone, Default)]
pub struct StackFilter {
    pub tags: Vec<String>,
    pub platform_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackStreamItem {
    pub event_type: StackApplyEventType,
    pub message: Option<String>,
    pub exit_code: Option<i32>,
    pub stack_status: Option<StackReleaseStatus>,
    pub severity: Option<String>,
}

impl Serialize for StackStreamItem {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;

        let mut item = serializer.serialize_map(None)?;
        item.serialize_entry("type", &self.event_type)?;
        if let Some(message) = &self.message {
            // Docker also writes normal progress to stderr. The public `message`
            // field is reserved for errors; .NET uses `progressMessage` otherwise.
            let field = if self.exit_code.is_some_and(|code| code != 0) {
                "message"
            } else {
                "progressMessage"
            };
            item.serialize_entry(field, message)?;
        }
        if let Some(code) = self.exit_code {
            item.serialize_entry("exitCode", &code)?;
        }
        if let Some(status) = self.stack_status {
            item.serialize_entry("stackStatus", &status)?;
        }
        if let Some(severity) = &self.severity {
            item.serialize_entry("severity", severity)?;
        }
        item.end()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StackApplyEventType {
    Unknown,
    StdOut,
    StdErr,
    SystemMessage,
    CommandCompleted,
}

impl StackStreamItem {
    #[must_use]
    pub fn system(message: impl Into<String>) -> Self {
        Self {
            event_type: StackApplyEventType::SystemMessage,
            message: Some(message.into()),
            exit_code: None,
            stack_status: None,
            severity: None,
        }
    }

    #[must_use]
    pub fn completed(status: StackReleaseStatus, message: impl Into<String>) -> Self {
        Self {
            event_type: StackApplyEventType::CommandCompleted,
            message: Some(message.into()),
            exit_code: Some(if status == StackReleaseStatus::Healthy {
                0
            } else {
                1
            }),
            stack_status: Some(status),
            severity: Some(if status == StackReleaseStatus::Healthy {
                "success".to_owned()
            } else {
                "error".to_owned()
            }),
        }
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

fn json_storage(error: serde_json::Error) -> StackError {
    StackError::Storage(format!("invalid persisted Stack data: {error}"))
}

fn validate_project_name(value: &str) -> Result<(), StackError> {
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

fn validate_relative_optional(value: Option<&str>, name: &str) -> Result<(), StackError> {
    if let Some(value) = value {
        validate_relative(value, name)?;
    }
    Ok(())
}

fn validate_relative(value: &str, name: &str) -> Result<(), StackError> {
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
    *value = value.take().and_then(|item| {
        let trimmed = item.trim().to_owned();
        (!trimmed.is_empty()).then_some(trimmed)
    });
    if value.as_ref().is_some_and(|item| item.len() > 2_000) {
        return Err(validation(
            "Stack description must not exceed 2000 characters.",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StackApplyOptions {
    pub expected_version: Option<i64>,
    pub service_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackOperationClaim {
    pub stack_id: Uuid,
    pub release_id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    pub project_name: String,
    pub platform_type: String,
    pub spec: StackSpec,
    pub row_version: i64,
    pub actor_id: Uuid,
    pub operation: String,
    /// Empty means the whole Stack; persisted with the operation for recovery.
    pub service_names: Vec<String>,
}

/// An immutable, bounded Stack source snapshot prepared for one Apply attempt.
/// Paths are repository-relative and are validated again by each runtime
/// transport before they are written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackApplySource {
    pub files: Vec<StackSourceFile>,
    pub compose_paths: Vec<String>,
    pub env_file_paths: Vec<String>,
    pub working_directory: String,
    pub labels_override_path: Option<String>,
    pub resolved_commit_sha: Option<String>,
}

impl StackApplySource {
    pub fn compose_contents(&self) -> Result<Vec<String>, StackError> {
        self.compose_paths
            .iter()
            .map(|path| {
                let file = self
                    .files
                    .iter()
                    .find(|file| file.relative_path == *path)
                    .ok_or_else(|| validation(&format!("Compose file '{path}' is unavailable.")))?;
                String::from_utf8(file.content.clone())
                    .map_err(|_| validation(&format!("Compose file '{path}' is not valid UTF-8.")))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackSourceFile {
    pub relative_path: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDeletionClaim {
    pub stack_id: Uuid,
    pub platform_id: Uuid,
    pub project_name: String,
    pub platform_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackStateClaim {
    pub stack_id: Uuid,
    pub release_id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    pub project_name: String,
    pub platform_type: String,
    pub previous_status: StackReleaseStatus,
    pub actor_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackRuntimeResult {
    pub status: StackReleaseStatus,
    pub messages: Vec<StackStreamItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackBinding {
    pub name: String,
    pub value: zeroize::Zeroizing<String>,
    pub secret: bool,
    pub snapshot: ResourceBindingSnapshot,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedStackBindings {
    pub entries: Vec<StackBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackDuplicateDraftView {
    pub draft: Value,
    pub warnings: Vec<DuplicateDraftWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateDraftWarning {
    pub code: String,
    pub message: String,
    pub field_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackRuntimeService {
    pub docker_service_id: String,
    pub name: String,
    pub version_index: i64,
    pub desired_tasks: i32,
    pub running_tasks: i32,
    pub update_state: String,
    pub update_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackRuntimeContainer {
    pub docker_container_id: String,
    pub service_name: String,
    pub state: String,
    pub health: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackRuntimeSnapshot {
    pub containers: Vec<StackRuntimeContainer>,
    pub services: Vec<StackRuntimeService>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackDriftReport {
    pub stack_id: Uuid,
    pub platform_id: Uuid,
    pub has_drift: bool,
    pub has_auto_fixable_drift: bool,
    pub has_structural_drift: bool,
    pub drifts: Vec<StackDrift>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDriftMonitorFailure {
    pub stack_id: Uuid,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDriftMonitorResult {
    pub checked: usize,
    pub reconciled: usize,
    pub failures: Vec<StackDriftMonitorFailure>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(tag = "$type", rename_all_fields = "camelCase")]
pub enum StackDrift {
    MissingContainer {
        service_name: String,
    },
    ExtraContainer {
        container_id: String,
        service_name: String,
    },
    ContainerStopped {
        container_id: String,
        service_name: String,
    },
    ContainerPaused {
        container_id: String,
        service_name: String,
    },
    ContainerUnhealthy {
        container_id: String,
        service_name: String,
        health_status: Option<String>,
    },
    ImageMismatch {
        service_name: String,
        expected_image: String,
        actual_image: String,
    },
    ConfigHashMismatch {
        service_name: String,
        expected_hash: Option<String>,
        actual_hash: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum StackReconciliationStatus {
    NoDrift,
    Reconciled,
    Partial,
    RequiresReapply,
    Disabled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum StackReconciliationActionType {
    StartContainer,
    ResumeContainer,
    RemoveContainer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackReconciliationAction {
    pub container_id: String,
    pub service_name: String,
    pub action: StackReconciliationActionType,
    pub succeeded: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackReconciliationResult {
    pub stack_id: Uuid,
    pub status: StackReconciliationStatus,
    pub before_report: StackDriftReport,
    pub after_report: Option<StackDriftReport>,
    pub actions: Vec<StackReconciliationAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackAction {
    Start,
    Stop,
    Pause,
    Resume,
    Restart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum StackImportKind {
    ComposeProject,
    SwarmStack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportComposeProjectInput {
    pub name: String,
    pub platform_id: Uuid,
    #[serde(default, skip_deserializing, skip_serializing)]
    pub project_name: String,
    pub description: Option<String>,
    pub spec: StackSpec,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
    pub import_kind: StackImportKind,
    pub preview_fingerprint: String,
    #[serde(default)]
    pub detected_secret_values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectRuntimeService {
    pub name: String,
    pub image: Option<String>,
    pub container_count: usize,
    pub states: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectImportSourceView {
    pub platform_id: Uuid,
    pub platform_name: String,
    pub project_name: String,
    pub container_ids: Vec<String>,
    pub container_names: Vec<String>,
    pub services: Vec<ComposeProjectRuntimeService>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectStackDraftView {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub drift_policy: StackDriftPolicy,
    pub tag_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackAdoptionIssue {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub field_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectImportDraftView {
    pub import_kind: StackImportKind,
    pub source: ComposeProjectImportSourceView,
    pub draft: ComposeProjectStackDraftView,
    pub issues: Vec<StackAdoptionIssue>,
    pub runtime_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectServiceComparison {
    pub name: String,
    pub runtime_container_count: usize,
    pub runtime_image: Option<String>,
    pub defined_in_source: bool,
    pub source_image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectImportValidation {
    pub services: Vec<ComposeProjectServiceComparison>,
    pub issues: Vec<StackAdoptionIssue>,
    pub preview_fingerprint: String,
    pub importable_sensitive_environment_names: Vec<String>,
    pub can_import_sensitive_environment_values: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackImportClaim {
    pub platform_id: Uuid,
    pub platform_name: String,
    pub project_name: String,
    pub import_kind: StackImportKind,
    pub runtime_fingerprint: String,
    pub service_names: Vec<String>,
    pub container_ids: Vec<String>,
    pub container_names: Vec<String>,
    pub services: Vec<ComposeProjectRuntimeService>,
}

pub(crate) fn merge_json(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(base), Value::Object(patch)) => {
            for (key, value) in patch {
                if value.is_null() {
                    base.remove(key);
                } else {
                    merge_json(base.entry(key).or_insert(Value::Null), value);
                }
            }
        }
        (base, patch) => *base = patch.clone(),
    }
}

fn rename_object_keys(value: &mut Value, pascal: bool) {
    match value {
        Value::Object(map) => {
            let old = std::mem::take(map);
            for (key, mut child) in old {
                rename_object_keys(&mut child, pascal);
                let mut chars = key.chars();
                let renamed = match chars.next() {
                    Some(first) if pascal => {
                        first.to_ascii_uppercase().to_string() + chars.as_str()
                    }
                    Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
                    None => key,
                };
                map.insert(renamed, child);
            }
        }
        Value::Array(values) => {
            for child in values {
                rename_object_keys(child, pascal);
            }
        }
        _ => {}
    }
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
    fn stack_stream_serializes_normal_output_as_progress() {
        for event_type in [
            StackApplyEventType::SystemMessage,
            StackApplyEventType::StdOut,
            StackApplyEventType::StdErr,
        ] {
            let mut item = StackStreamItem::system("Container beszel Started");
            item.event_type = event_type;
            let wire = serde_json::to_value(&item).unwrap();
            assert_eq!(wire["progressMessage"], "Container beszel Started");
            assert!(
                wire.get("message").is_none(),
                "Normal output must not populate the error field"
            );
        }
    }

    #[test]
    fn stack_stream_keeps_success_and_failure_distinct() {
        let success = serde_json::to_value(StackStreamItem::completed(
            StackReleaseStatus::Healthy,
            "Stack deployment completed.",
        ))
        .unwrap();
        assert_eq!(success["progressMessage"], "Stack deployment completed.");
        assert_eq!(success["exitCode"], 0);
        assert_eq!(success["severity"], "success");
        assert!(success.get("message").is_none());
        let failed = serde_json::to_value(StackStreamItem::completed(
            StackReleaseStatus::Failed,
            "Docker deployment failed.",
        ))
        .unwrap();
        assert_eq!(failed["message"], "Docker deployment failed.");
        assert_eq!(failed["exitCode"], 1);
        assert_eq!(failed["severity"], "error");
        assert!(failed.get("progressMessage").is_none());
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
