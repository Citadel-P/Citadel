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
    pub on_clone: Option<RepoCommand>,
    pub on_pull: Option<RepoCommand>,
}

#[derive(Debug, Clone)]
pub struct GitSyncClaim {
    /// Persisted claim timestamp fences completion against later attempts.
    pub started_at: DateTime<Utc>,
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
    Ignored { reason: &'static str },
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
    pub webhook: Option<citadel_primitives::WebhookConfig>,
    pub on_clone: Option<RepoCommand>,
    pub on_pull: Option<RepoCommand>,
    pub status: crate::GitRepositoryStatus,
    pub control_state: citadel_primitives::ResourceControlState,
    pub audit: citadel_primitives::AuditMetadata,

    // Related data populated by full resource reads.
    pub tags: Vec<citadel_tags::TagSummary>,
    pub latest_activity: Option<citadel_activities::ActivitySummary>,
    pub sync_status: Option<GitRepositorySyncStatus>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Latest persisted check of the configured default branch.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositorySyncStatus {
    pub status: crate::GitRepositoryRefStatus,
    pub last_synced_at: DateTime<Utc>,
}

impl GitRepositorySource {
    pub fn has_hooks(&self) -> bool {
        [&self.on_clone, &self.on_pull]
            .into_iter()
            .flatten()
            .any(|hook| !hook.commands.is_empty())
    }
}

impl RepoCommand {
    pub fn validate(&self) -> Result<(), GitRepositoryError> {
        use std::path::Component;
        if self.commands.len() > 100
            || self.commands.iter().any(|command| {
                command.trim().is_empty() || command.len() > 4096 || command.contains('\0')
            })
        {
            return Err(GitRepositoryError::Validation(
                "Git repository commands exceed the supported limits.".into(),
            ));
        }
        if self.path.is_empty()
            || self.path.len() > 1024
            || self.path.contains(['\0', '\\', ':'])
            || std::path::Path::new(&self.path)
                .components()
                .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
        {
            return Err(GitRepositoryError::Validation(
                "Hook working directory must be a relative path inside the repository.".into(),
            ));
        }
        Ok(())
    }
}
