pub use citadel_git::{GitRepositoryRefStatus, GitRepositoryStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum GitRepositorySyncMode {
    Manual,
    #[default]
    PullInterval,
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

impl From<GitRepositorySyncMode> for citadel_git::GitRepositorySyncMode {
    fn from(value: GitRepositorySyncMode) -> Self {
        match value {
            GitRepositorySyncMode::Manual => Self::Manual,
            GitRepositorySyncMode::PullInterval => Self::PullInterval,
        }
    }
}

impl From<citadel_git::GitRepositorySyncMode> for GitRepositorySyncMode {
    fn from(value: citadel_git::GitRepositorySyncMode) -> Self {
        match value {
            citadel_git::GitRepositorySyncMode::Manual => Self::Manual,
            citadel_git::GitRepositorySyncMode::PullInterval => Self::PullInterval,
        }
    }
}

impl From<RepoCommand> for citadel_git::RepoCommand {
    fn from(value: RepoCommand) -> Self {
        Self {
            commands: value.commands,
            path: value.path,
        }
    }
}

impl From<citadel_git::RepoCommand> for RepoCommand {
    fn from(value: citadel_git::RepoCommand) -> Self {
        Self {
            commands: value.commands,
            path: value.path,
        }
    }
}
