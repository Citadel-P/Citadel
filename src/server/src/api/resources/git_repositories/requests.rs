use crate::api::resources::git_repositories::spec::*;
use citadel_primitives::PatchField;
use citadel_primitives::{WebhookConfig, WebhookPatch};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewGitRepository {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub default_branch: String,
    pub git_account_id: Option<Uuid>,
    #[serde(default)]
    pub sync_mode: GitRepositorySyncMode,
    #[serde(default = "default_sync_interval")]
    pub sync_interval_minutes: Option<i32>,
    #[schema(value_type = Option<crate::api::resources::schema_models::primitives::WebhookConfigSchema>)]
    pub webhook: Option<WebhookConfig>,
    pub on_clone: Option<RepoCommand>,
    pub on_pull: Option<RepoCommand>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl From<NewGitRepository> for citadel_git::CreateGitRepository {
    fn from(value: NewGitRepository) -> Self {
        Self {
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
            tag_ids: value.tag_ids,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryPatch {
    pub name: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub description: PatchField<String>,
    pub url: Option<String>,
    pub default_branch: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<Uuid>, required = false)]
    pub git_account_id: PatchField<Uuid>,
    pub sync_mode: Option<GitRepositorySyncMode>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    pub sync_interval_minutes: PatchField<i32>,
    #[serde(default)]
    #[schema(value_type = Option<crate::api::resources::schema_models::primitives::WebhookPatchSchema>, required = false)]
    pub webhook: PatchField<WebhookPatch>,
    #[serde(default)]
    #[schema(value_type = Option<RepoCommand>, required = false)]
    pub on_clone: PatchField<RepoCommand>,
    #[serde(default)]
    #[schema(value_type = Option<RepoCommand>, required = false)]
    pub on_pull: PatchField<RepoCommand>,
    pub tag_ids: Option<Vec<Uuid>>,
}

impl From<GitRepositoryPatch> for citadel_git::GitRepositoryPatch {
    fn from(value: GitRepositoryPatch) -> Self {
        Self {
            name: value.name,
            description: value.description,
            url: value.url,
            default_branch: value.default_branch,
            git_account_id: value.git_account_id,
            sync_mode: value.sync_mode.map(|v| v.into()),
            sync_interval_minutes: value.sync_interval_minutes,
            webhook: value.webhook,
            on_clone: value.on_clone.map(Into::into),
            on_pull: value.on_pull.map(Into::into),
            tag_ids: value.tag_ids,
        }
    }
}

const fn default_sync_interval() -> Option<i32> {
    Some(5)
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FilesQuery {
    pub(crate) commit_sha: Option<String>,
    pub(crate) path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompareQuery {
    pub(crate) base_commit_sha: String,
    pub(crate) head_commit_sha: String,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct BranchQuery {
    pub(crate) branch: Option<String>,
}
