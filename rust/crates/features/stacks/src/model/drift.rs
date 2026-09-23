use serde::Serialize;

use uuid::Uuid;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackReconciliationStatus {
    NoDrift,
    Reconciled,
    Partial,
    RequiresReapply,
    Disabled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
