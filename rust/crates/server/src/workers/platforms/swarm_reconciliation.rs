//! Swarm projection scheduling is independent of standalone resource lanes and
//! active service operations. Pending + in-flight IDs share one bounded set.
use super::event_refresh::ScopedEventRequest;
use citadel_platforms::jobs::EventRefresh;
use citadel_runtime::runtime_metrics::RuntimeWork;
use futures_util::{StreamExt, stream::FuturesUnordered};
use std::{
    collections::BTreeMap,
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::Notify, time::Instant};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(crate) const DEBOUNCE: Duration = Duration::from_millis(1500);
const CONCURRENCY: usize = 4;

struct Pending {
    generation: u64,
    due: Option<Instant>,
    retry: bool,
}
struct State {
    pending: BTreeMap<Uuid, Pending>,
    closed: bool,
}
struct Shared {
    state: Mutex<State>,
    changed: Notify,
    capacity: usize,
}
#[derive(Clone)]
pub(crate) struct DirtySender(Arc<Shared>);
pub(crate) struct DirtyReceiver(Arc<Shared>);

pub(crate) fn channel(capacity: usize) -> (DirtySender, DirtyReceiver) {
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            pending: BTreeMap::new(),
            closed: false,
        }),
        changed: Notify::new(),
        capacity: capacity.max(1),
    });
    (DirtySender(shared.clone()), DirtyReceiver(shared))
}
impl DirtySender {
    /// Duplicates never occupy queue slots, even during a read or retry. A new
    /// Platform waits for capacity; cancellation and receiver closure release it.
    pub(crate) async fn mark(&self, id: Uuid, cancel: &CancellationToken) -> Result<(), ()> {
        loop {
            let changed = self.0.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let mut state = self.0.state.lock().unwrap();
                if state.closed || cancel.is_cancelled() {
                    return Err(());
                }
                if let Some(entry) = state.pending.get_mut(&id) {
                    entry.generation = entry.generation.wrapping_add(1);
                    return Ok(());
                }
                if state.pending.len() < self.0.capacity {
                    state.pending.insert(
                        id,
                        Pending {
                            generation: 0,
                            due: Some(Instant::now() + DEBOUNCE),
                            retry: false,
                        },
                    );
                    self.0.changed.notify_waiters();
                    return Ok(());
                }
            }
            tokio::select! {
                () = cancel.cancelled() => return Err(()),
                () = &mut changed => {},
            }
        }
    }
}
impl Drop for DirtyReceiver {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().unwrap();
        state.closed = true;
        state.pending.clear();
        self.0.changed.notify_waiters();
    }
}

enum Outcome {
    Committed,
    Superseded,
    Failed,
}

pub(crate) async fn run<S, E, C, CF, P, PF>(
    cancel: &CancellationToken,
    receiver: DirtyReceiver,
    retry: Duration,
    collect: &C,
    persist: &P,
) where
    E: std::fmt::Display,
    C: Fn(ScopedEventRequest) -> CF,
    CF: Future<Output = Result<Option<S>, E>>,
    P: Fn(ScopedEventRequest, S) -> PF,
    PF: Future<Output = Result<bool, E>>,
{
    let shared = &receiver.0;
    let mut tasks = FuturesUnordered::new();
    loop {
        let changed = shared.changed.notified();
        tokio::pin!(changed);
        changed.as_mut().enable();
        let next = {
            let mut state = shared.state.lock().unwrap();
            while tasks.len() < CONCURRENCY {
                let Some((&id, entry)) = state
                    .pending
                    .iter_mut()
                    .filter(|(_, entry)| entry.due.is_some_and(|due| due <= Instant::now()))
                    .min_by_key(|(_, entry)| entry.due)
                else {
                    break;
                };
                entry.due = None;
                if entry.retry {
                    RuntimeWork::SwarmRefreshRetry.units(1);
                }
                let generation = entry.generation;
                tasks.push(async move {
                    let request = ScopedEventRequest { platform_id: id, refresh: EventRefresh::Swarm };
                    let result = collect(request).await;
                    if shared.state.lock().unwrap().pending.get(&id).is_some_and(|entry| entry.generation != generation) {
                        return (id, generation, Outcome::Superseded);
                    }
                    let result = match result {
                        Ok(Some(snapshot)) => persist(request, snapshot).await,
                        Ok(None) => Ok(true),
                        Err(error) => Err(error),
                    };
                    let outcome = match result {
                        Ok(true) => Outcome::Committed,
                        Ok(false) => Outcome::Superseded,
                        Err(error) => {
                            RuntimeWork::EventSwarmRefresh.failures(1);
                            tracing::warn!(%error, platform_id=%id, "Swarm projection reconciliation failed; retaining dirty Platform");
                            Outcome::Failed
                        }
                    };
                    (id, generation, outcome)
                });
            }
            if tasks.len() < CONCURRENCY {
                state.pending.values().filter_map(|entry| entry.due).min()
            } else {
                None
            }
        };
        tokio::select! {
            biased;
            () = cancel.cancelled() => return,
            Some((id, generation, outcome)) = tasks.next(), if !tasks.is_empty() => {
                let mut state = shared.state.lock().unwrap();
                let entry = state.pending.get_mut(&id).expect("in-flight Platform retains its slot");
                if matches!(outcome, Outcome::Committed) && entry.generation == generation {
                    state.pending.remove(&id);
                    shared.changed.notify_waiters();
                } else {
                    entry.retry = matches!(outcome, Outcome::Failed);
                    if !entry.retry { RuntimeWork::SwarmRefreshDiscarded.units(1); }
                    entry.due = Some(Instant::now() + if entry.retry { retry.max(Duration::from_millis(250)) } else { DEBOUNCE });
                }
            },
            () = async { match next { Some(at) => tokio::time::sleep_until(at).await, None => std::future::pending().await } } => {},
            () = &mut changed => {},
        }
    }
}

/// Independently configured safety recovery marks only manager projections. The
/// target source is an in-memory registry (Local/Direct) or session state (Edge).
pub(crate) async fn safety_pass<T, TF>(
    cancel: &CancellationToken,
    interval: Duration,
    sender: DirtySender,
    targets: T,
) where
    T: Fn() -> TF,
    TF: Future<Output = Vec<Uuid>>,
{
    let interval = interval.max(Duration::from_secs(1));
    let mut timer = tokio::time::interval_at(Instant::now() + interval, interval);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! { biased; () = cancel.cancelled() => return, _ = timer.tick() => {} }
        let ids = tokio::select! { () = cancel.cancelled() => return, ids = targets() => ids };
        for id in ids {
            if sender.mark(id, cancel).await.is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests;
