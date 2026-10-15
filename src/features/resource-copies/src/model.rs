use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CopyKind {
    Deployment,
    Stack,
    SwarmService,
}
impl CopyKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deployment => "Deployment",
            Self::Stack => "Stack",
            Self::SwarmService => "SwarmService",
        }
    }
    pub const fn resource_type(self) -> citadel_primitives::ResourceType {
        match self {
            Self::Deployment => citadel_primitives::ResourceType::Deployment,
            Self::Stack => citadel_primitives::ResourceType::Stack,
            Self::SwarmService => citadel_primitives::ResourceType::SwarmService,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "spec")]
pub enum CopySpec {
    Deployment(Box<citadel_deployments::DeploymentSpec>),
    Stack(Box<citadel_stacks::StackSpec>),
    SwarmService(Box<citadel_swarm_services::SwarmServiceSpec>),
}

/// Internal only: source configuration and binding fingerprint never enter a UI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopySource {
    pub kind: CopyKind,
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub platform_name: String,
    pub swarm: bool,
    pub fingerprint: String,
    pub spec: CopySpec,
    pub description: Option<String>,
    pub tag_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopySelection {
    pub volume: String,
    pub backup_run_id: Uuid,
    pub backup_run_item_id: Uuid,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CopyDataMode {
    CurrentData,
    #[default]
    ExistingBackup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyRequest {
    #[serde(default)]
    pub destination_platform_id: Option<Uuid>,
    #[serde(default)]
    pub destination_node_id: Option<String>,
    #[serde(default)]
    pub data_mode: CopyDataMode,
    #[serde(default)]
    pub consistency_acknowledged: bool,
    pub kind: CopyKind,
    pub source_id: Uuid,
    pub name: String,
    pub selections: Vec<CopySelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopySnapshot {
    #[serde(default)]
    pub supports_cross_platform_restore: bool,
    pub backup_run_id: Uuid,
    pub backup_run_item_id: Uuid,
    pub policy_id: Uuid,
    pub policy_name: String,
    pub repository_id: Uuid,
    pub volume: String,
    pub docker_node_id: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyVolume {
    pub source: String,
    pub target: String,
    pub docker_node_id: Option<String>,
    pub snapshots: Vec<CopySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyPreview {
    #[serde(default)]
    pub destination_node_id: Option<String>,
    pub id: Uuid,
    pub source_name: String,
    pub platform_id: Uuid,
    pub platform_name: String,
    pub name: String,
    pub volumes: Vec<CopyVolume>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyPlan {
    #[serde(default)]
    pub destination: Option<CopyDestination>,
    /// Durable, policy-independent snapshot/restore receipts. Running receipts
    /// are failed on worker recovery, never blindly executed a second time.
    #[serde(default)]
    pub current_volumes: Vec<CopyRestore>,
    pub request: CopyRequest,
    pub source: CopySource,
    pub preview: CopyPreview,
    pub spec: CopySpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CopyStatus {
    Review,
    Restoring,
    Cancelling,
    Cleaning,
    Cleaned,
    Succeeded,
    Failed,
    Cancelled,
}
impl CopyStatus {
    pub const fn terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Cleaned
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyProgress {
    pub data_mode: CopyDataMode,
    pub id: Uuid,
    pub kind: CopyKind,
    pub source_id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub status: CopyStatus,
    pub destination_id: Option<Uuid>,
    pub message: Option<String>,
    pub restores: Vec<CopyRestore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyRestore {
    pub id: Uuid,
    pub source: String,
    pub target: String,
    pub status: String,
    pub message: Option<String>,
}

/// Actor-specific preflight hint. Starting a copy still revalidates its source,
/// permissions, snapshots and runtime storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyEligibility {
    pub status: CopyEligibilityStatus,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CopyEligibilityStatus {
    Available,
    Unavailable,
    Unknown,
}
impl CopyEligibility {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            status: CopyEligibilityStatus::Unavailable,
            reason: Some(reason.into()),
        }
    }
}

/// Reviewed destination identity; observation counters are deliberately excluded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CopyDestination {
    pub platform_id: Uuid,
    pub platform_name: String,
    pub docker_node_id: Option<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyTarget {
    pub id: Uuid,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyTargets {
    pub source_platform_id: Uuid,
    pub source_platform_name: String,
    pub source_node_id: Option<String>,
    pub swarm: bool,
    pub platforms: Vec<CopyTarget>,
}

impl CopyPlan {
    pub fn target_platform(&self) -> Uuid {
        self.destination
            .as_ref()
            .map_or(self.source.platform_id, |d| d.platform_id)
    }
    pub fn target_node<'a>(&'a self, volume: &'a CopyVolume) -> Option<&'a str> {
        self.destination
            .as_ref()
            .map_or(volume.docker_node_id.as_deref(), |d| {
                d.docker_node_id.as_deref()
            })
    }
    pub fn moves_storage(&self) -> bool {
        self.target_platform() != self.source.platform_id
            || self
                .preview
                .volumes
                .iter()
                .any(|v| self.target_node(v) != v.docker_node_id.as_deref())
    }
}
