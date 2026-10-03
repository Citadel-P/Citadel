mod container_delta;
mod container_stats;
pub use container_delta::{ContainerDeltaResult, ContainerLifecycleState, ContainerStateDelta};
mod event_policy;
mod inventory_reconciliation;

pub use container_stats::{ContainerStatsBatch, sample_running_container_stats};
pub use event_policy::{
    ContainerChange, DeltaOutcome, ReconciliationDecision, ReconciliationScope, ResourceChange,
    RuntimeEventKind, SwarmResource, event_decision,
};
mod event_refresh;
pub use event_refresh::{
    EventRefresh, ResourceCollector, ResourceInventory, ResourceSnapshot, collect_event_scope,
};
pub use inventory_reconciliation::{InventoryCollectionTarget, collect_swarm_snapshot};

mod swarm_rollout;
pub use swarm_rollout::{rollout_complete_state, rollout_paused, stack_observed_status};

mod generation;
pub use generation::{ProjectionKind, ProjectionWrite, SnapshotGeneration};
mod resource_reconciliation;
pub use resource_reconciliation::{
    ContainerInventoryPort, ContainerReconciler, ImageInventoryPort, ImageReconciler,
};

mod recovery;
pub use recovery::{RecoveryReason, RuntimeRecoveryCoordinator};
pub use resource_reconciliation::{
    NetworkInventoryPort, NetworkReconciler, PlatformMetadataPort, PlatformReconciler,
    VolumeInventoryPort, VolumeReconciler,
};

mod projection_change;
pub use projection_change::ProjectionChange;

mod resource_delta;
pub use resource_delta::ResourceDelta;
