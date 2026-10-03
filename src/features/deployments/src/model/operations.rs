use super::*;
use serde::Serialize;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentApplyError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePullProgress {
    pub current: Option<i64>,
    pub total: Option<i64>,
    pub start: Option<i64>,
    pub units: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeploymentProgress {
    pub id: Option<String>,
    pub status: Option<String>,
    pub stream: Option<String>,
    pub progress_message: Option<String>,
    pub error_message: Option<String>,
    pub progress: Option<ImagePullProgress>,
    pub error: Option<DeploymentApplyError>,
}

impl DeploymentProgress {
    #[must_use]
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            progress_message: Some(message.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn failure(code: i64, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            error_message: Some(message.clone()),
            error: Some(DeploymentApplyError { code, message }),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeletionClaim {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    pub docker_container_ids: Vec<String>,
    pub row_version: i64,
    pub previous_status: crate::DeploymentStatus,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
}

#[derive(Debug, Clone)]
pub struct ApplyClaim {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub platform_address: String,
    pub name: String,
    pub row_version: i64,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
    pub existing_container_id: Option<Uuid>,
    pub existing_docker_container_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedDeploymentImage {
    pub docker_image_id: String,
    pub digest: Option<String>,
    pub resolved_build: Option<ResolvedDeploymentBuild>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDeploymentBuild {
    pub image_reference: String,
    pub digest: Option<String>,
    pub build_run_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RuntimeDeploymentCommand {
    pub deployment_id: Uuid,
    pub name: String,
    pub image_id: String,
    pub spec: DeploymentSpec,
    pub environment_variables: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeContainerState {
    Running,
    Exited,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDeploymentResult {
    pub docker_container_id: String,
    pub docker_image_id: String,
    pub state: RuntimeContainerState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeploymentBindingSnapshot {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Kind")]
    pub kind: String,
    #[serde(rename = "Scope")]
    pub scope: String,
    #[serde(rename = "Value")]
    pub value: String,
    #[serde(rename = "SecretId", skip_serializing_if = "Option::is_none")]
    pub secret_id: Option<Uuid>,
    #[serde(rename = "SecretDeliveryMode", skip_serializing_if = "Option::is_none")]
    pub secret_delivery_mode: Option<String>,
    #[serde(rename = "TargetPath", skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
}

#[derive(Debug)]
pub struct ResolvedDeploymentBinding {
    pub name: String,
    pub value: zeroize::Zeroizing<String>,
    pub secret: bool,
    pub snapshot: DeploymentBindingSnapshot,
}

#[derive(Debug, Default)]
pub struct ResolvedDeploymentBindings {
    pub entries: Vec<ResolvedDeploymentBinding>,
}
