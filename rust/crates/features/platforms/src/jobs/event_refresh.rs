//! Resource collection shared by event repair and recovery.
use super::{InventoryCollectionTarget, ReconciliationScope};
use crate::*;
use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventRefresh {
    Platform,
    Resource(ReconciliationScope),
    Swarm,
}

#[derive(Debug, Clone)]
pub enum ResourceInventory {
    Platform(RuntimePlatformInfo),
    Containers(Vec<RuntimeContainerSummary>),
    Images(Vec<RuntimeImageSummary>),
    Networks(Vec<RuntimeNetworkSummary>),
    Volumes(Vec<RuntimeVolumeSummary>),
    Swarm {
        inventory: RuntimeSwarmInventory,
        networks: Vec<RuntimeNetworkSummary>,
    },
}

#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    pub platform_id: Uuid,
    pub observed_at: DateTime<Utc>,
    pub inventory: ResourceInventory,
}

/// The dispatcher receives only the selected capability. Swarm intentionally
/// also receives networks to maintain task/service network relationships.
pub enum ResourceCollector<'a> {
    Platform(&'a dyn PlatformInfoPort),
    Containers(&'a dyn ContainerInventoryPort),
    Images(&'a dyn ImageInventoryPort),
    Networks(&'a dyn NetworkInventoryPort),
    Volumes(&'a dyn VolumeInventoryPort),
    Swarm {
        inventory: &'a dyn SwarmInventoryPort,
        networks: &'a dyn NetworkInventoryPort,
    },
}
impl<'a> ResourceCollector<'a> {
    /// Composition boundary for callers which own a complete adapter.
    pub fn for_scope(runtime: &'a dyn PlatformInventoryPort, scope: EventRefresh) -> Self {
        match scope {
            EventRefresh::Platform => Self::Platform(runtime),
            EventRefresh::Resource(ReconciliationScope::Containers) => Self::Containers(runtime),
            EventRefresh::Resource(ReconciliationScope::Images) => Self::Images(runtime),
            EventRefresh::Resource(ReconciliationScope::Networks) => Self::Networks(runtime),
            EventRefresh::Resource(ReconciliationScope::Volumes) => Self::Volumes(runtime),
            EventRefresh::Swarm => Self::Swarm {
                inventory: runtime,
                networks: runtime,
            },
        }
    }
}

pub async fn collect_event_scope(
    source: ResourceCollector<'_>,
    platform_id: Uuid,
    cancellation: &CancellationToken,
) -> Result<ResourceSnapshot, RuntimeCapabilityError> {
    let observed_at = Utc::now();
    let inventory = match source {
        ResourceCollector::Platform(runtime) => ResourceInventory::Platform(
            super::PlatformReconciler::collect(runtime, cancellation).await?,
        ),
        ResourceCollector::Containers(runtime) => ResourceInventory::Containers(
            super::ContainerReconciler::collect(runtime, cancellation).await?,
        ),
        ResourceCollector::Images(runtime) => {
            ResourceInventory::Images(super::ImageReconciler::collect(runtime, cancellation).await?)
        }
        ResourceCollector::Networks(runtime) => ResourceInventory::Networks(
            super::NetworkReconciler::collect(runtime, cancellation).await?,
        ),
        ResourceCollector::Volumes(runtime) => ResourceInventory::Volumes(
            super::VolumeReconciler::collect(runtime, cancellation).await?,
        ),
        ResourceCollector::Swarm {
            inventory,
            networks,
        } => {
            let target = InventoryCollectionTarget {
                platform_id,
                platform_type: PlatformKind::DockerSwarm,
            };
            let inventory = super::inventory_reconciliation::collect_swarm_inventory(
                inventory,
                &target,
                cancellation,
            )
            .await?
            .expect("Swarm target");
            ResourceInventory::Swarm {
                inventory,
                networks: super::NetworkReconciler::collect(networks, cancellation).await?,
            }
        }
    };
    Ok(ResourceSnapshot {
        platform_id,
        observed_at,
        inventory,
    })
}

impl ResourceInventory {
    pub fn projection_kind(&self) -> Option<super::ProjectionKind> {
        match self {
            Self::Platform(_) => Some(super::ProjectionKind::Platform),
            Self::Networks(_) => Some(super::ProjectionKind::Networks),
            Self::Volumes(_) => Some(super::ProjectionKind::Volumes),
            Self::Containers(_) => Some(super::ProjectionKind::Containers),
            Self::Images(_) => Some(super::ProjectionKind::Images),
            _ => None,
        }
    }
}
impl EventRefresh {
    pub fn projection_kind(self) -> Option<super::ProjectionKind> {
        match self {
            Self::Platform => Some(super::ProjectionKind::Platform),
            Self::Resource(ReconciliationScope::Networks) => Some(super::ProjectionKind::Networks),
            Self::Resource(ReconciliationScope::Volumes) => Some(super::ProjectionKind::Volumes),
            Self::Resource(ReconciliationScope::Containers) => {
                Some(super::ProjectionKind::Containers)
            }
            Self::Resource(ReconciliationScope::Images) => Some(super::ProjectionKind::Images),
            _ => None,
        }
    }
}
