//! Connect committed projections to live commands in the same database/process.
use super::super::runtime_index::{Database, database};
use citadel_platforms::{
    containers::{ContainerOperationCoordinator, ContainerTarget},
    jobs::{ContainerDeltaResult, ContainerStateDelta},
};
use sqlx::PgPool;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock, Weak},
};
use uuid::Uuid;

static COORDINATORS: OnceLock<Mutex<BTreeMap<Database, Weak<ContainerOperationCoordinator>>>> =
    OnceLock::new();
pub(super) fn attach(pool: &PgPool) -> Arc<ContainerOperationCoordinator> {
    let mut entries = COORDINATORS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(coordinator) = entries.get(&database(pool)).and_then(Weak::upgrade) {
        return coordinator;
    }
    entries.retain(|_, entry| entry.strong_count() > 0);
    let coordinator = Arc::new(ContainerOperationCoordinator::default());
    entries.insert(database(pool), Arc::downgrade(&coordinator));
    coordinator
}
pub(crate) fn committed(
    pool: &PgPool,
    operation: Uuid,
    target: &ContainerTarget,
    state: Option<&str>,
    observed: i64,
) {
    let coordinator = COORDINATORS.get().and_then(|entries| {
        entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&database(pool))
            .and_then(Weak::upgrade)
    });
    if let Some(coordinator) = coordinator {
        coordinator.committed(operation, target, state, observed);
    }
}
pub(crate) fn state_batch(
    pool: &PgPool,
    platform: Uuid,
    node: Option<&str>,
    deltas: &[ContainerStateDelta],
    results: &[ContainerDeltaResult],
) {
    for (delta, result) in deltas.iter().zip(results) {
        if result.observed
            && result.defer_parent_effects
            && let (Some(operation), Some(id)) = (result.operation_id, result.container_id)
        {
            committed(
                pool,
                operation,
                &ContainerTarget {
                    id,
                    platform_id: platform,
                    node_id: node.map(str::to_owned),
                    docker_id: delta.docker_id.clone(),
                },
                Some(delta.state.as_str()),
                delta.observed_at_millis,
            );
        }
    }
}
