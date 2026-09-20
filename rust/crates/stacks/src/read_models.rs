//! Transport-neutral read results and supporting filters.
//! Repository implementations populate these types; server adapters map them to HTTP views.

use serde_json::Value;

use crate::*;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackReleaseDetails {
    pub release: StackRelease,
    pub actor_name: String,
    pub actor_type: String,
    pub platform_status: String,
    pub platform_name: Option<String>,
}

impl std::ops::Deref for StackReleaseDetails {
    type Target = StackRelease;
    fn deref(&self) -> &Self::Target {
        &self.release
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}
#[derive(Debug, Clone, PartialEq)]
pub struct StackDetails {
    pub stack: Stack,
    pub status: StackReleaseStatus,
    pub platform_type: citadel_platforms::PlatformKind,
    pub platform_id: Option<Uuid>,
    pub version: Option<String>,
    pub spec: Option<StackSpec>,
    pub source: Option<StackReleaseSource>,
    pub resource_bindings: Option<Vec<ResourceBindingSnapshot>>,
    pub platform_status: String,
    pub platform_name: Option<String>,
    pub tags: Vec<TagSummary>,
    pub latest_activity: Option<Value>,
    pub effective_permission: citadel_primitives::EffectivePermission,
}

impl std::ops::Deref for StackDetails {
    type Target = Stack;
    fn deref(&self) -> &Self::Target {
        &self.stack
    }
}

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

impl From<StackDetails> for StackConfig {
    fn from(value: StackDetails) -> Self {
        Self {
            id: value.stack.id,
            name: value.stack.name,
            platform_id: value.platform_id.unwrap_or_default(),
            platform_type: value.platform_type,
            description: value.stack.description,
            stack_source: value.stack.stack_source,
            spec: value
                .spec
                .expect("Stack persistence always includes a current release"),
            stack_update_state: value.stack.stack_update_state,
            drift_policy: value.stack.drift_policy,
            row_version: value.stack.row_version,
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
