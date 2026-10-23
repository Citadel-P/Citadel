use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
// Public specification preserves the native shape while excluding webhook credentials.
#[allow(clippy::large_enum_variant)]
pub enum StackSpecView {
    WebEditor {
        #[serde(rename = "composeFile")]
        compose_file: String,
        #[serde(rename = "updateBehavior")]
        #[schema(value_type = crate::api::resources::schema_models::stacks::StackUpdateBehaviorSchema)]
        update_behavior: citadel_stacks::StackUpdateBehavior,
        #[serde(flatten)]
        #[schema(value_type = crate::api::resources::schema_models::stacks::StackSpecCommonSchema)]
        common: citadel_stacks::StackSpecCommon,
    },

    Git {
        #[serde(rename = "gitRepoId")]
        git_repo_id: Uuid,

        branch: String,
        #[serde(rename = "commitSha", default, skip_serializing_if = "Option::is_none")]
        commit_sha: Option<String>,
        #[serde(rename = "updateBehavior")]
        #[schema(value_type = crate::api::resources::schema_models::stacks::StackUpdateBehaviorSchema)]
        update_behavior: citadel_stacks::StackUpdateBehavior,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schema(value_type = Option<StackWebhookView>)]
        webhook: Option<StackWebhookView>,
        #[serde(rename = "composePaths", default)]
        compose_paths: Vec<String>,
        #[serde(
            rename = "workingDirectory",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        working_directory: Option<String>,
        #[serde(rename = "composeEnvFilesFromRepo", default)]
        compose_env_files_from_repo: Vec<String>,
        #[serde(rename = "watchPaths", default)]
        watch_paths: Vec<String>,
        #[serde(rename = "additionalEnvFileFromRepo", default)]
        additional_env_file_from_repo: Vec<String>,
        #[serde(flatten)]
        #[schema(value_type = crate::api::resources::schema_models::stacks::StackSpecCommonSchema)]
        common: citadel_stacks::StackSpecCommon,
    },
}

impl From<citadel_stacks::StackSpec> for StackSpecView {
    fn from(value: citadel_stacks::StackSpec) -> Self {
        match value {
            citadel_stacks::StackSpec::WebEditor {
                compose_file,
                update_behavior,
                common,
            } => Self::WebEditor {
                compose_file,
                update_behavior,
                common,
            },
            citadel_stacks::StackSpec::Git {
                git_repo_id,
                branch,
                commit_sha,
                update_behavior,
                webhook,
                compose_paths,
                working_directory,
                compose_env_files_from_repo,
                watch_paths,
                additional_env_file_from_repo,
                common,
            } => Self::Git {
                git_repo_id,
                branch,
                commit_sha,
                update_behavior,
                webhook: webhook.map(Into::into),
                compose_paths,
                working_directory,
                compose_env_files_from_repo,
                watch_paths,
                additional_env_file_from_repo,
                common,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackWebhookView {
    #[serde(flatten)]
    pub config: crate::api::resources::webhooks::views::WebhookView,
    pub force_deploy: bool,
}
impl From<citadel_stacks::StackWebhookConfig> for StackWebhookView {
    fn from(value: citadel_stacks::StackWebhookConfig) -> Self {
        Self {
            config: value.config.into(),
            force_deploy: value.force_deploy,
        }
    }
}
pub fn serialize<S: serde::Serializer>(
    value: &citadel_stacks::StackSpec,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(&StackSpecView::from(value.clone()), serializer)
}
