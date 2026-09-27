//! Bounded lifecycle windows shared by Local, Direct and Edge consumers.
use futures_util::{Stream, StreamExt};
use std::{collections::BTreeMap, pin::Pin, time::Duration};
use tokio_util::sync::CancellationToken;

pub(super) const MAX_EVENTS: usize = 256;
// Leave headroom inside the 150 ms command confirmation deadline, while
// combining Docker completions that arrive tens of milliseconds apart.
const WINDOW: Duration = Duration::from_millis(75);

/// Never read past a barrier: return it to the caller before consuming more.
/// Count raw events, including duplicates, so a hot identity cannot starve others.
pub(super) async fn gather<T, S: Stream<Item = T> + ?Sized>(
    first: T,
    mut stream: Pin<&mut S>,
    compatible: impl Fn(&T, &T) -> bool,
    cancellation: &CancellationToken,
) -> Option<(Vec<T>, Option<T>)> {
    let deadline = tokio::time::sleep(WINDOW);
    tokio::pin!(deadline);
    let mut batch = vec![first];
    while batch.len() < MAX_EVENTS {
        let next = tokio::select! {
            biased;
            () = cancellation.cancelled() => return None,
            () = &mut deadline => break,
            next = stream.next() => next,
        };
        let Some(next) = next else { break };
        if !compatible(&batch[0], &next) {
            return Some((batch, Some(next)));
        }
        batch.push(next);
    }
    Some((batch, None))
}

/// Latest observation wins; arrival order breaks equal timestamp ties.
pub(super) fn coalesce<T, K: Ord, Stamp: Ord>(
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

pub(super) fn publish(
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

pub(super) async fn refresh(
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
#[path = "container_batch_tests.rs"]
mod tests;
