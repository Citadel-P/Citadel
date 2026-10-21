//! Resource copy schemas and actor-specific eligibility presentation.
#![allow(dead_code)]
use super::{
    deployments::views::DeploymentView, stacks::views::StackView,
    swarm_services::views::ManagedSwarmServiceView,
};
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use citadel_resource_copies::{CopyEligibility, CopyEligibilityStatus, CopyKind, CopyService};
use futures_util::{StreamExt, stream};
use uuid::Uuid;

#[derive(serde::Serialize, utoipa::ToSchema)]
pub enum ResourceCopyKind {
    Deployment,
    Stack,
    SwarmService,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCopySelection {
    pub volume: String,
    pub backup_run_id: Uuid,
    pub backup_run_item_id: Uuid,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCopyRequest {
    pub destination_platform_id: Option<Uuid>,
    pub destination_node_id: Option<String>,
    pub data_mode: Option<ResourceCopyDataMode>,
    pub consistency_acknowledged: Option<bool>,
    pub kind: ResourceCopyKind,
    pub source_id: Uuid,
    pub name: String,
    pub selections: Vec<ResourceCopySelection>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCopySnapshot {
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

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCopyVolume {
    pub source: String,
    pub target: String,
    pub docker_node_id: Option<String>,
    pub snapshots: Vec<ResourceCopySnapshot>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCopyPreview {
    pub destination_node_id: Option<String>,
    pub id: Uuid,
    pub source_name: String,
    pub platform_id: Uuid,
    pub platform_name: String,
    pub name: String,
    pub volumes: Vec<ResourceCopyVolume>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub enum ResourceCopyStatus {
    Review,
    Restoring,
    Cancelling,
    Cleaning,
    Cleaned,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCopyProgress {
    pub data_mode: ResourceCopyDataMode,
    pub id: Uuid,
    pub kind: ResourceCopyKind,
    pub source_id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub status: ResourceCopyStatus,
    pub destination_id: Option<Uuid>,
    pub message: Option<String>,
    pub restores: Vec<ResourceCopyRestore>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCopyRestore {
    pub id: Uuid,
    pub source: String,
    pub target: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub enum ResourceCopyEligibilityStatus {
    Available,
    Unavailable,
    Unknown,
}

/// Actor-specific configuration-plus-data eligibility, included in list/detail reads
/// and realtime snapshots. Preview and submission revalidate this hint.
#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCopyEligibility {
    pub status: ResourceCopyEligibilityStatus,
    pub reason: Option<String>,
}

pub async fn eligibility(
    service: &CopyService,
    actor: ActorId,
    kind: CopyKind,
    id: Uuid,
) -> CopyEligibility {
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        service.eligibility(actor, kind, id),
    )
    .await
    {
        Ok(Ok(value)) => value,
        result => {
            tracing::warn!(?kind, %id, ?result, "Unable to read resource copy eligibility");
            CopyEligibility {
                status: CopyEligibilityStatus::Unknown,
                reason: Some(
                    "Copy availability could not be checked. Refresh the resource to try again."
                        .into(),
                ),
            }
        }
    }
}

// Enrich authorized list rows in place, preserving their order.
// Each resource reuses the detail check with at most four concurrent reads.
pub async fn enrich_deployments(
    service: &CopyService,
    actor: ActorId,
    views: &mut [DeploymentView],
) {
    stream::iter(views)
        .for_each_concurrent(4, |view| async move {
            view.copy_eligibility =
                Some(eligibility(service, actor, CopyKind::Deployment, view.id).await);
        })
        .await;
}

pub async fn enrich_stacks(service: &CopyService, actor: ActorId, views: &mut [StackView]) {
    stream::iter(views)
        .for_each_concurrent(4, |view| async move {
            view.copy_eligibility =
                Some(eligibility(service, actor, CopyKind::Stack, view.id).await);
        })
        .await;
}

pub async fn enrich_swarm_services(
    service: &CopyService,
    actor: ActorId,
    views: &mut [ManagedSwarmServiceView],
) {
    stream::iter(views)
        .for_each_concurrent(4, |view| async move {
            view.copy_eligibility =
                Some(eligibility(service, actor, CopyKind::SwarmService, view.id).await);
        })
        .await;
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub enum ResourceCopyDataMode {
    CurrentData,
    ExistingBackup,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceCopyTargetsQuery {
    pub kind: CopyKind,
    pub source_id: Uuid,
}
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ResourceCopyTarget {
    pub id: Uuid,
    pub name: String,
}
#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCopyTargets {
    pub source_platform_id: Uuid,
    pub source_platform_name: String,
    pub source_node_id: Option<String>,
    pub swarm: bool,
    pub platforms: Vec<ResourceCopyTarget>,
}
