//! Independent bounded resource scheduling. Collection may run while requests
//! arrive; a newer dirty generation discards that read and retains one follow-up.
use super::*;
use std::{
    collections::BTreeMap,
    future::Future,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::time::Instant;

#[derive(Debug, Clone, Copy)]
enum ReconcileReason {
    Event,
    Retry,
    Superseded,
}

#[derive(Clone, Copy)]
struct Pending {
    at: Instant,
    reason: ReconcileReason,
}
impl Pending {
    fn after(delay: Duration, reason: ReconcileReason) -> Self {
        Self {
            at: Instant::now() + delay,
            reason,
        }
    }
}

type DirtyReads = Arc<Mutex<BTreeMap<ScopedEventRequest, Arc<AtomicU64>>>>;

#[derive(Clone)]
pub(crate) struct EventRefreshSender {
    dirty: DirtyReads,
    containers: BoundedSender<ScopedEventRequest>,
    images: BoundedSender<ScopedEventRequest>,
    platform: BoundedSender<ScopedEventRequest>,
    networks: BoundedSender<ScopedEventRequest>,
    volumes: BoundedSender<ScopedEventRequest>,
    swarm: super::swarm_reconciliation::DirtySender,
}
pub(crate) struct EventRefreshReceivers {
    dirty: DirtyReads,
    containers: BoundedReceiver<ScopedEventRequest>,
    images: BoundedReceiver<ScopedEventRequest>,
    platform: BoundedReceiver<ScopedEventRequest>,
    networks: BoundedReceiver<ScopedEventRequest>,
    volumes: BoundedReceiver<ScopedEventRequest>,
    swarm: super::swarm_reconciliation::DirtyReceiver,
}

pub(crate) fn event_refresh_channels(
    capacity: usize,
) -> (EventRefreshSender, EventRefreshReceivers) {
    let dirty = DirtyReads::default();
    let (containers, c) = bounded_channel(capacity, QueueOverflowPolicy::Wait);
    let (images, i) = bounded_channel(capacity, QueueOverflowPolicy::Wait);
    let (platform, platform_rx) = bounded_channel(capacity, QueueOverflowPolicy::Wait);
    let (networks, networks_rx) = bounded_channel(capacity, QueueOverflowPolicy::Wait);
    let (volumes, volumes_rx) = bounded_channel(capacity, QueueOverflowPolicy::Wait);
    let (swarm, swarm_rx) = super::swarm_reconciliation::channel(capacity);

    (
        EventRefreshSender {
            dirty: dirty.clone(),
            containers,
            images,
            platform,
            networks,
            volumes,
            swarm,
        },
        EventRefreshReceivers {
            dirty,
            containers: c,
            images: i,
            platform: platform_rx,
            networks: networks_rx,
            volumes: volumes_rx,
            swarm: swarm_rx,
        },
    )
}
impl EventRefreshSender {
    pub(crate) fn swarm(&self) -> super::swarm_reconciliation::DirtySender {
        self.swarm.clone()
    }

    pub(crate) async fn send(
        &self,
        request: ScopedEventRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), citadel_runtime::QueueSendError<ScopedEventRequest>> {
        if request.refresh == EventRefresh::Swarm {
            return self
                .swarm
                .mark(request.platform_id, cancellation)
                .await
                .map_err(|()| {
                    if cancellation.is_cancelled() {
                        citadel_runtime::QueueSendError::Cancelled(request)
                    } else {
                        citadel_runtime::QueueSendError::Closed(request)
                    }
                });
        }
        use citadel_platforms::jobs::ProjectionKind;
        let sender = match request.refresh.projection_kind() {
            Some(ProjectionKind::Containers) => &self.containers,
            Some(ProjectionKind::Images) => &self.images,
            Some(ProjectionKind::Platform) => &self.platform,
            Some(ProjectionKind::Networks) => &self.networks,
            Some(ProjectionKind::Volumes) => &self.volumes,
            None => unreachable!("Swarm uses its dedicated coordinator"),
        };
        // Invalidate even when the bounded queue must wait. At most one active
        // read per lane lives here; no per-platform cache survives completion.
        if let Some(generation) = self.dirty.lock().unwrap().get(&request) {
            generation.fetch_add(1, Ordering::Relaxed);
        }
        sender.send(request, cancellation).await
    }
}

pub(crate) async fn run_refresh_workers<S, E, C, CF, P, PF>(
    cancellation: &CancellationToken,
    receivers: EventRefreshReceivers,
    retry: Duration,
    collect: C,
    persist: P,
) where
    E: std::fmt::Display,
    C: Fn(ScopedEventRequest) -> CF,
    CF: Future<Output = Result<Option<S>, E>>,
    P: Fn(ScopedEventRequest, S) -> PF,
    PF: Future<Output = Result<bool, E>>,
{
    let dirty = &receivers.dirty;
    tokio::join!(
        run_scope(
            cancellation,
            dirty,
            receivers.containers,
            retry,
            &collect,
            &persist
        ),
        run_scope(
            cancellation,
            dirty,
            receivers.images,
            retry,
            &collect,
            &persist
        ),
        run_scope(
            cancellation,
            dirty,
            receivers.platform,
            retry,
            &collect,
            &persist
        ),
        run_scope(
            cancellation,
            dirty,
            receivers.networks,
            retry,
            &collect,
            &persist
        ),
        run_scope(
            cancellation,
            dirty,
            receivers.volumes,
            retry,
            &collect,
            &persist
        ),
        super::swarm_reconciliation::run(cancellation, receivers.swarm, retry, &collect, &persist),
    );
}

async fn run_scope<S, E, C, CF, P, PF>(
    cancel: &CancellationToken,
    dirty: &DirtyReads,
    mut receiver: BoundedReceiver<ScopedEventRequest>,
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
    let debounce = Duration::from_millis(500);
    let mut pending = BTreeMap::<ScopedEventRequest, Pending>::new();
    loop {
        if pending.is_empty() {
            let Some(request) = receiver.recv(cancel).await else {
                return;
            };
            pending.insert(request, Pending::after(debounce, ReconcileReason::Event));
        }
        let (&request, &next) = pending.iter().min_by_key(|(_, work)| work.at).unwrap();
        tokio::select! {
            biased;
            () = cancel.cancelled() => return,
            () = tokio::time::sleep_until(next.at) => {},
            incoming = receiver.recv(cancel), if pending.len() < 256 => {
                let Some(incoming) = incoming else { return; };
                pending.entry(incoming).or_insert_with(|| Pending::after(debounce, ReconcileReason::Event));
                continue;
            }
        }
        pending.remove(&request);
        tracing::debug!(platform_id=%request.platform_id, scope=?request.refresh, reason=?next.reason, "Starting resource reconciliation");
        if matches!(next.reason, ReconcileReason::Retry) {
            outcome_metric(request.refresh, true).units(1);
        }

        let generation = Arc::new(AtomicU64::new(0));
        dirty.lock().unwrap().insert(request, generation.clone());
        let read = collect(request);
        tokio::pin!(read);
        let result = loop {
            tokio::select! {
                () = cancel.cancelled() => return,
                incoming = receiver.recv(cancel), if pending.len() < 256 => {
                    let Some(incoming) = incoming else { return; };
                    pending.entry(incoming).or_insert_with(|| Pending::after(debounce, ReconcileReason::Event));
                },
                result = &mut read => break result,
            }
        };
        // Requests received during I/O advance this scope's dirty generation.
        // Never clear that pending generation when the older read completes.
        dirty.lock().unwrap().remove(&request);
        if generation.load(Ordering::Relaxed) != 0 || pending.contains_key(&request) {
            pending
                .entry(request)
                .or_insert_with(|| Pending::after(debounce, ReconcileReason::Event));
            discarded(request.refresh);
            continue;
        }
        let result = match result {
            Ok(Some(snapshot)) => tokio::select! {
                () = cancel.cancelled() => return,
                result = persist(request, snapshot) => result,
            },
            Ok(None) => Ok(true),
            Err(error) => Err(error),
        };
        let (delay, reason) = match result {
            Ok(true) => continue,
            Ok(false) => {
                discarded(request.refresh);
                (debounce, ReconcileReason::Superseded)
            }
            Err(error) => {
                refresh_metric(request.refresh).failures(1);
                tracing::warn!(%error, platform_id=%request.platform_id, scope=?request.refresh, "Resource reconciliation failed; retaining only this scope");
                (
                    retry.max(Duration::from_millis(250)),
                    ReconcileReason::Retry,
                )
            }
        };
        pending
            .entry(request)
            .or_insert_with(|| Pending::after(delay, reason));
    }
}

fn discarded(refresh: EventRefresh) {
    outcome_metric(refresh, false).units(1);
}

fn outcome_metric(refresh: EventRefresh, retry: bool) -> RuntimeWork {
    use citadel_platforms::jobs::ReconciliationScope;
    match (refresh, retry) {
        (EventRefresh::Platform, true) => RuntimeWork::PlatformRefreshRetry,
        (EventRefresh::Platform, false) => RuntimeWork::PlatformRefreshDiscarded,
        (EventRefresh::Resource(ReconciliationScope::Containers), true) => {
            RuntimeWork::ContainerRefreshRetry
        }
        (EventRefresh::Resource(ReconciliationScope::Containers), false) => {
            RuntimeWork::ContainerRefreshDiscarded
        }
        (EventRefresh::Resource(ReconciliationScope::Images), true) => {
            RuntimeWork::ImageRefreshRetry
        }
        (EventRefresh::Resource(ReconciliationScope::Images), false) => {
            RuntimeWork::ImageRefreshDiscarded
        }
        (EventRefresh::Resource(ReconciliationScope::Networks), true) => {
            RuntimeWork::NetworkRefreshRetry
        }
        (EventRefresh::Resource(ReconciliationScope::Networks), false) => {
            RuntimeWork::NetworkRefreshDiscarded
        }
        (EventRefresh::Resource(ReconciliationScope::Volumes), true) => {
            RuntimeWork::VolumeRefreshRetry
        }
        (EventRefresh::Resource(ReconciliationScope::Volumes), false) => {
            RuntimeWork::VolumeRefreshDiscarded
        }
        (EventRefresh::Swarm, true) => RuntimeWork::SwarmRefreshRetry,
        (EventRefresh::Swarm, false) => RuntimeWork::SwarmRefreshDiscarded,
    }
}

#[cfg(test)]
mod tests;
