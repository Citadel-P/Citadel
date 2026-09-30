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

#[derive(Debug, Clone, Default)]
pub struct StackFilter {
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
