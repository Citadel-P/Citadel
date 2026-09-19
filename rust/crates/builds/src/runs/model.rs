use crate::*;

#[derive(Debug, Clone)]
pub struct BuildRun {
    pub id: Uuid,
    pub build_project_id: Uuid,
    pub project_name_snapshot: String,
    pub git_repository_id: Uuid,
    pub branch: String,
    pub resolved_commit_sha: Option<String>,
    pub context_path: String,
    pub dockerfile_path: String,
    pub target: Option<String>,
    pub registry_id: Uuid,
    pub registry_host: String,
    pub image_repository: String,
    pub image_references: Vec<String>,
    pub trigger: String,
    pub status: String,
    pub image_digest: Option<String>,
    pub timeout_seconds: i32,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub triggered_by_actor_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct BuildClaim {
    pub project: BuildProject,
    pub run: BuildRun,
}

#[derive(Debug, Clone)]
pub struct BuildExecutionResult {
    pub status: &'static str,
    pub exit_code: Option<i32>,
    pub image_digest: Option<String>,
    pub resolved_commit_sha: Option<String>,
    pub image_references: Vec<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub logs: Vec<BuildLog>,
}

#[derive(Debug, Clone)]
pub struct BuildLog {
    pub stream: String,
    pub message: String,
}
