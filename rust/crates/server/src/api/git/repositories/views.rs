use chrono::{DateTime, Utc};
use serde::Serialize;
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
    pub resolved_commit_sha: Option<String>,
    pub status: String,
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

use super::spec::{GitRepositorySyncMode, RepoCommand};
use citadel_resources::TagSummary;
use serde_json::Value;
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
            latest_activity_view: citadel_application::public_latest_activity(
                value.latest_activity,
            ),
            tags: value
                .tags
                .into_iter()
                .map(|v| TagSummary {
                    id: v.id,
                    name: v.name,
                    color: v.color,
                })
                .collect(),
        }
    }
}
