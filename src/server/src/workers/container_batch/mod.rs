//! Bounded lifecycle windows shared by Local, Direct and Edge consumers.
pub(crate) use citadel_runtime::event_batch::gather;
#[cfg(test)]
use citadel_runtime::event_batch::{MAX_EVENTS, WINDOW};
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;
/// Latest observation wins; arrival order breaks equal timestamp ties.
pub(crate) fn coalesce<T, K: Ord, Stamp: Ord>(
    events: Vec<T>,
    key: impl Fn(&T) -> K,
    stamp: impl Fn(&T) -> Stamp,
) -> Vec<T> {
    let mut latest = BTreeMap::<K, T>::new();
    for event in events {
        let key = key(&event);
        if latest
            .get(&key)
            .is_none_or(|old| stamp(old) <= stamp(&event))
        {
            latest.insert(key, event);
        }
    }
    latest.into_values().collect()
}

pub(crate) fn publish(
    hub: Option<&crate::realtime::RealtimeHub>,
    platform: uuid::Uuid,
    results: &[citadel_platforms::jobs::ContainerDeltaResult],
) {
    if let Some(hub) = hub {
        let patches: Vec<_> = results
            .iter()
            .filter_map(|result| result.patch.as_ref())
            .cloned()
            .collect();
        hub.publish_container_state_patches(platform, &patches);
        let deployments: Vec<_> = results
            .iter()
            .filter(|r| r.changed && !r.defer_parent_effects)
            .filter_map(|r| r.deployment_id)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let stacks: Vec<_> = results
            .iter()
            .filter(|r| r.changed && !r.defer_parent_effects)
            .filter_map(|r| r.stack_id)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        hub.publish_scoped_resource_changes(platform, "Deployment", &deployments);
        hub.publish_scoped_resource_changes(platform, "Stack", &stacks);
    }
}

pub(crate) async fn refresh(
    platform: uuid::Uuid,
    swarm: bool,
    unavailable: bool,
    sender: &super::platforms::scoped_reconciler::EventRefreshSender,
    cancellation: &CancellationToken,
) {
    use citadel_platforms::jobs::{EventRefresh, ReconciliationScope};
    for refresh in [
        unavailable.then_some(EventRefresh::Resource(ReconciliationScope::Containers)),
        swarm.then_some(EventRefresh::Swarm),
    ]
    .into_iter()
    .flatten()
    {
        if sender
            .send(
                super::platforms::event_refresh::ScopedEventRequest {
                    platform_id: platform,
                    refresh,
                },
                cancellation,
            )
            .await
            .is_err()
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests;
