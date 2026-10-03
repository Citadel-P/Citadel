//! Server-owned OpenAPI descriptions of stacks values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

use chrono::{DateTime, Utc};
use uuid::Uuid;

enum_schema!(
    StackReconciliationStatusSchema,
    "StackReconciliationStatus",
    citadel_stacks::StackReconciliationStatus,
    [
        NoDrift,
        Reconciled,
        Partial,
        RequiresReapply,
        Disabled,
        Failed
    ]
);

enum_schema!(
    StackReconciliationActionTypeSchema,
    "StackReconciliationActionType",
    citadel_stacks::StackReconciliationActionType,
    [StartContainer, ResumeContainer, RemoveContainer]
);

enum_schema!(
    StackSourceSchema,
    "StackSource",
    citadel_stacks::StackSource,
    [WebEditor, Git]
);

enum_schema!(
    StackUpdateBehaviorSchema,
    "StackUpdateBehavior",
    citadel_stacks::StackUpdateBehavior,
    [Disabled, Notify, ServiceAutoDeploy, StackAutoDeploy]
);

enum_schema!(
    StackReleaseStatusSchema,
    "StackReleaseStatus",
    citadel_stacks::StackReleaseStatus,
    [
        Unknown, Created, Applying, Healthy, Pending, Paused, Degraded, Failed, Stopped, TimedOut
    ]
);

enum_schema!(
    StackDriftModeSchema,
    "StackDriftMode",
    citadel_stacks::StackDriftMode,
    [Disabled, DetectOnly, AutoFix]
);

schema_model! {
    citadel_stacks::StackDriftPolicy =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = StackDriftPolicy)]
    pub struct StackDriftPolicySchema {
        #[schema(value_type = crate::api::resources::schema_models::stacks::StackDriftModeSchema)]
        pub mode: citadel_stacks::StackDriftMode,
        pub alert_on_drift: bool,
        pub mark_degraded: bool,
        pub auto_start_stopped_containers: bool,
        pub auto_resume_paused_containers: bool,
        pub remove_extra_containers: bool,
    }
}

schema_model! {
    citadel_stacks::StackCommand =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    #[schema(as = StackCommand)]
    pub struct StackCommandSchema {
        #[serde(default)]
        pub commands: Vec<String>,
        #[serde(default = "default_command_path")]
        pub path: String,
    }
}

schema_model! {
    citadel_stacks::StackBuildImageBinding =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    #[schema(as = StackBuildImageBinding)]
    pub struct StackBuildImageBindingSchema {
        pub service_name: String,
        pub build_project_id: Uuid,
        #[serde(default)]
        pub redeploy_on_build: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub resolved_image_reference: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub resolved_digest: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub resolved_build_run_id: Option<Uuid>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub applied_image_reference: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub applied_digest: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub applied_build_run_id: Option<Uuid>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub applied_at: Option<DateTime<Utc>>,
    }
}

schema_model! {
    citadel_stacks::StackWebhookConfig =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    #[schema(as = StackWebhookConfig)]
    pub struct StackWebhookConfigSchema {
        #[serde(flatten)]
        #[schema(value_type = crate::api::resources::schema_models::primitives::WebhookConfigSchema)]
        pub config: citadel_primitives::WebhookConfig,
        #[serde(default)]
        pub force_deploy: bool,
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
#[schema(as = StackSpec)]
// Schema-only mirror; preserve the native shape without adding boxed runtime fields.
#[allow(clippy::large_enum_variant)]
pub enum StackSpecSchema {
    WebEditor {
        #[serde(rename = "composeFile")]
        compose_file: String,
        #[serde(rename = "updateBehavior", default = "disabled_update")]
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
        #[serde(rename = "updateBehavior", default = "disabled_update")]
        #[schema(value_type = crate::api::resources::schema_models::stacks::StackUpdateBehaviorSchema)]
        update_behavior: citadel_stacks::StackUpdateBehavior,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schema(value_type = Option < crate::api::resources::schema_models::stacks::StackWebhookConfigSchema >)]
        webhook: Option<citadel_stacks::StackWebhookConfig>,
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

impl From<citadel_stacks::StackSpec> for StackSpecSchema {
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
                webhook,
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

schema_model! {
    citadel_stacks::StackSpecCommon =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    #[schema(as = StackSpecCommon)]
    pub struct StackSpecCommonSchema {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub project_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schema(value_type = Option < crate::api::resources::schema_models::stacks::StackCommandSchema >)]
        pub pre_deploy: Option<citadel_stacks::StackCommand>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schema(value_type = Option < crate::api::resources::schema_models::stacks::StackCommandSchema >)]
        pub post_deploy: Option<citadel_stacks::StackCommand>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub env_file_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub registry_id: Option<Uuid>,
        #[serde(default = "default_destroy_before_deploy")]
        pub destroy_before_deploy: bool,
        #[serde(default)]
        #[schema(value_type = Vec < crate::api::resources::schema_models::stacks::StackBuildImageBindingSchema >)]
        pub build_image_bindings: Vec<citadel_stacks::StackBuildImageBinding>,
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
#[schema(as = StackUpdateState)]
pub enum StackUpdateStateSchema {
    WebEditor {
        #[serde(rename = "recreateStackOnNewImageState")]
        #[schema(value_type = crate::api::resources::schema_models::stacks::RecreateStackOnNewImageStateSchema)]
        recreate_stack_on_new_image_state: citadel_stacks::RecreateStackOnNewImageState,
    },

    Git {
        #[serde(rename = "recreateStackOnNewImageState")]
        #[schema(value_type = crate::api::resources::schema_models::stacks::RecreateStackOnNewImageStateSchema)]
        recreate_stack_on_new_image_state: citadel_stacks::RecreateStackOnNewImageState,
        #[serde(rename = "recreateStackOnNewCommitState")]
        #[schema(value_type = crate::api::resources::schema_models::stacks::RecreateStackOnNewCommitStateSchema)]
        recreate_stack_on_new_commit_state: citadel_stacks::RecreateStackOnNewCommitState,
    },
}

impl From<citadel_stacks::StackUpdateState> for StackUpdateStateSchema {
    fn from(value: citadel_stacks::StackUpdateState) -> Self {
        match value {
            citadel_stacks::StackUpdateState::WebEditor {
                recreate_stack_on_new_image_state,
            } => Self::WebEditor {
                recreate_stack_on_new_image_state,
            },
            citadel_stacks::StackUpdateState::Git {
                recreate_stack_on_new_image_state,
                recreate_stack_on_new_commit_state,
            } => Self::Git {
                recreate_stack_on_new_image_state,
                recreate_stack_on_new_commit_state,
            },
        }
    }
}

schema_model! {
    citadel_stacks::RecreateStackOnNewImageState =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = RecreateStackOnNewImageState)]
    pub struct RecreateStackOnNewImageStateSchema {
        #[serde(default)]
        #[schema(value_type = Vec < crate::api::resources::schema_models::stacks::ImageUpdateStateSchema >)]
        pub auto_update_states: Vec<citadel_stacks::ImageUpdateState>,
    }
}

schema_model! {
    citadel_stacks::RecreateStackOnNewCommitState =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = RecreateStackOnNewCommitState)]
    pub struct RecreateStackOnNewCommitStateSchema {
        pub current_commit_sha: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub remote_commit_sha: Option<String>,
        pub last_checked_at: DateTime<Utc>,
    }
}

