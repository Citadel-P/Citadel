use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitRepositoryActivitySnapshot {
    #[serde(rename = "Id")]
    pub id: Uuid,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "Url")]
    pub url: String,
    #[serde(rename = "DefaultBranch")]
    pub default_branch: String,
    #[serde(rename = "GitAccountId")]
    pub git_account_id: Option<Uuid>,
    #[serde(rename = "SyncMode")]
    pub sync_mode: String,
    #[serde(rename = "SyncIntervalMinutes")]
    pub sync_interval_minutes: Option<i32>,
    #[serde(rename = "Webhook")]
    pub webhook: Option<citadel_primitives::WebhookConfig>,
    #[serde(rename = "OnClone")]
    pub on_clone: Option<Value>,
    #[serde(rename = "OnPull")]
    pub on_pull: Option<Value>,
    #[serde(rename = "ResolvedCommitSha")]
    pub resolved_commit_sha: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitRepositorySyncActivitySnapshot {
    #[serde(rename = "CommitSha")]
    pub commit_sha: Option<String>,
    #[serde(rename = "Message")]
    pub message: Option<String>,
}
