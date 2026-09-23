use super::*;

#[derive(Debug, Clone)]
pub struct GitRepositorySource {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub default_branch: String,
    pub git_account_id: Option<Uuid>,
    pub sync_mode: GitRepositorySyncMode,
    pub sync_interval_minutes: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct GitSyncClaim {
    pub trigger: String,
    pub previous_commit: Option<String>,
    pub previous_error: Option<String>,
    pub repository: GitRepositorySource,
    pub reference_id: Uuid,
    pub branch: String,
    pub actor_id: ActorId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitWebhookOutcome {
    Queued { branch: String },
    Ignored,
}

#[derive(Debug, Clone)]
pub struct GitRepository {
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
    pub latest_activity: Option<Value>,
    pub tags: Vec<GitRepositoryTag>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct GitRepositoryTag {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
