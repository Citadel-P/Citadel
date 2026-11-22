use super::event_refresh::ScopedEventRequest;
use citadel_platforms::jobs::EventRefresh;
use std::{future::Future, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
pub(crate) const DEBOUNCE: Duration = Duration::from_millis(1500);
const CONCURRENCY: usize = 4;
pub(crate) type DirtySender = citadel_runtime::dirty_queue::DirtySender<Uuid>;
pub(crate) type DirtyReceiver = citadel_runtime::dirty_queue::DirtyReceiver<Uuid>;
pub(crate) fn channel(capacity: usize) -> (DirtySender, DirtyReceiver) {
    citadel_runtime::dirty_queue::channel(capacity, DEBOUNCE)
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
    super::scoped_reconciler::run_scope(
        cancel,
        receiver,
        EventRefresh::Swarm,
        CONCURRENCY,
        retry,
        collect,
        persist,
    )
    .await;
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
    let mut timer = tokio::time::interval_at(tokio::time::Instant::now() + interval, interval);
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
