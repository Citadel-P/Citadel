use crate::api::resources::bindings::spec::{
    ResourceBindingKind, ResourceBindingScope, SecretDeliveryMode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum StackSource {
    #[serde(alias = "webEditor", alias = "webeditor")]
    WebEditor,
    #[serde(alias = "git")]
    Git,
}

impl From<citadel_stacks::StackSource> for StackSource {
    fn from(value: citadel_stacks::StackSource) -> Self {
        match value {
            citadel_stacks::StackSource::WebEditor => Self::WebEditor,
            citadel_stacks::StackSource::Git => Self::Git,
        }
    }
}

impl From<StackSource> for citadel_stacks::StackSource {
    fn from(value: StackSource) -> Self {
        match value {
            StackSource::WebEditor => Self::WebEditor,
            StackSource::Git => Self::Git,
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

impl From<citadel_stacks::StackUpdateBehavior> for StackUpdateBehavior {
    fn from(value: citadel_stacks::StackUpdateBehavior) -> Self {
        match value {
            citadel_stacks::StackUpdateBehavior::Disabled => Self::Disabled,
            citadel_stacks::StackUpdateBehavior::Notify => Self::Notify,
            citadel_stacks::StackUpdateBehavior::ServiceAutoDeploy => Self::ServiceAutoDeploy,
            citadel_stacks::StackUpdateBehavior::StackAutoDeploy => Self::StackAutoDeploy,
        }
    }
}

impl From<StackUpdateBehavior> for citadel_stacks::StackUpdateBehavior {
    fn from(value: StackUpdateBehavior) -> Self {
        match value {
            StackUpdateBehavior::Disabled => Self::Disabled,
            StackUpdateBehavior::Notify => Self::Notify,
            StackUpdateBehavior::ServiceAutoDeploy => Self::ServiceAutoDeploy,
            StackUpdateBehavior::StackAutoDeploy => Self::StackAutoDeploy,
        }
    }
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

impl From<citadel_stacks::StackReleaseStatus> for StackReleaseStatus {
    fn from(value: citadel_stacks::StackReleaseStatus) -> Self {
        match value {
            citadel_stacks::StackReleaseStatus::Unknown => Self::Unknown,
            citadel_stacks::StackReleaseStatus::Created => Self::Created,
            citadel_stacks::StackReleaseStatus::Applying => Self::Applying,
            citadel_stacks::StackReleaseStatus::Healthy => Self::Healthy,
            citadel_stacks::StackReleaseStatus::Pending => Self::Pending,
            citadel_stacks::StackReleaseStatus::Paused => Self::Paused,
            citadel_stacks::StackReleaseStatus::Degraded => Self::Degraded,
            citadel_stacks::StackReleaseStatus::Failed => Self::Failed,
            citadel_stacks::StackReleaseStatus::Stopped => Self::Stopped,
            citadel_stacks::StackReleaseStatus::TimedOut => Self::TimedOut,
        }
    }
}

impl From<StackReleaseStatus> for citadel_stacks::StackReleaseStatus {
    fn from(value: StackReleaseStatus) -> Self {
        match value {
            StackReleaseStatus::Unknown => Self::Unknown,
            StackReleaseStatus::Created => Self::Created,
            StackReleaseStatus::Applying => Self::Applying,
            StackReleaseStatus::Healthy => Self::Healthy,
            StackReleaseStatus::Pending => Self::Pending,
            StackReleaseStatus::Paused => Self::Paused,
            StackReleaseStatus::Degraded => Self::Degraded,
            StackReleaseStatus::Failed => Self::Failed,
            StackReleaseStatus::Stopped => Self::Stopped,
            StackReleaseStatus::TimedOut => Self::TimedOut,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum StackDriftMode {
    Disabled,
    DetectOnly,
    AutoFix,
}

impl From<citadel_stacks::StackDriftMode> for StackDriftMode {
    fn from(value: citadel_stacks::StackDriftMode) -> Self {
        match value {
            citadel_stacks::StackDriftMode::Disabled => Self::Disabled,
            citadel_stacks::StackDriftMode::DetectOnly => Self::DetectOnly,
            citadel_stacks::StackDriftMode::AutoFix => Self::AutoFix,
        }
    }
}

impl From<StackDriftMode> for citadel_stacks::StackDriftMode {
    fn from(value: StackDriftMode) -> Self {
        match value {
            StackDriftMode::Disabled => Self::Disabled,
            StackDriftMode::DetectOnly => Self::DetectOnly,
            StackDriftMode::AutoFix => Self::AutoFix,
        }
    }
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

impl From<citadel_stacks::StackDriftPolicy> for StackDriftPolicy {
    fn from(value: citadel_stacks::StackDriftPolicy) -> Self {
        Self {
            mode: value.mode.into(),
            alert_on_drift: value.alert_on_drift,
            mark_degraded: value.mark_degraded,
            auto_start_stopped_containers: value.auto_start_stopped_containers,
            auto_resume_paused_containers: value.auto_resume_paused_containers,
            remove_extra_containers: value.remove_extra_containers,
        }
    }
}

impl From<StackDriftPolicy> for citadel_stacks::StackDriftPolicy {
    fn from(value: StackDriftPolicy) -> Self {
        Self {
            mode: value.mode.into(),
            alert_on_drift: value.alert_on_drift,
            mark_degraded: value.mark_degraded,
            auto_start_stopped_containers: value.auto_start_stopped_containers,
            auto_resume_paused_containers: value.auto_resume_paused_containers,
            remove_extra_containers: value.remove_extra_containers,
        }
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

impl From<citadel_stacks::StackCommand> for StackCommand {
    fn from(value: citadel_stacks::StackCommand) -> Self {
        Self {
            commands: value.commands,
            path: value.path,
        }
    }
}

impl From<StackCommand> for citadel_stacks::StackCommand {
    fn from(value: StackCommand) -> Self {
        Self {
            commands: value.commands,
            path: value.path,
        }
    }
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

impl From<citadel_stacks::StackBuildImageBinding> for StackBuildImageBinding {
    fn from(value: citadel_stacks::StackBuildImageBinding) -> Self {
        Self {
            service_name: value.service_name,
            build_project_id: value.build_project_id,
            redeploy_on_build: value.redeploy_on_build,
            resolved_image_reference: value.resolved_image_reference,
            resolved_digest: value.resolved_digest,
            resolved_build_run_id: value.resolved_build_run_id,
            applied_image_reference: value.applied_image_reference,
            applied_digest: value.applied_digest,
            applied_build_run_id: value.applied_build_run_id,
            applied_at: value.applied_at,
        }
    }
}

impl From<StackBuildImageBinding> for citadel_stacks::StackBuildImageBinding {
    fn from(value: StackBuildImageBinding) -> Self {
        Self {
            service_name: value.service_name,
            build_project_id: value.build_project_id,
            redeploy_on_build: value.redeploy_on_build,
            resolved_image_reference: value.resolved_image_reference,
            resolved_digest: value.resolved_digest,
            resolved_build_run_id: value.resolved_build_run_id,
            applied_image_reference: value.applied_image_reference,
            applied_digest: value.applied_digest,
            applied_build_run_id: value.applied_build_run_id,
            applied_at: value.applied_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum WebhookProvider {
    GitHub,
    GitLab,
    Generic,
}

impl From<citadel_stacks::WebhookProvider> for WebhookProvider {
    fn from(value: citadel_stacks::WebhookProvider) -> Self {
        match value {
            citadel_stacks::WebhookProvider::GitHub => Self::GitHub,
            citadel_stacks::WebhookProvider::GitLab => Self::GitLab,
            citadel_stacks::WebhookProvider::Generic => Self::Generic,
        }
    }
}

impl From<WebhookProvider> for citadel_stacks::WebhookProvider {
    fn from(value: WebhookProvider) -> Self {
        match value {
            WebhookProvider::GitHub => Self::GitHub,
            WebhookProvider::GitLab => Self::GitLab,
            WebhookProvider::Generic => Self::Generic,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum WebhookAuthScheme {
    GitHubHmacSha256,
    GitLabSignedToken,
    GitLabLegacyToken,
    BearerToken,
}

impl From<citadel_stacks::WebhookAuthScheme> for WebhookAuthScheme {
    fn from(value: citadel_stacks::WebhookAuthScheme) -> Self {
        match value {
            citadel_stacks::WebhookAuthScheme::GitHubHmacSha256 => Self::GitHubHmacSha256,
            citadel_stacks::WebhookAuthScheme::GitLabSignedToken => Self::GitLabSignedToken,
            citadel_stacks::WebhookAuthScheme::GitLabLegacyToken => Self::GitLabLegacyToken,
            citadel_stacks::WebhookAuthScheme::BearerToken => Self::BearerToken,
        }
    }
}

impl From<WebhookAuthScheme> for citadel_stacks::WebhookAuthScheme {
    fn from(value: WebhookAuthScheme) -> Self {
        match value {
            WebhookAuthScheme::GitHubHmacSha256 => Self::GitHubHmacSha256,
            WebhookAuthScheme::GitLabSignedToken => Self::GitLabSignedToken,
            WebhookAuthScheme::GitLabLegacyToken => Self::GitLabLegacyToken,
            WebhookAuthScheme::BearerToken => Self::BearerToken,
        }
    }
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

impl From<citadel_stacks::StackWebhookConfig> for StackWebhookConfig {
    fn from(value: citadel_stacks::StackWebhookConfig) -> Self {
        Self {
            enabled: value.enabled,
            provider: value.provider.into(),
            auth_scheme: value.auth_scheme.into(),
            secret: value.secret,
            branch_filter: value.branch_filter,
            force_deploy: value.force_deploy,
        }
    }
}

impl From<StackWebhookConfig> for citadel_stacks::StackWebhookConfig {
    fn from(value: StackWebhookConfig) -> Self {
        Self {
            enabled: value.enabled,
            provider: value.provider.into(),
            auth_scheme: value.auth_scheme.into(),
            secret: value.secret,
            branch_filter: value.branch_filter,
            force_deploy: value.force_deploy,
        }
    }
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

impl From<citadel_stacks::StackSpec> for StackSpec {
    fn from(value: citadel_stacks::StackSpec) -> Self {
        match value {
            citadel_stacks::StackSpec::WebEditor {
                compose_file,
                update_behavior,
                common,
            } => Self::WebEditor {
                compose_file,
                update_behavior: update_behavior.into(),
                common: common.into(),
            },
            citadel_stacks::StackSpec::Git {
                git_repo_id,
                branch,
                commit_sha,
                update_behavior,
                webhook,
                compose_paths,
                working_directory,
                compose_env_files_from_repo,
                watch_paths,
                additional_env_file_from_repo,
                common,
            } => Self::Git {
                git_repo_id,
                branch,
                commit_sha,
                update_behavior: update_behavior.into(),
                webhook: webhook.map(|item| item.into()),
                compose_paths,
                working_directory,
                compose_env_files_from_repo,
                watch_paths,
                additional_env_file_from_repo,
                common: common.into(),
            },
        }
    }
}

impl From<StackSpec> for citadel_stacks::StackSpec {
    fn from(value: StackSpec) -> Self {
        match value {
            StackSpec::WebEditor {
                compose_file,
                update_behavior,
                common,
            } => Self::WebEditor {
                compose_file,
                update_behavior: update_behavior.into(),
                common: common.into(),
            },
            StackSpec::Git {
                git_repo_id,
                branch,
                commit_sha,
                update_behavior,
                webhook,
                compose_paths,
                working_directory,
                compose_env_files_from_repo,
                watch_paths,
                additional_env_file_from_repo,
                common,
            } => Self::Git {
                git_repo_id,
                branch,
                commit_sha,
                update_behavior: update_behavior.into(),
                webhook: webhook.map(|item| item.into()),
                compose_paths,
                working_directory,
                compose_env_files_from_repo,
                watch_paths,
                additional_env_file_from_repo,
                common: common.into(),
            },
        }
    }
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

impl From<citadel_stacks::StackSpecCommon> for StackSpecCommon {
    fn from(value: citadel_stacks::StackSpecCommon) -> Self {
        Self {
            project_name: value.project_name,
            pre_deploy: value.pre_deploy.map(|item| item.into()),
            post_deploy: value.post_deploy.map(|item| item.into()),
            env_file_path: value.env_file_path,
            registry_id: value.registry_id,
            destroy_before_deploy: value.destroy_before_deploy,
            build_image_bindings: value
                .build_image_bindings
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<StackSpecCommon> for citadel_stacks::StackSpecCommon {
    fn from(value: StackSpecCommon) -> Self {
        Self {
            project_name: value.project_name,
            pre_deploy: value.pre_deploy.map(|item| item.into()),
            post_deploy: value.post_deploy.map(|item| item.into()),
            env_file_path: value.env_file_path,
            registry_id: value.registry_id,
            destroy_before_deploy: value.destroy_before_deploy,
            build_image_bindings: value
                .build_image_bindings
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
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

impl From<citadel_stacks::StackUpdateState> for StackUpdateState {
    fn from(value: citadel_stacks::StackUpdateState) -> Self {
        match value {
            citadel_stacks::StackUpdateState::WebEditor {
                recreate_stack_on_new_image_state,
            } => Self::WebEditor {
                recreate_stack_on_new_image_state: recreate_stack_on_new_image_state.into(),
            },
            citadel_stacks::StackUpdateState::Git {
                recreate_stack_on_new_image_state,
                recreate_stack_on_new_commit_state,
            } => Self::Git {
                recreate_stack_on_new_image_state: recreate_stack_on_new_image_state.into(),
                recreate_stack_on_new_commit_state: recreate_stack_on_new_commit_state.into(),
            },
        }
    }
}

impl From<StackUpdateState> for citadel_stacks::StackUpdateState {
    fn from(value: StackUpdateState) -> Self {
        match value {
            StackUpdateState::WebEditor {
                recreate_stack_on_new_image_state,
            } => Self::WebEditor {
                recreate_stack_on_new_image_state: recreate_stack_on_new_image_state.into(),
            },
            StackUpdateState::Git {
                recreate_stack_on_new_image_state,
                recreate_stack_on_new_commit_state,
            } => Self::Git {
                recreate_stack_on_new_image_state: recreate_stack_on_new_image_state.into(),
                recreate_stack_on_new_commit_state: recreate_stack_on_new_commit_state.into(),
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecreateStackOnNewImageState {
    #[serde(default)]
    pub auto_update_states: Vec<ImageUpdateState>,
}

impl From<citadel_stacks::RecreateStackOnNewImageState> for RecreateStackOnNewImageState {
    fn from(value: citadel_stacks::RecreateStackOnNewImageState) -> Self {
        Self {
            auto_update_states: value
                .auto_update_states
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<RecreateStackOnNewImageState> for citadel_stacks::RecreateStackOnNewImageState {
    fn from(value: RecreateStackOnNewImageState) -> Self {
        Self {
            auto_update_states: value
                .auto_update_states
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecreateStackOnNewCommitState {
    pub current_commit_sha: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_commit_sha: Option<String>,
    pub last_checked_at: DateTime<Utc>,
}

impl From<citadel_stacks::RecreateStackOnNewCommitState> for RecreateStackOnNewCommitState {
    fn from(value: citadel_stacks::RecreateStackOnNewCommitState) -> Self {
        Self {
            current_commit_sha: value.current_commit_sha,
            remote_commit_sha: value.remote_commit_sha,
            last_checked_at: value.last_checked_at,
        }
    }
}

impl From<RecreateStackOnNewCommitState> for citadel_stacks::RecreateStackOnNewCommitState {
    fn from(value: RecreateStackOnNewCommitState) -> Self {
        Self {
            current_commit_sha: value.current_commit_sha,
            remote_commit_sha: value.remote_commit_sha,
            last_checked_at: value.last_checked_at,
        }
    }
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

impl From<citadel_stacks::ImageUpdateState> for ImageUpdateState {
    fn from(value: citadel_stacks::ImageUpdateState) -> Self {
        Self {
            service_name: value.service_name,
            image_name: value.image_name,
            current_digest: value.current_digest,
            remote_digest: value.remote_digest,
            last_checked_at: value.last_checked_at,
            update_available: value.update_available,
        }
    }
}

impl From<ImageUpdateState> for citadel_stacks::ImageUpdateState {
    fn from(value: ImageUpdateState) -> Self {
        Self {
            service_name: value.service_name,
            image_name: value.image_name,
            current_digest: value.current_digest,
            remote_digest: value.remote_digest,
            last_checked_at: value.last_checked_at,
            update_available: value.update_available,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBindingSnapshot {
    pub name: String,
    pub kind: ResourceBindingKind,
    pub scope: ResourceBindingScope,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_delivery_mode: Option<SecretDeliveryMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
}

impl TryFrom<citadel_stacks::ResourceBindingSnapshot> for ResourceBindingSnapshot {
    type Error = serde_json::Error;

    fn try_from(value: citadel_stacks::ResourceBindingSnapshot) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            kind: serde_json::from_value(value.kind.into())?,
            scope: serde_json::from_value(value.scope.into())?,
            value: value.value,
            secret_id: value.secret_id,
            secret_delivery_mode: value
                .secret_delivery_mode
                .map(|value| serde_json::from_value(value.into()))
                .transpose()?,
            target_path: value.target_path,
        })
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

impl From<citadel_stacks::StackReleaseSource> for StackReleaseSource {
    fn from(value: citadel_stacks::StackReleaseSource) -> Self {
        Self {
            source_type: value.source_type.into(),
            git_repository_id: value.git_repository_id,
            git_repository_name: value.git_repository_name,
            branch: value.branch,
            requested_commit_sha: value.requested_commit_sha,
            resolved_commit_sha: value.resolved_commit_sha,
            compose_paths: value.compose_paths,
            env_file_paths: value.env_file_paths,
            git_repository_url: value.git_repository_url,
            working_directory: value.working_directory,
            watch_paths: value.watch_paths,
            compose_env_files_from_repo: value.compose_env_files_from_repo,
            compose_digest: value.compose_digest,
        }
    }
}

impl From<StackReleaseSource> for citadel_stacks::StackReleaseSource {
    fn from(value: StackReleaseSource) -> Self {
        Self {
            source_type: value.source_type.into(),
            git_repository_id: value.git_repository_id,
            git_repository_name: value.git_repository_name,
            branch: value.branch,
            requested_commit_sha: value.requested_commit_sha,
            resolved_commit_sha: value.resolved_commit_sha,
            compose_paths: value.compose_paths,
            env_file_paths: value.env_file_paths,
            git_repository_url: value.git_repository_url,
            working_directory: value.working_directory,
            watch_paths: value.watch_paths,
            compose_env_files_from_repo: value.compose_env_files_from_repo,
            compose_digest: value.compose_digest,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateDraftWarning {
    pub code: String,
    pub message: String,
    #[schema(required = true)]
    pub field_path: Option<String>,
}

impl From<citadel_stacks::DuplicateDraftWarning> for DuplicateDraftWarning {
    fn from(value: citadel_stacks::DuplicateDraftWarning) -> Self {
        Self {
            code: value.code,
            message: value.message,
            field_path: value.field_path,
        }
    }
}

impl From<DuplicateDraftWarning> for citadel_stacks::DuplicateDraftWarning {
    fn from(value: DuplicateDraftWarning) -> Self {
        Self {
            code: value.code,
            message: value.message,
            field_path: value.field_path,
        }
    }
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

impl From<citadel_stacks::StackDriftReport> for StackDriftReport {
    fn from(value: citadel_stacks::StackDriftReport) -> Self {
        Self {
            stack_id: value.stack_id,
            platform_id: value.platform_id,
            has_drift: value.has_drift,
            has_auto_fixable_drift: value.has_auto_fixable_drift,
            has_structural_drift: value.has_structural_drift,
            drifts: value.drifts.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<StackDriftReport> for citadel_stacks::StackDriftReport {
    fn from(value: StackDriftReport) -> Self {
        Self {
            stack_id: value.stack_id,
            platform_id: value.platform_id,
            has_drift: value.has_drift,
            has_auto_fixable_drift: value.has_auto_fixable_drift,
            has_structural_drift: value.has_structural_drift,
            drifts: value.drifts.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(tag = "$type", rename_all_fields = "camelCase")]
pub enum StackDrift {
    MissingContainer {
        #[serde(rename = "serviceName")]
        service_name: String,
    },
    ExtraContainer {
        #[serde(rename = "containerId")]
        container_id: String,
        #[serde(rename = "serviceName")]
        service_name: String,
    },
    ContainerStopped {
        #[serde(rename = "containerId")]
        container_id: String,
        #[serde(rename = "serviceName")]
        service_name: String,
    },
    ContainerPaused {
        #[serde(rename = "containerId")]
        container_id: String,
        #[serde(rename = "serviceName")]
        service_name: String,
    },
    ContainerUnhealthy {
        #[serde(rename = "containerId")]
        container_id: String,
        #[serde(rename = "serviceName")]
        service_name: String,
        #[serde(rename = "healthStatus")]
        health_status: Option<String>,
    },
    ImageMismatch {
        #[serde(rename = "serviceName")]
        service_name: String,
        #[serde(rename = "expectedImage")]
        expected_image: String,
        #[serde(rename = "actualImage")]
        actual_image: String,
    },
    ConfigHashMismatch {
        #[serde(rename = "serviceName")]
        service_name: String,
        #[serde(rename = "expectedHash")]
        expected_hash: Option<String>,
        #[serde(rename = "actualHash")]
        actual_hash: Option<String>,
    },
}

impl From<citadel_stacks::StackDrift> for StackDrift {
    fn from(value: citadel_stacks::StackDrift) -> Self {
        match value {
            citadel_stacks::StackDrift::MissingContainer { service_name } => {
                Self::MissingContainer { service_name }
            }
            citadel_stacks::StackDrift::ExtraContainer {
                container_id,
                service_name,
            } => Self::ExtraContainer {
                container_id,
                service_name,
            },
            citadel_stacks::StackDrift::ContainerStopped {
                container_id,
                service_name,
            } => Self::ContainerStopped {
                container_id,
                service_name,
            },
            citadel_stacks::StackDrift::ContainerPaused {
                container_id,
                service_name,
            } => Self::ContainerPaused {
                container_id,
                service_name,
            },
            citadel_stacks::StackDrift::ContainerUnhealthy {
                container_id,
                service_name,
                health_status,
            } => Self::ContainerUnhealthy {
                container_id,
                service_name,
                health_status,
            },
            citadel_stacks::StackDrift::ImageMismatch {
                service_name,
                expected_image,
                actual_image,
            } => Self::ImageMismatch {
                service_name,
                expected_image,
                actual_image,
            },
            citadel_stacks::StackDrift::ConfigHashMismatch {
                service_name,
                expected_hash,
                actual_hash,
            } => Self::ConfigHashMismatch {
                service_name,
                expected_hash,
                actual_hash,
            },
        }
    }
}

impl From<StackDrift> for citadel_stacks::StackDrift {
    fn from(value: StackDrift) -> Self {
        match value {
            StackDrift::MissingContainer { service_name } => {
                Self::MissingContainer { service_name }
            }
            StackDrift::ExtraContainer {
                container_id,
                service_name,
            } => Self::ExtraContainer {
                container_id,
                service_name,
            },
            StackDrift::ContainerStopped {
                container_id,
                service_name,
            } => Self::ContainerStopped {
                container_id,
                service_name,
            },
            StackDrift::ContainerPaused {
                container_id,
                service_name,
            } => Self::ContainerPaused {
                container_id,
                service_name,
            },
            StackDrift::ContainerUnhealthy {
                container_id,
                service_name,
                health_status,
            } => Self::ContainerUnhealthy {
                container_id,
                service_name,
                health_status,
            },
            StackDrift::ImageMismatch {
                service_name,
                expected_image,
                actual_image,
            } => Self::ImageMismatch {
                service_name,
                expected_image,
                actual_image,
            },
            StackDrift::ConfigHashMismatch {
                service_name,
                expected_hash,
                actual_hash,
            } => Self::ConfigHashMismatch {
                service_name,
                expected_hash,
                actual_hash,
            },
        }
    }
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

impl From<citadel_stacks::StackReconciliationStatus> for StackReconciliationStatus {
    fn from(value: citadel_stacks::StackReconciliationStatus) -> Self {
        match value {
            citadel_stacks::StackReconciliationStatus::NoDrift => Self::NoDrift,
            citadel_stacks::StackReconciliationStatus::Reconciled => Self::Reconciled,
            citadel_stacks::StackReconciliationStatus::Partial => Self::Partial,
            citadel_stacks::StackReconciliationStatus::RequiresReapply => Self::RequiresReapply,
            citadel_stacks::StackReconciliationStatus::Disabled => Self::Disabled,
            citadel_stacks::StackReconciliationStatus::Failed => Self::Failed,
        }
    }
}

impl From<StackReconciliationStatus> for citadel_stacks::StackReconciliationStatus {
    fn from(value: StackReconciliationStatus) -> Self {
        match value {
            StackReconciliationStatus::NoDrift => Self::NoDrift,
            StackReconciliationStatus::Reconciled => Self::Reconciled,
            StackReconciliationStatus::Partial => Self::Partial,
            StackReconciliationStatus::RequiresReapply => Self::RequiresReapply,
            StackReconciliationStatus::Disabled => Self::Disabled,
            StackReconciliationStatus::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum StackReconciliationActionType {
    StartContainer,
    ResumeContainer,
    RemoveContainer,
}

impl From<citadel_stacks::StackReconciliationActionType> for StackReconciliationActionType {
    fn from(value: citadel_stacks::StackReconciliationActionType) -> Self {
        match value {
            citadel_stacks::StackReconciliationActionType::StartContainer => Self::StartContainer,
            citadel_stacks::StackReconciliationActionType::ResumeContainer => Self::ResumeContainer,
            citadel_stacks::StackReconciliationActionType::RemoveContainer => Self::RemoveContainer,
        }
    }
}

impl From<StackReconciliationActionType> for citadel_stacks::StackReconciliationActionType {
    fn from(value: StackReconciliationActionType) -> Self {
        match value {
            StackReconciliationActionType::StartContainer => Self::StartContainer,
            StackReconciliationActionType::ResumeContainer => Self::ResumeContainer,
            StackReconciliationActionType::RemoveContainer => Self::RemoveContainer,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackReconciliationAction {
    pub container_id: String,
    pub service_name: String,
    pub action: StackReconciliationActionType,
    pub succeeded: bool,
    #[schema(required = true)]
    pub error_message: Option<String>,
}

impl From<citadel_stacks::StackReconciliationAction> for StackReconciliationAction {
    fn from(value: citadel_stacks::StackReconciliationAction) -> Self {
        Self {
            container_id: value.container_id,
            service_name: value.service_name,
            action: value.action.into(),
            succeeded: value.succeeded,
            error_message: value.error_message,
        }
    }
}

impl From<StackReconciliationAction> for citadel_stacks::StackReconciliationAction {
    fn from(value: StackReconciliationAction) -> Self {
        Self {
            container_id: value.container_id,
            service_name: value.service_name,
            action: value.action.into(),
            succeeded: value.succeeded,
            error_message: value.error_message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackReconciliationResult {
    pub stack_id: Uuid,
    pub status: StackReconciliationStatus,
    pub before_report: StackDriftReport,
    #[schema(required = true)]
    pub after_report: Option<StackDriftReport>,
    pub actions: Vec<StackReconciliationAction>,
}

impl From<citadel_stacks::StackReconciliationResult> for StackReconciliationResult {
    fn from(value: citadel_stacks::StackReconciliationResult) -> Self {
        Self {
            stack_id: value.stack_id,
            status: value.status.into(),
            before_report: value.before_report.into(),
            after_report: value.after_report.map(|item| item.into()),
            actions: value.actions.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<StackReconciliationResult> for citadel_stacks::StackReconciliationResult {
    fn from(value: StackReconciliationResult) -> Self {
        Self {
            stack_id: value.stack_id,
            status: value.status.into(),
            before_report: value.before_report.into(),
            after_report: value.after_report.map(|item| item.into()),
            actions: value.actions.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum StackImportKind {
    ComposeProject,
    SwarmStack,
}

impl From<citadel_stacks::StackImportKind> for StackImportKind {
    fn from(value: citadel_stacks::StackImportKind) -> Self {
        match value {
            citadel_stacks::StackImportKind::ComposeProject => Self::ComposeProject,
            citadel_stacks::StackImportKind::SwarmStack => Self::SwarmStack,
        }
    }
}

impl From<StackImportKind> for citadel_stacks::StackImportKind {
    fn from(value: StackImportKind) -> Self {
        match value {
            StackImportKind::ComposeProject => Self::ComposeProject,
            StackImportKind::SwarmStack => Self::SwarmStack,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectRuntimeService {
    pub name: String,
    #[schema(required = true)]
    pub image: Option<String>,
    pub container_count: usize,
    pub states: Vec<String>,
}

impl From<citadel_stacks::ComposeProjectRuntimeService> for ComposeProjectRuntimeService {
    fn from(value: citadel_stacks::ComposeProjectRuntimeService) -> Self {
        Self {
            name: value.name,
            image: value.image,
            container_count: value.container_count,
            states: value.states,
        }
    }
}

impl From<ComposeProjectRuntimeService> for citadel_stacks::ComposeProjectRuntimeService {
    fn from(value: ComposeProjectRuntimeService) -> Self {
        Self {
            name: value.name,
            image: value.image,
            container_count: value.container_count,
            states: value.states,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackAdoptionIssue {
    pub code: String,
    pub message: String,
    pub severity: String,
    #[schema(required = true)]
    pub field_path: Option<String>,
}

impl From<citadel_stacks::StackAdoptionIssue> for StackAdoptionIssue {
    fn from(value: citadel_stacks::StackAdoptionIssue) -> Self {
        Self {
            code: value.code,
            message: value.message,
            severity: value.severity,
            field_path: value.field_path,
        }
    }
}

impl From<StackAdoptionIssue> for citadel_stacks::StackAdoptionIssue {
    fn from(value: StackAdoptionIssue) -> Self {
        Self {
            code: value.code,
            message: value.message,
            severity: value.severity,
            field_path: value.field_path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectServiceComparison {
    pub name: String,
    pub runtime_container_count: usize,
    #[schema(required = true)]
    pub runtime_image: Option<String>,
    pub defined_in_source: bool,
    #[schema(required = true)]
    pub source_image: Option<String>,
}

impl From<citadel_stacks::ComposeProjectServiceComparison> for ComposeProjectServiceComparison {
    fn from(value: citadel_stacks::ComposeProjectServiceComparison) -> Self {
        Self {
            name: value.name,
            runtime_container_count: value.runtime_container_count,
            runtime_image: value.runtime_image,
            defined_in_source: value.defined_in_source,
            source_image: value.source_image,
        }
    }
}

impl From<ComposeProjectServiceComparison> for citadel_stacks::ComposeProjectServiceComparison {
    fn from(value: ComposeProjectServiceComparison) -> Self {
        Self {
            name: value.name,
            runtime_container_count: value.runtime_container_count,
            runtime_image: value.runtime_image,
            defined_in_source: value.defined_in_source,
            source_image: value.source_image,
        }
    }
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

impl From<citadel_stacks::ComposeProjectImportValidation> for ComposeProjectImportValidation {
    fn from(value: citadel_stacks::ComposeProjectImportValidation) -> Self {
        Self {
            services: value.services.into_iter().map(|item| item.into()).collect(),
            issues: value.issues.into_iter().map(|item| item.into()).collect(),
            preview_fingerprint: value.preview_fingerprint,
            importable_sensitive_environment_names: value.importable_sensitive_environment_names,
            can_import_sensitive_environment_values: value.can_import_sensitive_environment_values,
        }
    }
}

impl From<ComposeProjectImportValidation> for citadel_stacks::ComposeProjectImportValidation {
    fn from(value: ComposeProjectImportValidation) -> Self {
        Self {
            services: value.services.into_iter().map(|item| item.into()).collect(),
            issues: value.issues.into_iter().map(|item| item.into()).collect(),
            preview_fingerprint: value.preview_fingerprint,
            importable_sensitive_environment_names: value.importable_sensitive_environment_names,
            can_import_sensitive_environment_values: value.can_import_sensitive_environment_values,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmStackCompatibilityIssue {
    pub severity: SwarmStackCompatibilitySeverity,
    pub code: String,
    pub message: String,
    pub field_path: Option<String>,
}

impl From<citadel_stacks::SwarmStackCompatibilityIssue> for SwarmStackCompatibilityIssue {
    fn from(value: citadel_stacks::SwarmStackCompatibilityIssue) -> Self {
        Self {
            severity: value.severity.into(),
            code: value.code,
            message: value.message,
            field_path: value.field_path,
        }
    }
}

impl From<SwarmStackCompatibilityIssue> for citadel_stacks::SwarmStackCompatibilityIssue {
    fn from(value: SwarmStackCompatibilityIssue) -> Self {
        Self {
            severity: value.severity.into(),
            code: value.code,
            message: value.message,
            field_path: value.field_path,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum SwarmStackCompatibilitySeverity {
    Warning,
    Error,
}

impl From<citadel_stacks::SwarmStackCompatibilitySeverity> for SwarmStackCompatibilitySeverity {
    fn from(value: citadel_stacks::SwarmStackCompatibilitySeverity) -> Self {
        match value {
            citadel_stacks::SwarmStackCompatibilitySeverity::Warning => Self::Warning,
            citadel_stacks::SwarmStackCompatibilitySeverity::Error => Self::Error,
        }
    }
}

impl From<SwarmStackCompatibilitySeverity> for citadel_stacks::SwarmStackCompatibilitySeverity {
    fn from(value: SwarmStackCompatibilitySeverity) -> Self {
        match value {
            SwarmStackCompatibilitySeverity::Warning => Self::Warning,
            SwarmStackCompatibilitySeverity::Error => Self::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmStackCompatibilityReport {
    pub is_compatible: bool,
    pub issues: Vec<SwarmStackCompatibilityIssue>,
}

impl From<citadel_stacks::SwarmStackCompatibilityReport> for SwarmStackCompatibilityReport {
    fn from(value: citadel_stacks::SwarmStackCompatibilityReport) -> Self {
        Self {
            is_compatible: value.is_compatible,
            issues: value.issues.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<SwarmStackCompatibilityReport> for citadel_stacks::SwarmStackCompatibilityReport {
    fn from(value: SwarmStackCompatibilityReport) -> Self {
        Self {
            is_compatible: value.is_compatible,
            issues: value.issues.into_iter().map(|item| item.into()).collect(),
        }
    }
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

fn default_command_path() -> String {
    "./".to_owned()
}

const fn default_webhook_provider() -> WebhookProvider {
    WebhookProvider::GitHub
}

const fn default_webhook_auth() -> WebhookAuthScheme {
    WebhookAuthScheme::GitHubHmacSha256
}

const fn disabled_update() -> StackUpdateBehavior {
    StackUpdateBehavior::Disabled
}

const fn default_destroy_before_deploy() -> bool {
    true
}
