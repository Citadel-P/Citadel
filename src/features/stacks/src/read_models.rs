//! Transport-neutral read results and supporting filters.
//! Repository implementations populate these types; server adapters map them to HTTP views.
use citadel_primitives::AuthorizedResource;

use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct StackConfig {
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub platform_type: citadel_platforms::PlatformKind,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub spec: StackSpec,
    pub stack_update_state: StackUpdateState,
    pub drift_policy: StackDriftPolicy,
    pub row_version: i64,
}

impl From<AuthorizedResource<crate::Stack>> for StackConfig {
    fn from(value: AuthorizedResource<crate::Stack>) -> Self {
        let value = value.resource;
        Self {
            id: value.id,
            name: value.name,
            platform_id: value.platform_id.unwrap_or_default(),
            platform_type: value.platform_type,
            description: value.description,
            stack_source: value.stack_source,
            spec: value
                .spec
                .expect("Stack persistence always includes a current release"),
            stack_update_state: value.stack_update_state,
            drift_policy: value.drift_policy,
            row_version: value.row_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackCatalogGit {
    pub git_repository_name: Option<String>,
    pub branch: Option<String>,
    pub has_matching_applied_commit: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StackFilter {
    pub catalog: bool,
    pub limit: Option<i64>,
    pub resource_ids: Option<Vec<Uuid>>,
    pub container_ids: Option<Vec<String>>,
    pub tags: Vec<String>,
    pub platform_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDuplicateDraft {
    pub draft: StackDraft,
    pub warnings: Vec<DuplicateDraftWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateDraftWarning {
    pub code: String,
    pub message: String,
    pub field_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeProjectImportSource {
    pub platform_id: Uuid,
    pub platform_name: String,
    pub project_name: String,
    pub container_ids: Vec<String>,
    pub container_names: Vec<String>,
    pub services: Vec<ComposeProjectRuntimeService>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeProjectStackDraft {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub drift_policy: StackDriftPolicy,
    pub tag_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeProjectImportDraft {
    pub import_kind: StackImportKind,
    pub source: ComposeProjectImportSource,
    pub draft: ComposeProjectStackDraft,
    pub issues: Vec<StackAdoptionIssue>,
    pub runtime_fingerprint: String,
}

/// Editable configuration copied from an existing Stack, without wire envelopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDraft {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub spec: StackSpec,
    pub drift_policy: StackDriftPolicy,
    pub tag_ids: Vec<Uuid>,
    pub source_id: Uuid,
    pub source_name: String,
}

/// Data needed to act on a known update. The candidate query reads this snapshot
/// only when the current release and healthy ref still match the checked inputs.
pub struct StackUpdateAction {
    pub id: Uuid,
    pub name: String,
    pub current_stack_release_id: Uuid,
    pub row_version: i64,
    pub behavior: StackUpdateBehavior,
    pub service_only_supported: bool,
    pub pending_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub branch: Option<String>,
    pub repository_name: Option<String>,
    pub applied_commit: Option<String>,
    pub stack_update_state: StackUpdateState,
}

impl TryFrom<Stack> for StackUpdateAction {
    type Error = StackError;
    fn try_from(stack: Stack) -> Result<Self, StackError> {
        let spec = stack.spec.ok_or(StackError::NotFound)?;
        let service_only_supported = spec.supports_service_auto_deploy(stack.platform_type);
        let (behavior, branch) = match spec {
            StackSpec::Git {
                update_behavior,
                branch,
                ..
            } => (update_behavior, Some(branch)),
            StackSpec::WebEditor {
                update_behavior, ..
            } => (update_behavior, None),
        };
        let (repository_name, applied_commit) = stack
            .source
            .map(|source| {
                (
                    source.git_repository_name,
                    Some(source.resolved_commit_sha).filter(|s| !s.is_empty()),
                )
            })
            .unwrap_or_default();
        Ok(Self {
            id: stack.id,
            name: stack.name,
            current_stack_release_id: stack.current_stack_release_id,
            row_version: stack.row_version,
            behavior,
            service_only_supported,
            pending_sync_at: None,
            branch,
            repository_name,
            applied_commit,
            stack_update_state: stack.stack_update_state,
        })
    }
}
