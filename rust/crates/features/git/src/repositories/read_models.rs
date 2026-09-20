use super::*;

#[derive(Debug, Clone)]
pub struct GitDirectoryListing {
    pub repository_id: Uuid,
    pub commit_sha: String,
    pub path: String,
    pub entries: Vec<GitDirectoryEntry>,
    pub is_truncated: bool,
    pub provider_repository_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitDirectoryEntry {
    pub name: String,
    pub path: String,
    pub entry_type: GitBrowserEntryType,
    pub size: Option<u64>,
    pub mode: String,
    pub target_commit_sha: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum GitBrowserEntryType {
    Directory,
    File,
    Symlink,
    Submodule,
}

#[derive(Debug, Clone)]
pub struct GitFileContent {
    pub repository_id: Uuid,
    pub commit_sha: String,
    pub path: String,
    pub entry_type: GitBrowserEntryType,
    pub size: u64,
    pub is_binary: bool,
    pub is_truncated: bool,
    pub content: Option<String>,
    pub preview_unavailable_reason: Option<String>,
    pub provider_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitCommitComparison {
    pub repository_id: Uuid,
    pub base_commit_sha: String,
    pub head_commit_sha: String,
    pub files: Vec<GitChangedPath>,
    pub is_truncated: bool,
}

#[derive(Debug, Clone)]
pub struct GitComposeDiscovery {
    pub repository_id: Uuid,
    pub branch: String,
    pub resolved_commit_sha: String,
    pub projects: Vec<GitComposeProjectCandidate>,
}

#[derive(Debug, Clone)]
pub struct GitComposeProjectCandidate {
    pub working_directory: String,
    pub compose_paths: Vec<String>,
    pub env_file_paths: Vec<String>,
    pub suggested_watch_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSnapshotFile {
    pub relative_path: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSnapshot {
    pub resolved_commit_sha: String,
    pub files: Vec<GitSnapshotFile>,
}

#[derive(Debug, Clone)]
pub struct GitRepositoryRef {
    pub id: Uuid,
    pub git_repository_id: Uuid,
    pub branch: String,
    pub resolved_commit_sha: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub last_synced_at: DateTime<Utc>,
}
