use super::spec::*;
use crate::api::metadata_patch::MetadataPatch;
use serde::Deserialize;
use serde_json::Value;
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
    pub webhook: Option<Value>,
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
    pub description: MetadataPatch<String>,
    pub url: Option<String>,
    pub default_branch: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<Uuid>, required = false)]
    pub git_account_id: MetadataPatch<Uuid>,
    pub sync_mode: Option<GitRepositorySyncMode>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    pub sync_interval_minutes: MetadataPatch<i32>,
    #[serde(default)]
    #[schema(value_type = Option<Value>, required = false)]
    pub webhook: MetadataPatch<Value>,
    #[serde(default)]
    #[schema(value_type = Option<RepoCommand>, required = false)]
    pub on_clone: MetadataPatch<RepoCommand>,
    #[serde(default)]
    #[schema(value_type = Option<RepoCommand>, required = false)]
    pub on_pull: MetadataPatch<RepoCommand>,
    pub tag_ids: Option<Vec<Uuid>>,
}
impl From<GitRepositoryPatch> for citadel_git::GitRepositoryPatch {
    fn from(value: GitRepositoryPatch) -> Self {
        Self {
            name: value.name,
            description: match value.description {
                MetadataPatch::Missing => citadel_git::repositories::FieldPatch::Missing,
                MetadataPatch::Null => citadel_git::repositories::FieldPatch::Null,
                MetadataPatch::Value(v) => citadel_git::repositories::FieldPatch::Value(v),
            },
            url: value.url,
            default_branch: value.default_branch,
            git_account_id: match value.git_account_id {
                MetadataPatch::Missing => citadel_git::repositories::FieldPatch::Missing,
                MetadataPatch::Null => citadel_git::repositories::FieldPatch::Null,
                MetadataPatch::Value(v) => citadel_git::repositories::FieldPatch::Value(v),
            },
            sync_mode: value.sync_mode.map(|v| v.into()),
            sync_interval_minutes: match value.sync_interval_minutes {
                MetadataPatch::Missing => citadel_git::repositories::FieldPatch::Missing,
                MetadataPatch::Null => citadel_git::repositories::FieldPatch::Null,
                MetadataPatch::Value(v) => citadel_git::repositories::FieldPatch::Value(v),
            },
            webhook: match value.webhook {
                MetadataPatch::Missing => citadel_git::repositories::FieldPatch::Missing,
                MetadataPatch::Null => citadel_git::repositories::FieldPatch::Null,
                MetadataPatch::Value(v) => citadel_git::repositories::FieldPatch::Value(v),
            },
            on_clone: match value.on_clone {
                MetadataPatch::Missing => citadel_git::repositories::FieldPatch::Missing,
                MetadataPatch::Null => citadel_git::repositories::FieldPatch::Null,
                MetadataPatch::Value(v) => citadel_git::repositories::FieldPatch::Value(v.into()),
            },
            on_pull: match value.on_pull {
                MetadataPatch::Missing => citadel_git::repositories::FieldPatch::Missing,
                MetadataPatch::Null => citadel_git::repositories::FieldPatch::Null,
                MetadataPatch::Value(v) => citadel_git::repositories::FieldPatch::Value(v.into()),
            },
            tag_ids: value.tag_ids,
        }
    }
}
const fn default_sync_interval() -> Option<i32> {
    Some(5)
}
