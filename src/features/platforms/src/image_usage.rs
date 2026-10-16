//! Image dependencies are scoped to a daemon. Saved tags are references, not running content.
use crate::{RuntimeCapabilityError, RuntimeErrorKind};
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageLocation {
    pub docker_image_id: String,
    pub docker_node_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ImageRelationship {
    Container,
    SavedLocalImage,
    SavedDigest,
    SavedTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ImageWorkloadKind {
    Deployment,
    Stack,
    SwarmService,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUsageEntry {
    pub relationship: ImageRelationship,
    pub workload_kind: Option<ImageWorkloadKind>,
    pub workload_id: Option<Uuid>,
    pub workload_name: Option<String>,
    pub workload_status: Option<String>,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub container_state: Option<String>,
    pub reference: Option<String>,
    pub observed_at: Option<DateTime<Utc>>,
    pub is_stale: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUsage {
    pub entries: Vec<ImageUsageEntry>,
    pub next_offset: Option<u32>,
    pub coverage_complete: bool,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImageDeletionSnapshot {
    pub docker_image_id: String,
    pub row_version: i64,
    pub tags: Vec<String>,
    pub has_local_dependency: bool,
    pub processing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDeletionReview {
    pub docker_image_id: String,
    pub row_version: i64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDeletionPreviewItem {
    pub review: ImageDeletionReview,
    pub can_delete: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub usage: ImageUsage,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUsageSummary {
    pub containers: u32,
    pub deployments: u32,
    pub stacks: u32,
    pub swarm_services: u32,
    pub is_partial: bool,
}

pub trait ImageUsageStore: Send + Sync {
    fn summaries<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        images: &'a [crate::ImageDetails],
    ) -> BoxFuture<'a, Result<Vec<ImageUsageSummary>, RuntimeCapabilityError>>;

    fn deletion_snapshots<'a>(
        &'a self,
        platform: Uuid,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<ImageDeletionSnapshot>, RuntimeCapabilityError>>;

    fn usage<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        location: &'a ImageLocation,
        offset: u32,
    ) -> BoxFuture<'a, Result<ImageUsage, RuntimeCapabilityError>>;
}

/// Only explicit review reads Docker. Ordinary list/usage reads stay on projections.
pub async fn preview_deletion(
    store: &dyn crate::image_mutations::ImageMutationStore,
    runtime: &dyn crate::runtime_provider::PlatformRuntimeProvider,
    actor: ActorId,
    administrator: bool,
    platform: Uuid,
    ids: &[String],
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<Vec<ImageDeletionPreviewItem>, RuntimeCapabilityError> {
    let snapshots = store.deletion_snapshots(platform, ids).await?;
    if snapshots.len() != ids.len() {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::NotFound,
            "One or more images were not found. Refresh inventory.",
            false,
        ));
    }
    let runtime = runtime.images(platform, None, cancel).await?;
    let mut items = Vec::new();
    for snapshot in snapshots {
        let mut blockers = Vec::new();
        let mut tags = snapshot.tags;
        tags.sort();
        if snapshot.has_local_dependency {
            blockers.push("A saved Local image dependency prevents deletion.".into());
        }
        if snapshot.processing {
            blockers.push("An image operation is already in progress.".into());
        }
        match runtime.inspect_image(&snapshot.docker_image_id, cancel).await {
            Ok(image) => {
                let mut observed_tags = image.repo_tags;
                observed_tags.sort();
                if image.id != snapshot.docker_image_id || observed_tags != tags {
                    blockers.push("Image identity or tags changed. Refresh inventory before reviewing again.".into());
                }
                if !image.containers.is_empty() {
                    blockers.push("A container references this image, including possibly a stopped container.".into());
                }
            }
            Err(_) => blockers.push("Live image identity and usage could not be verified. Refresh inventory and retry when the host is available.".into()),
        }
        let usage = store
            .usage(
                actor,
                administrator,
                platform,
                &ImageLocation {
                    docker_image_id: snapshot.docker_image_id.clone(),
                    docker_node_id: None,
                },
                0,
            )
            .await?;
        let mut warnings = vec!["Removing local content does not remove the registry artifact. Saved registry references may need to pull it again.".into()];
        if tags.len() > 1 {
            warnings.push("This image has multiple tags. Docker may refuse non-forced deletion; no tags will be removed automatically to resolve that conflict.".into());
        }
        items.push(ImageDeletionPreviewItem {
            review: ImageDeletionReview {
                docker_image_id: snapshot.docker_image_id,
                row_version: snapshot.row_version,
                tags,
            },
            can_delete: blockers.is_empty(),
            blockers,
            warnings,
            usage,
            observed_at: Utc::now(),
        });
    }
    Ok(items)
}
