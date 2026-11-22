mod lane;
pub(super) use lane::run_scope;

use super::*;
use citadel_runtime::dirty_queue::{DirtyReceiver, DirtySender, channel};
use std::future::Future;
const DEBOUNCE: Duration = Duration::from_millis(500);
#[derive(Clone)]
pub(crate) struct EventRefreshSender {
    containers: DirtySender<uuid::Uuid>,
    images: DirtySender<uuid::Uuid>,
    platform: DirtySender<uuid::Uuid>,
    networks: DirtySender<uuid::Uuid>,
    volumes: DirtySender<uuid::Uuid>,
    swarm: super::swarm_reconciliation::DirtySender,
}
pub(crate) struct EventRefreshReceivers {
    containers: DirtyReceiver<uuid::Uuid>,
    images: DirtyReceiver<uuid::Uuid>,
    platform: DirtyReceiver<uuid::Uuid>,
    networks: DirtyReceiver<uuid::Uuid>,
    volumes: DirtyReceiver<uuid::Uuid>,
    swarm: super::swarm_reconciliation::DirtyReceiver,
}

pub(crate) fn event_refresh_channels(
    capacity: usize,
) -> (EventRefreshSender, EventRefreshReceivers) {
    let (containers, c) = channel(capacity, DEBOUNCE);
    let (images, i) = channel(capacity, DEBOUNCE);
    let (platform, platform_rx) = channel(capacity, DEBOUNCE);
    let (networks, networks_rx) = channel(capacity, DEBOUNCE);
    let (volumes, volumes_rx) = channel(capacity, DEBOUNCE);
    let (swarm, swarm_rx) = super::swarm_reconciliation::channel(capacity);

    (
        EventRefreshSender {
            containers,
            images,
            platform,
            networks,
            volumes,
            swarm,
        },
        EventRefreshReceivers {
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
        use citadel_platforms::jobs::ProjectionKind;
        let sender = match request.refresh.projection_kind() {
            Some(ProjectionKind::Containers) => &self.containers,
            Some(ProjectionKind::Images) => &self.images,
            Some(ProjectionKind::Platform) => &self.platform,
            Some(ProjectionKind::Networks) => &self.networks,
            Some(ProjectionKind::Volumes) => &self.volumes,
            None => &self.swarm,
        };
        sender
            .mark(request.platform_id, cancellation)
            .await
            .map_err(|error| match error {
                citadel_runtime::QueueSendError::Cancelled(_) => {
                    citadel_runtime::QueueSendError::Cancelled(request)
                }
                _ => citadel_runtime::QueueSendError::Closed(request),
            })
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
    if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
        job.waiting("Awaiting resource changes or reconciliation retries", None);
    }
    use citadel_platforms::jobs::ReconciliationScope::*;
    tokio::join!(
        run_scope(
            cancellation,
            receivers.containers,
            EventRefresh::Resource(Containers),
            2,
            retry,
            &collect,
            &persist
        ),
        run_scope(
            cancellation,
            receivers.images,
            EventRefresh::Resource(Images),
            2,
            retry,
            &collect,
            &persist
        ),
        run_scope(
            cancellation,
            receivers.platform,
            EventRefresh::Platform,
            2,
            retry,
            &collect,
            &persist
        ),
        run_scope(
            cancellation,
            receivers.networks,
            EventRefresh::Resource(Networks),
            2,
            retry,
            &collect,
            &persist
        ),
        run_scope(
            cancellation,
            receivers.volumes,
            EventRefresh::Resource(Volumes),
            2,
            retry,
            &collect,
            &persist
        ),
        super::swarm_reconciliation::run(cancellation, receivers.swarm, retry, &collect, &persist),
    );
}
#[cfg(test)]
mod tests;
