use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwarmServiceProgressItem {
    pub service_id: Uuid,
    pub operation_id: Option<Uuid>,
    pub stage: String,
    pub message: String,
    pub is_completed: bool,
    pub is_warning: bool,
    pub error_message: Option<String>,
}

impl SwarmServiceProgressItem {
    pub fn info(
        service_id: Uuid,
        operation_id: Option<Uuid>,
        stage: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            service_id,
            operation_id,
            stage: stage.to_owned(),
            message: message.into(),
            is_completed: false,
            is_warning: false,
            error_message: None,
        }
    }
    pub fn completed(service_id: Uuid, operation_id: Uuid, message: impl Into<String>) -> Self {
        Self {
            is_completed: true,
            ..Self::info(service_id, Some(operation_id), "Completed", message)
        }
    }
    pub fn failed(
        service_id: Uuid,
        operation_id: Option<Uuid>,
        message: impl Into<String>,
    ) -> Self {
        let message = message.into();
        Self {
            error_message: Some(message.clone()),
            ..Self::info(service_id, operation_id, "Failed", message)
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceOperationClaim {
    pub operation_id: Uuid,
    pub id: Uuid,
    pub platform_id: Uuid,
    pub docker_name: String,
    pub docker_service_id: Option<String>,
    pub docker_version_index: Option<i64>,
    pub row_version: i64,
    pub desired_hash: String,
    pub spec: SwarmServiceSpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDeletionClaim {
    pub operation_id: Uuid,
    pub id: Uuid,
    pub platform_id: Uuid,
    pub docker_service_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeServiceResult {
    pub docker_service_id: String,
    pub version_index: i64,
    pub accepted: bool,
    pub rollout_complete: bool,
    pub rollout_error: Option<String>,
    pub runtime_hash: String,
    pub applied_digest: Option<String>,
    pub warnings: Vec<String>,
}

/// Operations accepted by the apply pipeline; deletion uses its own claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceOperationKind {
    Apply,
    Scale,
    ForceUpdate,
}

impl ServiceOperationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Apply => SwarmServiceOperationKind::Apply.as_str(),
            Self::Scale => SwarmServiceOperationKind::Scale.as_str(),
            Self::ForceUpdate => SwarmServiceOperationKind::ForceUpdate.as_str(),
        }
    }
}
