use serde::Serialize;

use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStackBuildImageBinding {
    pub service_name: String,
    pub build_project_id: Uuid,
    pub image_reference: String,
    pub digest: Option<String>,
    pub build_run_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackProgressItem {
    pub event_type: StackApplyEventType,
    pub message: Option<String>,
    pub exit_code: Option<i32>,
    pub stack_status: Option<StackReleaseStatus>,
    pub severity: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StackApplyEventType {
    Unknown,
    StdOut,
    StdErr,
    SystemMessage,
    CommandCompleted,
}

impl StackProgressItem {
    #[must_use]
    pub fn system(message: impl Into<String>) -> Self {
        Self {
            event_type: StackApplyEventType::SystemMessage,
            message: Some(message.into()),
            exit_code: None,
            stack_status: None,
            severity: None,
        }
    }

    #[must_use]
    pub fn completed(status: StackReleaseStatus, message: impl Into<String>) -> Self {
        Self {
            event_type: StackApplyEventType::CommandCompleted,
            message: Some(message.into()),
            exit_code: Some(if status == StackReleaseStatus::Healthy {
                0
            } else {
                1
            }),
            stack_status: Some(status),
            severity: Some(if status == StackReleaseStatus::Healthy {
                "success".to_owned()
            } else {
                "error".to_owned()
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackOperationClaim {
    pub stack_id: Uuid,
    pub release_id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    pub project_name: String,
    pub platform_type: String,
    pub spec: StackSpec,
    pub row_version: i64,
    pub actor_id: Uuid,
    pub operation: String,
    /// Empty means the whole Stack; persisted with the operation for recovery.
    pub service_names: Vec<String>,
}

/// An immutable, bounded Stack source snapshot prepared for one Apply attempt.
/// Paths are repository-relative and are validated again by each runtime
/// transport before they are written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackApplySource {
    pub files: Vec<StackSourceFile>,
    pub compose_paths: Vec<String>,
    pub env_file_paths: Vec<String>,
    pub working_directory: String,
    pub labels_override_path: Option<String>,
    pub resolved_commit_sha: Option<String>,
}

impl StackApplySource {
    pub fn compose_contents(&self) -> Result<Vec<String>, StackError> {
        self.compose_paths
            .iter()
            .map(|path| {
                let file = self
                    .files
                    .iter()
                    .find(|file| file.relative_path == *path)
                    .ok_or_else(|| validation(&format!("Compose file '{path}' is unavailable.")))?;
                String::from_utf8(file.content.clone())
                    .map_err(|_| validation(&format!("Compose file '{path}' is not valid UTF-8.")))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackSourceFile {
    pub relative_path: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDeletionClaim {
    pub stack_id: Uuid,
    pub platform_id: Uuid,
    pub project_name: String,
    pub platform_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackStateClaim {
    pub stack_id: Uuid,
    pub release_id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    pub project_name: String,
    pub platform_type: String,
    pub previous_status: StackReleaseStatus,
    pub actor_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackRuntimeResult {
    pub status: StackReleaseStatus,
    pub messages: Vec<StackProgressItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackBinding {
    pub name: String,
    pub value: zeroize::Zeroizing<String>,
    pub secret: bool,
    pub snapshot: ResourceBindingSnapshot,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedStackBindings {
    pub entries: Vec<StackBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackRuntimeService {
    pub docker_service_id: String,
    pub name: String,
    pub version_index: i64,
    pub desired_tasks: i32,
    pub running_tasks: i32,
    pub update_state: String,
    pub update_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackRuntimeContainer {
    pub docker_container_id: String,
    pub service_name: String,
    pub state: String,
    pub health: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackRuntimeSnapshot {
    pub containers: Vec<StackRuntimeContainer>,
    pub services: Vec<StackRuntimeService>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackImportClaim {
    pub platform_id: Uuid,
    pub platform_name: String,
    pub project_name: String,
    pub import_kind: StackImportKind,
    pub runtime_fingerprint: String,
    pub service_names: Vec<String>,
    pub container_ids: Vec<String>,
    pub container_names: Vec<String>,
    pub services: Vec<ComposeProjectRuntimeService>,
}
