use crate::api::resources::{
    capabilities::ResourceCapabilitiesView,
    git_repositories::spec::{GitRepositorySyncMode, RepoCommand},
    tags::views::TagSummary,
};
use chrono::{DateTime, Utc};
use citadel_git::RemoteBranch;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDirectoryListing {
    pub repository_id: Uuid,
    pub commit_sha: String,
    pub path: String,
    pub entries: Vec<GitDirectoryEntry>,
    pub is_truncated: bool,
    pub provider_repository_url: Option<String>,
}

impl From<citadel_git::GitDirectoryListing> for GitDirectoryListing {
    fn from(value: citadel_git::GitDirectoryListing) -> Self {
        Self {
            repository_id: value.repository_id,
            commit_sha: value.commit_sha,
            path: value.path,
            entries: value.entries.into_iter().map(Into::into).collect(),
            is_truncated: value.is_truncated,
            provider_repository_url: value.provider_repository_url,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDirectoryEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: GitEntryTypeView,
    pub size: Option<u64>,
    pub mode: String,
    pub target_commit_sha: Option<String>,
}

impl From<citadel_git::GitDirectoryEntry> for GitDirectoryEntry {
    fn from(value: citadel_git::GitDirectoryEntry) -> Self {
        Self {
            name: value.name,
            path: value.path,
            entry_type: value.entry_type.into(),
            size: value.size,
            mode: value.mode,
            target_commit_sha: value.target_commit_sha,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum GitEntryTypeView {
    Directory,
    File,
    Symlink,
    Submodule,
}

impl From<citadel_git::GitBrowserEntryType> for GitEntryTypeView {
    fn from(value: citadel_git::GitBrowserEntryType) -> Self {
        match value {
            citadel_git::GitBrowserEntryType::Directory => Self::Directory,
            citadel_git::GitBrowserEntryType::File => Self::File,
            citadel_git::GitBrowserEntryType::Symlink => Self::Symlink,
            citadel_git::GitBrowserEntryType::Submodule => Self::Submodule,
        }
    }
}

impl From<GitEntryTypeView> for citadel_git::GitBrowserEntryType {
    fn from(value: GitEntryTypeView) -> Self {
        match value {
            GitEntryTypeView::Directory => Self::Directory,
            GitEntryTypeView::File => Self::File,
            GitEntryTypeView::Symlink => Self::Symlink,
            GitEntryTypeView::Submodule => Self::Submodule,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileContent {
    pub repository_id: Uuid,
    pub commit_sha: String,
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: GitEntryTypeView,
    pub size: u64,
    pub is_binary: bool,
    pub is_truncated: bool,
    pub content: Option<String>,
    pub preview_unavailable_reason: Option<String>,
    pub provider_url: Option<String>,
}

impl From<citadel_git::GitFileContent> for GitFileContent {
    fn from(value: citadel_git::GitFileContent) -> Self {
        Self {
            repository_id: value.repository_id,
            commit_sha: value.commit_sha,
            path: value.path,
            entry_type: value.entry_type.into(),
            size: value.size,
            is_binary: value.is_binary,
            is_truncated: value.is_truncated,
            content: value.content,
            preview_unavailable_reason: value.preview_unavailable_reason,
            provider_url: value.provider_url,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitComparison {
    pub repository_id: Uuid,
    pub base_commit_sha: String,
    pub head_commit_sha: String,
    pub files: Vec<GitChangedPath>,
    pub is_truncated: bool,
}

impl From<citadel_git::GitCommitComparison> for GitCommitComparison {
    fn from(value: citadel_git::GitCommitComparison) -> Self {
        Self {
            repository_id: value.repository_id,
            base_commit_sha: value.base_commit_sha,
            head_commit_sha: value.head_commit_sha,
            files: value.files.into_iter().map(Into::into).collect(),
            is_truncated: value.is_truncated,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitComposeDiscovery {
    pub repository_id: Uuid,
    pub branch: String,
    pub resolved_commit_sha: String,
    pub projects: Vec<GitComposeProjectCandidate>,
}

impl From<citadel_git::GitComposeDiscovery> for GitComposeDiscovery {
    fn from(value: citadel_git::GitComposeDiscovery) -> Self {
        Self {
            repository_id: value.repository_id,
            branch: value.branch,
            resolved_commit_sha: value.resolved_commit_sha,
            projects: value.projects.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitComposeProjectCandidate {
    pub working_directory: String,
    pub compose_paths: Vec<String>,
    pub env_file_paths: Vec<String>,
    pub suggested_watch_paths: Vec<String>,
}

impl From<citadel_git::GitComposeProjectCandidate> for GitComposeProjectCandidate {
    fn from(value: citadel_git::GitComposeProjectCandidate) -> Self {
        Self {
            working_directory: value.working_directory,
            compose_paths: value.compose_paths,
            env_file_paths: value.env_file_paths,
            suggested_watch_paths: value.suggested_watch_paths,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryRefView {
    pub id: Uuid,
    pub git_repository_id: Uuid,
    pub branch: String,
    #[schema(required = true)]
    pub resolved_commit_sha: Option<String>,
    #[schema(value_type = crate::openapi::compatibility::GitReposStatus)]
    pub status: String,
    #[schema(required = true)]
    pub last_error: Option<String>,
    pub last_synced_at: DateTime<Utc>,
}

impl From<citadel_git::GitRepositoryRef> for GitRepositoryRefView {
    fn from(value: citadel_git::GitRepositoryRef) -> Self {
        Self {
            id: value.id,
            git_repository_id: value.git_repository_id,
            branch: value.branch,
            resolved_commit_sha: value.resolved_commit_sha,
            status: value.status,
            last_error: value.last_error,
            last_synced_at: value.last_synced_at,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GitChangedPathStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
}

impl From<citadel_git::GitChangedPathStatus> for GitChangedPathStatus {
    fn from(value: citadel_git::GitChangedPathStatus) -> Self {
        match value {
            citadel_git::GitChangedPathStatus::Added => Self::Added,
            citadel_git::GitChangedPathStatus::Modified => Self::Modified,
            citadel_git::GitChangedPathStatus::Deleted => Self::Deleted,
            citadel_git::GitChangedPathStatus::Renamed => Self::Renamed,
            citadel_git::GitChangedPathStatus::Copied => Self::Copied,
            citadel_git::GitChangedPathStatus::TypeChanged => Self::TypeChanged,
        }
    }
}

impl From<GitChangedPathStatus> for citadel_git::GitChangedPathStatus {
    fn from(value: GitChangedPathStatus) -> Self {
        match value {
            GitChangedPathStatus::Added => Self::Added,
            GitChangedPathStatus::Modified => Self::Modified,
            GitChangedPathStatus::Deleted => Self::Deleted,
            GitChangedPathStatus::Renamed => Self::Renamed,
            GitChangedPathStatus::Copied => Self::Copied,
            GitChangedPathStatus::TypeChanged => Self::TypeChanged,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitChangedPath {
    pub status: GitChangedPathStatus,
    pub path: String,
    pub previous_path: Option<String>,
}

impl From<citadel_git::GitChangedPath> for GitChangedPath {
    fn from(value: citadel_git::GitChangedPath) -> Self {
        Self {
            status: value.status.into(),
            path: value.path,
            previous_path: value.previous_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryView {
    pub id: Uuid,
    pub name: String,
    #[schema(required = true)]
    pub description: Option<String>,
    pub url: String,
    pub default_branch: String,
    #[schema(required = true)]
    pub git_account_id: Option<Uuid>,
    pub sync_mode: GitRepositorySyncMode,
    #[schema(required = true)]
    pub sync_interval_minutes: Option<i32>,
    #[schema(value_type = Option<crate::openapi::compatibility::RepoWebhookConfig>, required = true)]
    pub webhook: Option<Value>,
    #[schema(required = true)]
    pub on_clone: Option<RepoCommand>,
    #[schema(required = true)]
    pub on_pull: Option<RepoCommand>,
    #[schema(value_type = crate::openapi::compatibility::GitReposStatus)]
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    #[schema(value_type = crate::api::resources::common::ResourceControlState)]
    pub control_state: String,
    #[schema(value_type = Option<crate::api::resources::activities::views::LatestActivityView>, required = true)]
    pub latest_activity_view: Option<Value>,
    pub tags: Vec<TagSummary>,
}

impl From<citadel_git::GitRepository> for GitRepositoryView {
    fn from(value: citadel_git::GitRepository) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            url: value.url,
            default_branch: value.default_branch,
            git_account_id: value.git_account_id,
            sync_mode: value.sync_mode.into(),
            sync_interval_minutes: value.sync_interval_minutes,
            webhook: value.webhook,
            on_clone: value.on_clone.map(|v| v.into()),
            on_pull: value.on_pull.map(|v| v.into()),
            status: value.status,
            created_at: value.created_at,
            created_by_actor_id: value.created_by_actor_id,
            control_state: value.control_state,
            latest_activity_view:
                crate::api::resources::activities::presentation::public_latest_activity(
                    value.latest_activity,
                ),
            tags: value.tags.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitRepositoryRefsResponse {
    pub(crate) refs: Vec<GitRepositoryRefView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitRepositoryBranchesResponse {
    pub(crate) branches: Vec<GitRepositoryBranchResponse>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitRepositoryBranchResponse {
    pub(crate) branch: String,
    pub(crate) commit_sha: String,
}

impl From<RemoteBranch> for GitRepositoryBranchResponse {
    fn from(value: RemoteBranch) -> Self {
        Self {
            branch: value.name,
            commit_sha: value.commit,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitRepositoriesResponse {
    pub(crate) git_repositories: Vec<AuthorizedGitRepositoryView>,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorizedGitRepositoryView {
    #[serde(flatten)]
    pub(crate) repository: GitRepositoryView,
    pub(crate) capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitRepositoryConfigResponse {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    #[schema(required = true)]
    pub(crate) description: Option<String>,
    pub(crate) url: String,
    pub(crate) default_branch: String,
    #[schema(required = true)]
    pub(crate) git_account_id: Option<Uuid>,
    pub(crate) sync_mode: GitRepositorySyncMode,
    #[schema(required = true)]
    pub(crate) sync_interval_minutes: Option<i32>,
    #[schema(value_type = Option<crate::openapi::compatibility::RepoWebhookConfig>, required = true)]
    pub(crate) webhook: Option<Value>,
    #[schema(required = true)]
    pub(crate) on_clone: Option<RepoCommand>,
    #[schema(required = true)]
    pub(crate) on_pull: Option<RepoCommand>,
    pub(crate) tags: Vec<crate::api::resources::tags::views::TagSummary>,
}
