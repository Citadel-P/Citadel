use super::*;
use citadel_platforms::jobs::{ReconciliationScope, collect_event_scope};
use citadel_platforms::jobs::{ResourceSnapshot, SnapshotGeneration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ScopedEventRequest {
    pub platform_id: uuid::Uuid,
    pub refresh: EventRefresh,
}

pub(super) struct EventRefreshWorker {
    pub targets: Arc<PlatformRuntimeRegistry>,
    pub refreshes: EventRefreshSender,
    pub docker: DockerClient,
    pub pool: PgPool,
    pub realtime: Option<RealtimeHub>,
    pub node_policy: citadel_platforms::node_agents::NodeAgentReconciliationPolicy,
    pub retry_delay: Duration,
    pub swarm_interval: Duration,
}

/// Every resource has an independent bounded scheduling lane.
pub(super) async fn event_resource_refresh(
    cancellation: CancellationToken,
    receivers: EventRefreshReceivers,
    worker: EventRefreshWorker,
) -> Result<(), std::convert::Infallible> {
    let store = PostgresInventoryProjectionStore::new(worker.pool.clone())
        .with_node_policy(worker.node_policy.clone());
    let refresh = run_refresh_workers(
        &cancellation,
        receivers,
        worker.retry_delay,
        |request| refresh_event_scope(&worker, &store, request, &cancellation),
        |request, observation: ScopedObservation| {
            let store = &store;
            let worker = &worker;
            let cancellation = &cancellation;
            async move {
                let accepted = store
                    .persist_resource_committed(
                        &observation.snapshot,
                        observation.generation.as_ref(),
                    )
                    .await?;
                drop(observation._permit);
                if accepted.accepted() && request.refresh == EventRefresh::Platform {
                    let targets = worker.targets.snapshot().await;
                    if let Some(target) = targets.iter().find(|t| t.id == request.platform_id) {
                        for refresh in
                            citadel_platforms::jobs::RuntimeRecoveryCoordinator::platform_committed(
                                target.platform_type,
                            )
                        {
                            let _ = worker
                                .refreshes
                                .send(
                                    ScopedEventRequest {
                                        platform_id: request.platform_id,
                                        refresh,
                                    },
                                    cancellation,
                                )
                                .await;
                        }
                    }
                }
                if accepted.changed()
                    && let Some(hub) = &worker.realtime
                {
                    hub.publish_resource_snapshot(&observation.snapshot, None);
                }
                Ok::<_, RuntimeCapabilityError>(accepted.accepted())
            }
        },
    );
    let safety = super::swarm_reconciliation::safety_pass(
        &cancellation,
        worker.swarm_interval,
        worker.refreshes.swarm(),
        || async {
            worker
                .targets
                .snapshot()
                .await
                .iter()
                .filter(|target| {
                    target.platform_type == citadel_platforms::PlatformKind::DockerSwarm
                        && matches!(
                            target.connector_type,
                            citadel_platforms::ConnectorKind::Local
                                | citadel_platforms::ConnectorKind::Agent
                        )
                })
                .map(|target| target.id)
                .collect()
        },
    );
    tokio::join!(refresh, safety);
    Ok(())
}

pub(crate) struct ScopedObservation {
    pub snapshot: ResourceSnapshot,
    pub generation: Option<SnapshotGeneration>,
    pub _permit: citadel_runtime::IoPermit,
}

async fn refresh_event_scope(
    worker: &EventRefreshWorker,
    store: &PostgresInventoryProjectionStore,
    request: ScopedEventRequest,
    cancellation: &CancellationToken,
) -> Result<Option<ScopedObservation>, RuntimeCapabilityError> {
    let targets = worker.targets.snapshot().await;
    let Some(target) = targets
        .iter()
        .find(|target| target.id == request.platform_id)
    else {
        return Ok(None);
    };
    if request.refresh == EventRefresh::Swarm
        && target.platform_type != citadel_platforms::PlatformKind::DockerSwarm
    {
        return Ok(None);
    }
    let runtime: &dyn PlatformInventoryPort = match target.connector_type {
        citadel_platforms::ConnectorKind::Local => &worker.docker,
        citadel_platforms::ConnectorKind::Agent => {
            let Some(agent) = target.agent.as_ref() else {
                return Ok(None);
            };
            agent.as_ref()
        }
        _ => return Ok(None),
    };
    let budget = worker
        .targets
        .inventory_budget
        .measured_as(refresh_metric(request.refresh));
    let Some(permit) = budget.enter(cancellation).await else {
        return Ok(None);
    };
    let generation = match request.refresh.projection_kind() {
        Some(kind) => Some(SnapshotGeneration::capture(target.id, None, kind).await),
        None => None,
    };
    let started = chrono::Utc::now();
    let snapshot = match collect_event_scope(
        citadel_platforms::jobs::ResourceCollector::for_scope(runtime, request.refresh),
        target.id,
        cancellation,
    )
    .await
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            if request.refresh == EventRefresh::Swarm && !cancellation.is_cancelled() {
                store.mark_swarm_stale(target.id, started).await?;
            }
            return Err(error);
        }
    };
    Ok(Some(ScopedObservation {
        snapshot,
        generation,
        _permit: permit,
    }))
}

pub(crate) fn refresh_metric(refresh: EventRefresh) -> RuntimeWork {
    match refresh {
        EventRefresh::Platform => RuntimeWork::EventPlatformRefresh,
        EventRefresh::Resource(ReconciliationScope::Containers) => {
            RuntimeWork::EventContainersRefresh
        }
        EventRefresh::Resource(ReconciliationScope::Images) => RuntimeWork::EventImagesRefresh,
        EventRefresh::Resource(ReconciliationScope::Networks) => RuntimeWork::EventNetworksRefresh,
        EventRefresh::Resource(ReconciliationScope::Volumes) => RuntimeWork::EventVolumesRefresh,
        EventRefresh::Swarm => RuntimeWork::EventSwarmRefresh,
    }
}
