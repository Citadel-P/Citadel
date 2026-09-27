use super::*;
use crate::workers::platforms::{
    event_refresh::{ScopedObservation, refresh_metric},
    scoped_reconciler::{event_refresh_channels, run_refresh_workers},
};
use citadel_platforms::jobs::{SnapshotGeneration, collect_event_scope};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub(super) async fn observe(
    session: Arc<EdgeSession>,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    scans: IoBudget,
    settings: InventorySettings,
    cancellation: &CancellationToken,
) -> Result<(), Error> {
    let (sender, receivers) = event_refresh_channels(256);
    let runtime = EdgeRuntime {
        session: session.clone(),
    };
    let store = PostgresEdgeStore::new(pool.clone()).with_node_policy(settings.node_policy.clone());
    // Session-local completion state is bounded to five resource kinds. A failed
    // scope retains its own retry and cannot declare the entire node fresh.
    let completion = tokio::sync::Mutex::new(
        None::<(
            chrono::DateTime<chrono::Utc>,
            std::collections::BTreeSet<citadel_platforms::jobs::ProjectionKind>,
        )>,
    );
    let manager = std::sync::atomic::AtomicBool::new(false);
    let scopes = run_refresh_workers(
        cancellation,
        receivers,
        Duration::from_secs(5),
        |request| {
            let runtime = &runtime;
            let scans = &scans;
            let session = &session;
            let pool = &pool;
            async move {
                let budget = scans.measured_as(refresh_metric(request.refresh));
                let Some(permit) = budget.enter(cancellation).await else {
                    return Ok(None);
                };
                let generation = match request.refresh.projection_kind() {
                    Some(kind) => Some(
                        SnapshotGeneration::capture(
                            request.platform_id,
                            session.target.node_id.as_deref(),
                            kind,
                        )
                        .await,
                    ),
                    None => None,
                };
                let started = chrono::Utc::now();
                let snapshot = match collect_event_scope(
                    citadel_platforms::jobs::ResourceCollector::for_scope(runtime, request.refresh),
                    request.platform_id,
                    cancellation,
                )
                .await
                {
                    Ok(snapshot) => snapshot,
                    Err(error) => {
                        if request.refresh == EventRefresh::Swarm && !cancellation.is_cancelled() {
                            citadel_adapters::persistence::postgres::platforms::inventory::store::PostgresInventoryProjectionStore::new(pool.clone())
                                .mark_swarm_stale(request.platform_id, started).await?;
                        }
                        return Err(Error::from(error));
                    }
                };
                Ok::<_, Error>(Some(ScopedObservation {
                    snapshot,
                    generation,
                    _permit: permit,
                }))
            }
        },
        |request, observation: ScopedObservation| {
            let store = &store;
            let session = &session;
            let realtime = &realtime;
            let sender = &sender;
            let completion = &completion;
            let manager = &manager;
            async move {
                let accepted = store
                    .persist_resource_committed(
                        session,
                        &observation.snapshot,
                        observation.generation.as_ref(),
                    )
                    .await?;
                drop(observation._permit);
                let mut changed = accepted.changed();
                if accepted.accepted() && session.target.node_id.is_some() {
                    let mut state = completion.lock().await;
                    if request.refresh == EventRefresh::Platform {
                        *state = Some((observation.snapshot.observed_at, Default::default()));
                    } else if let Some((started, scopes)) = state.as_mut()
                        && observation.snapshot.observed_at >= *started
                        && let Some(kind) = request.refresh.projection_kind()
                    {
                        scopes.insert(kind);
                        if scopes.len() == 4 {
                            // Completing recovery changes visible freshness even if the
                            // last resource observation was semantically unchanged.
                            changed |= store
                                .complete_resource_recovery(session, *started)
                                .await?
                                .changed();
                            *state = None;
                        }
                    }
                }

                if accepted.accepted()
                    && let citadel_platforms::jobs::ResourceInventory::Platform(info) =
                        &observation.snapshot.inventory
                {
                    let kind = if session.target.node_id.is_none()
                        && info.swarm.as_ref().is_some_and(|s| s.control_available)
                    {
                        citadel_platforms::PlatformKind::DockerSwarm
                    } else {
                        citadel_platforms::PlatformKind::Docker
                    };
                    manager.store(
                        kind == citadel_platforms::PlatformKind::DockerSwarm,
                        std::sync::atomic::Ordering::Release,
                    );
                    for refresh in
                        citadel_platforms::jobs::RuntimeRecoveryCoordinator::platform_committed(
                            kind,
                        )
                    {
                        let _ = sender
                            .send(
                                crate::workers::platforms::event_refresh::ScopedEventRequest {
                                    platform_id: request.platform_id,
                                    refresh,
                                },
                                cancellation,
                            )
                            .await;
                    }
                }
                if changed && let Some(hub) = realtime {
                    hub.publish_resource_snapshot(
                        &observation.snapshot,
                        session.target.node_id.as_deref(),
                    );
                }
                Ok::<_, Error>(accepted.accepted())
            }
        },
    );
    let safety = crate::workers::platforms::swarm_reconciliation::safety_pass(
        cancellation,
        settings.reconciliation_interval,
        sender.swarm(),
        || async {
            if manager.load(std::sync::atomic::Ordering::Acquire) {
                vec![session.target.platform_id]
            } else {
                vec![]
            }
        },
    );
    // Dropping these structured futures cancels in-flight reads and rolls back
    // unfinished transactions; no detached resource task survives its session.
    tokio::select! {
        () = cancellation.cancelled() => Ok(()),
        () = session.closed() => Ok(()),
        result = observe_events(session.clone(), pool.clone(), realtime.clone(), settings, cancellation, sender.clone()) => result,
        () = scopes => Ok(()),
        () = safety => Ok(()),
    }
}