schema_model! {
    citadel_stacks::ImageUpdateState =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ImageUpdateState)]
    pub struct ImageUpdateStateSchema {
        pub service_name: String,
        pub image_name: String,
        pub current_digest: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub remote_digest: Option<String>,
        pub last_checked_at: DateTime<Utc>,
        pub update_available: bool,
    }
}

schema_model! {
    citadel_stacks::ResourceBindingSnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ResourceBindingSnapshot)]
    pub struct ResourceBindingSnapshotSchema {
        pub name: String,
        pub kind: String,
        pub scope: String,
        pub value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub secret_id: Option<Uuid>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub secret_delivery_mode: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub target_path: Option<String>,
    }
}

schema_model! {
    citadel_stacks::StackReleaseSource =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = StackReleaseSource)]
    pub struct StackReleaseSourceSchema {
        #[schema(value_type = crate::api::resources::schema_models::stacks::StackSourceSchema)]
        pub source_type: citadel_stacks::StackSource,
        pub git_repository_id: Option<Uuid>,
        pub git_repository_name: Option<String>,
        pub branch: Option<String>,
        pub requested_commit_sha: Option<String>,
        pub resolved_commit_sha: String,
        #[serde(default)]
        pub compose_paths: Vec<String>,
        #[serde(default)]
        pub env_file_paths: Vec<String>,
        pub git_repository_url: Option<String>,
        pub working_directory: Option<String>,
        pub watch_paths: Option<Vec<String>>,
        pub compose_env_files_from_repo: Option<Vec<String>>,
        pub compose_digest: Option<String>,
    }
}

schema_model! {
    citadel_stacks::ComposeProjectRuntimeService =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ComposeProjectRuntimeService)]
    pub struct ComposeProjectRuntimeServiceSchema {
        pub name: String,
        #[schema(required = true)]
        pub image: Option<String>,
        pub container_count: usize,
        pub states: Vec<String>,
    }
}

schema_model! {
    citadel_stacks::StackAdoptionIssue =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = StackAdoptionIssue)]
    pub struct StackAdoptionIssueSchema {
        pub code: String,
        pub message: String,
        pub severity: String,
        #[schema(required = true)]
        pub field_path: Option<String>,
    }
}

schema_model! {
    citadel_stacks::ComposeProjectServiceComparison =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ComposeProjectServiceComparison)]
    pub struct ComposeProjectServiceComparisonSchema {
        pub name: String,
        pub runtime_container_count: usize,
        #[schema(required = true)]
        pub runtime_image: Option<String>,
        pub defined_in_source: bool,
        #[schema(required = true)]
        pub source_image: Option<String>,
    }
}

schema_model! {
    citadel_stacks::ComposeProjectImportValidation =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ComposeProjectImportValidation)]
    pub struct ComposeProjectImportValidationSchema {
        #[schema(value_type = Vec < crate::api::resources::schema_models::stacks::ComposeProjectServiceComparisonSchema >)]
        pub services: Vec<citadel_stacks::ComposeProjectServiceComparison>,
        #[schema(value_type = Vec < crate::api::resources::schema_models::stacks::StackAdoptionIssueSchema >)]
        pub issues: Vec<citadel_stacks::StackAdoptionIssue>,
        pub preview_fingerprint: String,
        pub importable_sensitive_environment_names: Vec<String>,
        pub can_import_sensitive_environment_values: bool,
    }
}
