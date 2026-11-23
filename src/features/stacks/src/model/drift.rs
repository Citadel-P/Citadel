use serde::Serialize;

use uuid::Uuid;

/// Inputs and optimistic write fence for a single Stack drift observation.
#[derive(Debug)]
pub struct StackDriftContext {
    pub id: Uuid,
    pub name: String,
    pub drift_policy: super::StackDriftPolicy,
    pub control_state: citadel_primitives::ResourceControlState,
    pub current_stack_release_id: Uuid,
    pub row_version: i64,
    pub input_revision: Option<Uuid>,
    pub status: super::StackReleaseStatus,
    pub platform_type: citadel_platforms::PlatformKind,
    pub platform_status: citadel_primitives::PlatformStatus,
    pub platform_id: Option<Uuid>,
    pub spec: Option<super::StackSpec>,
    pub deployed_commit: Option<String>,
    pub previous_fingerprint: Option<String>,
}

impl From<super::Stack> for StackDriftContext {
    fn from(stack: super::Stack) -> Self {
        let previous_fingerprint = stack
            .latest_activity
            .as_ref()
            .map(|a| &a.info)
            .filter(|info| info.get("$type").and_then(|v| v.as_str()) == Some("StackDriftDetected"))
            .and_then(|info| info.get("Fingerprint").or_else(|| info.get("fingerprint")))
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        Self {
            id: stack.id,
            name: stack.name,
            drift_policy: stack.drift_policy,
            control_state: stack.control_state,
            current_stack_release_id: stack.current_stack_release_id,
            row_version: stack.row_version,
            input_revision: None,
            status: stack.status,
            platform_type: stack.platform_type,
            platform_status: stack.platform_status,
            platform_id: stack.platform_id,
            spec: stack.spec,
            deployed_commit: stack.source.map(|s| s.resolved_commit_sha),
            previous_fingerprint,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDriftReport {
    pub stack_id: Uuid,
    pub platform_id: Uuid,
    pub has_drift: bool,
    pub has_auto_fixable_drift: bool,
    pub has_structural_drift: bool,
    pub drifts: Vec<StackDrift>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDriftMonitorFailure {
    pub stack_id: Uuid,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDriftMonitorResult {
    pub checked: usize,
    pub reconciled: usize,
    pub failures: Vec<StackDriftMonitorFailure>,
    /// Continue after this candidate when the fetched page was full.
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "$type", rename_all_fields = "camelCase")]
pub enum StackDrift {
    MissingContainer {
        service_name: String,
    },
    ExtraContainer {
        container_id: String,
        service_name: String,
    },
    ContainerStopped {
        container_id: String,
        service_name: String,
    },
    ContainerPaused {
        container_id: String,
        service_name: String,
    },
    ContainerUnhealthy {
        container_id: String,
        service_name: String,
        health_status: Option<String>,
    },
    ImageMismatch {
        service_name: String,
        expected_image: String,
        actual_image: String,
    },
    ConfigHashMismatch {
        service_name: String,
        expected_hash: Option<String>,
        actual_hash: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
pub enum StackReconciliationStatus {
    NoDrift,
    Reconciled,
    Partial,
    RequiresReapply,
    Disabled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
pub enum StackReconciliationActionType {
    StartContainer,
    ResumeContainer,
    RemoveContainer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackReconciliationAction {
    pub container_id: String,
    pub service_name: String,
    pub action: StackReconciliationActionType,
    pub succeeded: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackReconciliationResult {
    pub stack_id: Uuid,
    pub status: StackReconciliationStatus,
    pub before_report: StackDriftReport,
    pub after_report: Option<StackDriftReport>,
    pub actions: Vec<StackReconciliationAction>,
}
