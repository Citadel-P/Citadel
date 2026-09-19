use super::spec::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackReleaseView {
    pub id: Uuid,
    pub stack_id: Uuid,
    pub platform_id: Uuid,
    pub status: StackReleaseStatus,
    pub version: String,
    pub spec: StackSpec,
    pub source: Option<StackReleaseSource>,
    pub resource_bindings: Option<Vec<ResourceBindingSnapshot>>,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    pub actor_name: String,
    pub actor_type: String,
    pub platform_status: String,
    pub platform_name: Option<String>,
}
impl From<citadel_stacks::StackReleaseDetails> for StackReleaseView {
    fn from(value: citadel_stacks::StackReleaseDetails) -> Self {
        Self {
            id: value.release.id,
            stack_id: value.release.stack_id,
            platform_id: value.release.platform_id,
            status: value.release.status.into(),
            version: value.release.version,
            spec: value.release.spec.into(),
            source: value.release.source.map(|item| item.into()),
            resource_bindings: value
                .release
                .resource_bindings
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            created_at: value.release.created_at,
            created_by_actor_id: value.release.created_by_actor_id,
            actor_name: value.actor_name,
            actor_type: value.actor_type,
            platform_status: value.platform_status,
            platform_name: value.platform_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackReleasesView {
    pub releases: Vec<StackReleaseView>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_delete: bool,
    pub can_view_logs: bool,
    pub can_inspect: bool,
    pub can_open_terminal: bool,
    pub can_pull: bool,
    pub can_apply: bool,
    pub can_view_resource_bindings: bool,
    pub can_view_releases: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[schema(as = stacks::model::ResourceCapabilities)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub stack_update_state: StackUpdateState,
    pub drift_policy: StackDriftPolicy,
    pub status: StackReleaseStatus,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    pub control_state: String,
    pub current_stack_release_id: Uuid,
    pub platform_type: String,
    pub platform_id: Option<Uuid>,
    pub version: Option<String>,
    pub spec: Option<StackSpec>,
    pub source: Option<StackReleaseSource>,
    pub resource_bindings: Option<Vec<ResourceBindingSnapshot>>,
    pub platform_status: String,
    pub platform_name: Option<String>,
    pub tags: Vec<TagSummary>,
    pub latest_activity_view: Option<Value>,
    pub capabilities: Option<StackCapabilities>,
    pub row_version: i64,
}
impl From<citadel_stacks::StackDetails> for StackView {
    fn from(value: citadel_stacks::StackDetails) -> Self {
        Self {
            id: value.stack.id,
            name: value.stack.name,
            description: value.stack.description,
            stack_source: value.stack.stack_source.into(),
            stack_update_state: value.stack.stack_update_state.into(),
            drift_policy: value.stack.drift_policy.into(),
            status: value.status.into(),
            created_at: value.stack.created_at,
            created_by_actor_id: value.stack.created_by_actor_id,
            control_state: value.stack.control_state,
            current_stack_release_id: value.stack.current_stack_release_id,
            platform_type: value.platform_type,
            platform_id: value.platform_id,
            version: value.version,
            spec: value.spec.map(|item| item.into()),
            source: value.source.map(|item| item.into()),
            resource_bindings: value
                .resource_bindings
                .map(|item| item.into_iter().map(|item| item.into()).collect()),
            platform_status: value.platform_status,
            platform_name: value.platform_name,
            tags: value.tags.into_iter().map(|item| item.into()).collect(),
            latest_activity_view: citadel_application::public_latest_activity(
                value.latest_activity,
            ),
            capabilities: Some(super::capabilities::capabilities(
                value.effective_permission,
            )),
            row_version: value.stack.row_version,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StacksView {
    pub stacks: Vec<StackView>,
    pub capabilities: ResourceCapabilities,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackConfigView {
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub platform_type: String,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub spec: StackSpec,
    pub stack_update_state: StackUpdateState,
    pub drift_policy: StackDriftPolicy,
    pub row_version: i64,
}
impl From<citadel_stacks::StackConfig> for StackConfigView {
    fn from(value: citadel_stacks::StackConfig) -> Self {
        Self {
            id: value.id,
            name: value.name,
            platform_id: value.platform_id,
            platform_type: value.platform_type,
            description: value.description,
            stack_source: value.stack_source.into(),
            spec: value.spec.into(),
            stack_update_state: value.stack_update_state.into(),
            drift_policy: value.drift_policy.into(),
            row_version: value.row_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackStreamItem {
    pub event_type: StackApplyEventType,
    pub message: Option<String>,
    pub exit_code: Option<i32>,
    pub stack_status: Option<StackReleaseStatus>,
    pub severity: Option<String>,
}
impl From<citadel_stacks::StackProgressItem> for StackStreamItem {
    fn from(value: citadel_stacks::StackProgressItem) -> Self {
        Self {
            event_type: value.event_type.into(),
            message: value.message,
            exit_code: value.exit_code,
            stack_status: value.stack_status.map(|item| item.into()),
            severity: value.severity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StackApplyEventType {
    Unknown,
    StdOut,
    StdErr,
    SystemMessage,
    CommandCompleted,
}
impl From<citadel_stacks::StackApplyEventType> for StackApplyEventType {
    fn from(value: citadel_stacks::StackApplyEventType) -> Self {
        match value {
            citadel_stacks::StackApplyEventType::Unknown => Self::Unknown,
            citadel_stacks::StackApplyEventType::StdOut => Self::StdOut,
            citadel_stacks::StackApplyEventType::StdErr => Self::StdErr,
            citadel_stacks::StackApplyEventType::SystemMessage => Self::SystemMessage,
            citadel_stacks::StackApplyEventType::CommandCompleted => Self::CommandCompleted,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackDuplicateDraftView {
    pub draft: Value,
    pub warnings: Vec<DuplicateDraftWarning>,
}
impl From<citadel_stacks::StackDuplicateDraft> for StackDuplicateDraftView {
    fn from(value: citadel_stacks::StackDuplicateDraft) -> Self {
        Self {
            draft: serde_json::json!({
                "name": value.draft.name,
                "platformId": value.draft.platform_id,
                "description": value.draft.description,
                "stackSource": StackSource::from(value.draft.stack_source),
                "spec": StackSpec::from(value.draft.spec),
                "driftPolicy": StackDriftPolicy::from(value.draft.drift_policy),
                "tagIds": value.draft.tag_ids,
                "duplicateSource": { "resourceType": "Stack", "resourceId": value.draft.source_id, "resourceName": value.draft.source_name },
            }),
            warnings: value.warnings.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectImportSourceView {
    pub platform_id: Uuid,
    pub platform_name: String,
    pub project_name: String,
    pub container_ids: Vec<String>,
    pub container_names: Vec<String>,
    pub services: Vec<ComposeProjectRuntimeService>,
}
impl From<citadel_stacks::ComposeProjectImportSource> for ComposeProjectImportSourceView {
    fn from(value: citadel_stacks::ComposeProjectImportSource) -> Self {
        Self {
            platform_id: value.platform_id,
            platform_name: value.platform_name,
            project_name: value.project_name,
            container_ids: value.container_ids,
            container_names: value.container_names,
            services: value.services.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectStackDraftView {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub drift_policy: StackDriftPolicy,
    pub tag_ids: Vec<Uuid>,
}
impl From<citadel_stacks::ComposeProjectStackDraft> for ComposeProjectStackDraftView {
    fn from(value: citadel_stacks::ComposeProjectStackDraft) -> Self {
        Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            drift_policy: value.drift_policy.into(),
            tag_ids: value.tag_ids,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectImportDraftView {
    pub import_kind: StackImportKind,
    pub source: ComposeProjectImportSourceView,
    pub draft: ComposeProjectStackDraftView,
    pub issues: Vec<StackAdoptionIssue>,
    pub runtime_fingerprint: String,
}
impl From<citadel_stacks::ComposeProjectImportDraft> for ComposeProjectImportDraftView {
    fn from(value: citadel_stacks::ComposeProjectImportDraft) -> Self {
        Self {
            import_kind: value.import_kind.into(),
            source: value.source.into(),
            draft: value.draft.into(),
            issues: value.issues.into_iter().map(|item| item.into()).collect(),
            runtime_fingerprint: value.runtime_fingerprint,
        }
    }
}

impl Serialize for StackStreamItem {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;

        let mut item = serializer.serialize_map(None)?;
        item.serialize_entry("type", &self.event_type)?;
        if let Some(message) = &self.message {
            // Docker also writes normal progress to stderr. The public `message`
            // field is reserved for errors; .NET uses `progressMessage` otherwise.
            let field = if self.exit_code.is_some_and(|code| code != 0) {
                "message"
            } else {
                "progressMessage"
            };
            item.serialize_entry(field, message)?;
        }
        if let Some(code) = self.exit_code {
            item.serialize_entry("exitCode", &code)?;
        }
        if let Some(status) = self.stack_status {
            item.serialize_entry("stackStatus", &status)?;
        }
        if let Some(severity) = &self.severity {
            item.serialize_entry("severity", severity)?;
        }
        item.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_stream_serializes_normal_output_as_progress() {
        for event_type in [
            citadel_stacks::StackApplyEventType::SystemMessage,
            citadel_stacks::StackApplyEventType::StdOut,
            citadel_stacks::StackApplyEventType::StdErr,
        ] {
            let mut item = citadel_stacks::StackProgressItem::system("Container beszel Started");
            item.event_type = event_type;
            let wire = serde_json::to_value(StackStreamItem::from(item)).unwrap();
            assert_eq!(wire["progressMessage"], "Container beszel Started");
            assert!(
                wire.get("message").is_none(),
                "Normal output must not populate the error field"
            );
        }
    }

    #[test]
    fn stack_stream_keeps_success_and_failure_distinct() {
        let success = serde_json::to_value(StackStreamItem::from(
            citadel_stacks::StackProgressItem::completed(
                citadel_stacks::StackReleaseStatus::Healthy,
                "Stack deployment completed.",
            ),
        ))
        .unwrap();
        assert_eq!(success["progressMessage"], "Stack deployment completed.");
        assert_eq!(success["exitCode"], 0);
        assert_eq!(success["severity"], "success");
        assert!(success.get("message").is_none());
        let failed = serde_json::to_value(StackStreamItem::from(
            citadel_stacks::StackProgressItem::completed(
                citadel_stacks::StackReleaseStatus::Failed,
                "Docker deployment failed.",
            ),
        ))
        .unwrap();
        assert_eq!(failed["message"], "Docker deployment failed.");
        assert_eq!(failed["exitCode"], 1);
        assert_eq!(failed["severity"], "error");
        assert!(failed.get("progressMessage").is_none());
    }

    #[test]
    fn bindings_capability_uses_the_frontend_contract() {
        let value = serde_json::to_value(StackCapabilities {
            can_view_resource_bindings: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(value["canViewResourceBindings"], true);
    }
}
