use crate::api::resources::bindings::spec::{
    ResourceBindingKind, ResourceBindingScope, SecretDeliveryMode,
};
pub use citadel_stacks::{
    ComposeProjectImportValidation, ComposeProjectRuntimeService, ComposeProjectServiceComparison,
    ImageUpdateState, RecreateStackOnNewCommitState, RecreateStackOnNewImageState,
    StackAdoptionIssue, StackBuildImageBinding, StackCommand, StackDriftMode, StackDriftPolicy,
    StackReleaseSource, StackReleaseStatus, StackSource, StackSpec, StackSpecCommon,
    StackUpdateBehavior, StackUpdateState, StackWebhookConfig,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

impl TryFrom<citadel_stacks::ResourceBindingSnapshot> for ResourceBindingSnapshot {
    type Error = serde_json::Error;

    fn try_from(value: citadel_stacks::ResourceBindingSnapshot) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            kind: serde_json::from_value(value.kind.into())?,
            scope: serde_json::from_value(value.scope.into())?,
            value: value.value,
            secret_id: value.secret_id,
            secret_delivery_mode: value
                .secret_delivery_mode
                .map(|value| serde_json::from_value(value.into()))
                .transpose()?,
            target_path: value.target_path,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateDraftWarning {
    pub code: String,
    pub message: String,
    #[schema(required = true)]
    pub field_path: Option<String>,
}

impl From<citadel_stacks::DuplicateDraftWarning> for DuplicateDraftWarning {
    fn from(value: citadel_stacks::DuplicateDraftWarning) -> Self {
        Self {
            code: value.code,
            message: value.message,
            field_path: value.field_path,
        }
    }
}

impl From<DuplicateDraftWarning> for citadel_stacks::DuplicateDraftWarning {
    fn from(value: DuplicateDraftWarning) -> Self {
        Self {
            code: value.code,
            message: value.message,
            field_path: value.field_path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackDriftReport {
    pub stack_id: Uuid,
    pub platform_id: Uuid,
    pub has_drift: bool,
    pub has_auto_fixable_drift: bool,
    pub has_structural_drift: bool,
    pub drifts: Vec<StackDrift>,
}

impl From<citadel_stacks::StackDriftReport> for StackDriftReport {
    fn from(value: citadel_stacks::StackDriftReport) -> Self {
        Self {
            stack_id: value.stack_id,
            platform_id: value.platform_id,
            has_drift: value.has_drift,
            has_auto_fixable_drift: value.has_auto_fixable_drift,
            has_structural_drift: value.has_structural_drift,
            drifts: value.drifts.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<StackDriftReport> for citadel_stacks::StackDriftReport {
    fn from(value: StackDriftReport) -> Self {
        Self {
            stack_id: value.stack_id,
            platform_id: value.platform_id,
            has_drift: value.has_drift,
            has_auto_fixable_drift: value.has_auto_fixable_drift,
            has_structural_drift: value.has_structural_drift,
            drifts: value.drifts.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(tag = "$type", rename_all_fields = "camelCase")]
pub enum StackDrift {
    MissingContainer {
        #[serde(rename = "serviceName")]
        service_name: String,
    },
    ExtraContainer {
        #[serde(rename = "containerId")]
        container_id: String,
        #[serde(rename = "serviceName")]
        service_name: String,
    },
    ContainerStopped {
        #[serde(rename = "containerId")]
        container_id: String,
        #[serde(rename = "serviceName")]
        service_name: String,
    },
    ContainerPaused {
        #[serde(rename = "containerId")]
        container_id: String,
        #[serde(rename = "serviceName")]
        service_name: String,
    },
    ContainerUnhealthy {
        #[serde(rename = "containerId")]
        container_id: String,
        #[serde(rename = "serviceName")]
        service_name: String,
        #[serde(rename = "healthStatus")]
        health_status: Option<String>,
    },
    ImageMismatch {
        #[serde(rename = "serviceName")]
        service_name: String,
        #[serde(rename = "expectedImage")]
        expected_image: String,
        #[serde(rename = "actualImage")]
        actual_image: String,
    },
    ConfigHashMismatch {
        #[serde(rename = "serviceName")]
        service_name: String,
        #[serde(rename = "expectedHash")]
        expected_hash: Option<String>,
        #[serde(rename = "actualHash")]
        actual_hash: Option<String>,
    },
}

impl From<citadel_stacks::StackDrift> for StackDrift {
    fn from(value: citadel_stacks::StackDrift) -> Self {
        match value {
            citadel_stacks::StackDrift::MissingContainer { service_name } => {
                Self::MissingContainer { service_name }
            }
            citadel_stacks::StackDrift::ExtraContainer {
                container_id,
                service_name,
            } => Self::ExtraContainer {
                container_id,
                service_name,
            },
            citadel_stacks::StackDrift::ContainerStopped {
                container_id,
                service_name,
            } => Self::ContainerStopped {
                container_id,
                service_name,
            },
            citadel_stacks::StackDrift::ContainerPaused {
                container_id,
                service_name,
            } => Self::ContainerPaused {
                container_id,
                service_name,
            },
            citadel_stacks::StackDrift::ContainerUnhealthy {
                container_id,
                service_name,
                health_status,
            } => Self::ContainerUnhealthy {
                container_id,
                service_name,
                health_status,
            },
            citadel_stacks::StackDrift::ImageMismatch {
                service_name,
                expected_image,
                actual_image,
            } => Self::ImageMismatch {
                service_name,
                expected_image,
                actual_image,
            },
            citadel_stacks::StackDrift::ConfigHashMismatch {
                service_name,
                expected_hash,
                actual_hash,
            } => Self::ConfigHashMismatch {
                service_name,
                expected_hash,
                actual_hash,
            },
        }
    }
}

impl From<StackDrift> for citadel_stacks::StackDrift {
    fn from(value: StackDrift) -> Self {
        match value {
            StackDrift::MissingContainer { service_name } => {
                Self::MissingContainer { service_name }
            }
            StackDrift::ExtraContainer {
                container_id,
                service_name,
            } => Self::ExtraContainer {
                container_id,
                service_name,
            },
            StackDrift::ContainerStopped {
                container_id,
                service_name,
            } => Self::ContainerStopped {
                container_id,
                service_name,
            },
            StackDrift::ContainerPaused {
                container_id,
                service_name,
            } => Self::ContainerPaused {
                container_id,
                service_name,
            },
            StackDrift::ContainerUnhealthy {
                container_id,
                service_name,
                health_status,
            } => Self::ContainerUnhealthy {
                container_id,
                service_name,
                health_status,
            },
            StackDrift::ImageMismatch {
                service_name,
                expected_image,
                actual_image,
            } => Self::ImageMismatch {
                service_name,
                expected_image,
                actual_image,
            },
            StackDrift::ConfigHashMismatch {
                service_name,
                expected_hash,
                actual_hash,
            } => Self::ConfigHashMismatch {
                service_name,
                expected_hash,
                actual_hash,
            },
        }
    }
}

pub use citadel_stacks::{StackReconciliationActionType, StackReconciliationStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackReconciliationAction {
    pub container_id: String,
    pub service_name: String,
    #[schema(value_type = crate::api::resources::schema_models::stacks::StackReconciliationActionTypeSchema)]
    pub action: StackReconciliationActionType,
    pub succeeded: bool,
    #[schema(required = true)]
    pub error_message: Option<String>,
}

impl From<citadel_stacks::StackReconciliationAction> for StackReconciliationAction {
    fn from(value: citadel_stacks::StackReconciliationAction) -> Self {
        Self {
            container_id: value.container_id,
            service_name: value.service_name,
            action: value.action,
            succeeded: value.succeeded,
            error_message: value.error_message,
        }
    }
}

impl From<StackReconciliationAction> for citadel_stacks::StackReconciliationAction {
    fn from(value: StackReconciliationAction) -> Self {
        Self {
            container_id: value.container_id,
            service_name: value.service_name,
            action: value.action,
            succeeded: value.succeeded,
            error_message: value.error_message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackReconciliationResult {
    pub stack_id: Uuid,
    #[schema(value_type = crate::api::resources::schema_models::stacks::StackReconciliationStatusSchema)]
    pub status: StackReconciliationStatus,
    pub before_report: StackDriftReport,
    #[schema(required = true)]
    pub after_report: Option<StackDriftReport>,
    pub actions: Vec<StackReconciliationAction>,
}

impl From<citadel_stacks::StackReconciliationResult> for StackReconciliationResult {
    fn from(value: citadel_stacks::StackReconciliationResult) -> Self {
        Self {
            stack_id: value.stack_id,
            status: value.status,
            before_report: value.before_report.into(),
            after_report: value.after_report.map(|item| item.into()),
            actions: value.actions.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<StackReconciliationResult> for citadel_stacks::StackReconciliationResult {
    fn from(value: StackReconciliationResult) -> Self {
        Self {
            stack_id: value.stack_id,
            status: value.status,
            before_report: value.before_report.into(),
            after_report: value.after_report.map(|item| item.into()),
            actions: value.actions.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum StackImportKind {
    ComposeProject,
    SwarmStack,
}

impl From<citadel_stacks::StackImportKind> for StackImportKind {
    fn from(value: citadel_stacks::StackImportKind) -> Self {
        match value {
            citadel_stacks::StackImportKind::ComposeProject => Self::ComposeProject,
            citadel_stacks::StackImportKind::SwarmStack => Self::SwarmStack,
        }
    }
}

impl From<StackImportKind> for citadel_stacks::StackImportKind {
    fn from(value: StackImportKind) -> Self {
        match value {
            StackImportKind::ComposeProject => Self::ComposeProject,
            StackImportKind::SwarmStack => Self::SwarmStack,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmStackCompatibilityIssue {
    pub severity: SwarmStackCompatibilitySeverity,
    pub code: String,
    pub message: String,
    pub field_path: Option<String>,
}

impl From<citadel_stacks::SwarmStackCompatibilityIssue> for SwarmStackCompatibilityIssue {
    fn from(value: citadel_stacks::SwarmStackCompatibilityIssue) -> Self {
        Self {
            severity: value.severity.into(),
            code: value.code,
            message: value.message,
            field_path: value.field_path,
        }
    }
}

impl From<SwarmStackCompatibilityIssue> for citadel_stacks::SwarmStackCompatibilityIssue {
    fn from(value: SwarmStackCompatibilityIssue) -> Self {
        Self {
            severity: value.severity.into(),
            code: value.code,
            message: value.message,
            field_path: value.field_path,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum SwarmStackCompatibilitySeverity {
    Warning,
    Error,
}

impl From<citadel_stacks::SwarmStackCompatibilitySeverity> for SwarmStackCompatibilitySeverity {
    fn from(value: citadel_stacks::SwarmStackCompatibilitySeverity) -> Self {
        match value {
            citadel_stacks::SwarmStackCompatibilitySeverity::Warning => Self::Warning,
            citadel_stacks::SwarmStackCompatibilitySeverity::Error => Self::Error,
        }
    }
}

impl From<SwarmStackCompatibilitySeverity> for citadel_stacks::SwarmStackCompatibilitySeverity {
    fn from(value: SwarmStackCompatibilitySeverity) -> Self {
        match value {
            SwarmStackCompatibilitySeverity::Warning => Self::Warning,
            SwarmStackCompatibilitySeverity::Error => Self::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmStackCompatibilityReport {
    pub is_compatible: bool,
    pub issues: Vec<SwarmStackCompatibilityIssue>,
}

impl From<citadel_stacks::SwarmStackCompatibilityReport> for SwarmStackCompatibilityReport {
    fn from(value: citadel_stacks::SwarmStackCompatibilityReport) -> Self {
        Self {
            is_compatible: value.is_compatible,
            issues: value.issues.into_iter().map(|item| item.into()).collect(),
        }
    }
}

impl From<SwarmStackCompatibilityReport> for citadel_stacks::SwarmStackCompatibilityReport {
    fn from(value: SwarmStackCompatibilityReport) -> Self {
        Self {
            is_compatible: value.is_compatible,
            issues: value.issues.into_iter().map(|item| item.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBindingSnapshot {
    pub name: String,
    pub kind: ResourceBindingKind,
    pub scope: ResourceBindingScope,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_delivery_mode: Option<SecretDeliveryMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
}
