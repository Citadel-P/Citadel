use crate::realtime::RealtimeHub;
use citadel_adapters::connectors::edge::EdgeRegistry;
use citadel_adapters::connectors::edge::EdgeRuntime;
use citadel_adapters::connectors::edge::EdgeSession;
use citadel_adapters::persistence::postgres::platforms::edge::store::PostgresEdgeStore;
use citadel_contracts::citadel::edge::v1::EdgeCommandKind;
use citadel_platforms::jobs::{
    ContainerChange, DeltaOutcome, EventRefresh, ReconciliationDecision, RuntimeEventKind,
    event_decision,
};
use citadel_runtime::IoBudget;
use futures_util::StreamExt;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct InventorySettings {
    pub node_policy: citadel_adapters::persistence::postgres::platforms::node_agents::reconciliation::NodeAgentReconciliationPolicy,
    pub reconciliation_interval: Duration,
    pub monitoring_interval: Duration,
}

/// One supervised task per authenticated Edge Platform or Node. Reconnects
/// replace the task; event bursts are coalesced and full scans are bounded.
pub async fn run(
    cancellation: CancellationToken,
    registry: EdgeRegistry,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    settings: InventorySettings,
    scans: IoBudget,
    stats: super::statistics::StatsIngress,
) -> Result<(), std::convert::Infallible> {
    let mut active = HashMap::new();
    let mut tasks = JoinSet::new();
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            () = cancellation.cancelled() => break,
            result = tasks.join_next_with_id(), if !tasks.is_empty() => {
                match result {
                    Some(Ok((id, ()))) => { active.remove(&id); },
                    Some(Err(error)) => {
                        active.remove(&error.id());
                        tracing::warn!(%error,"Edge inventory task failed");
                    },
                    None => {}
                }
            }
            _ = tick.tick() => {
                for session in registry.current_sessions().into_iter().filter(|session| session.target.resource_type==0) {
                    if active.values().any(|id| *id == session.id) { continue; }
                    let session_id = session.id;
                    let pool=pool.clone(); let realtime=realtime.clone(); let settings=settings.clone(); let scans=scans.clone(); let cancellation=cancellation.child_token();
                    let stats = stats.clone();
                    let task = tasks.spawn(async move {
                        tokio::select! {
                            ()=cancellation.cancelled()=>{},
                            ()=session.closed()=>{},
                            result=monitor(session.clone(),pool,realtime,settings,scans,stats,&cancellation)=>{
                                if let Err(error)=result { tracing::warn!(%error,platform_id=%session.target.platform_id,"Edge inventory synchronization interrupted"); }
                            }
                        }
                    });
                    active.insert(task.id(), session_id);
                }
            }
        }
    }
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    Ok(())
}

async fn monitor(
    session: Arc<EdgeSession>,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    settings: InventorySettings,
    scans: IoBudget,
    stats: super::statistics::StatsIngress,
    cancellation: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval = settings.monitoring_interval;
    tokio::try_join!(
        observe(
            session.clone(),
            pool.clone(),
            realtime.clone(),
            scans,
            settings,
            cancellation
        ),
        observe_stats(session, stats, interval, cancellation),
    )?;
    Ok(())
}

async fn observe_stats(
    session: Arc<EdgeSession>,
    ingress: super::statistics::StatsIngress,
    interval: Duration,
    cancellation: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runtime = EdgeRuntime {
        session: session.clone(),
    };
    let mut stream = runtime.stream_container_stats(interval, cancellation)?;
    let mut disk = super::disk::LatestPlatformStats::new(interval);
    let mut disk_updates = if session.target.node_id.is_none() {
        super::disk::samples(runtime, interval, cancellation.clone())
    } else {
        Box::pin(futures_util::stream::pending())
    };
    loop {
        let stats = tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            update = disk_updates.next() => { disk.record(update.flatten()); continue; },
            stats = stream.next() => stats,
        };
        let Some(stats) = stats else {
            break;
        };
        let stats = stats?;
        if session.is_closed() {
            return Ok(());
        }
        let scope = citadel_platforms::stats_ingestion::StatsScope {
            platform_id: session.target.platform_id,
            node_id: session.target.node_id.clone(),
            connector: "EdgeAgent".into(),
            address: None,
            agent_id: Some(session.agent_id),
            connected_at: Some(session.connected_at),
            closed: Some(session.observation_token()),
        };
        if !ingress
            .submit(scope, stats, disk.get().cloned(), cancellation)
            .await
        {
            return Ok(());
        }
    }
    Err("Edge statistics stream ended.".into())
}

async fn observe_events(
    session: Arc<EdgeSession>,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    settings: InventorySettings,
    cancellation: &CancellationToken,
    refreshes: super::platforms::scoped_reconciler::EventRefreshSender,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let platform_id = session.target.platform_id;
    let (platform_type,_expected_daemon):(String,String)=sqlx::query_as("SELECT COALESCE(platform.platformdescriptor::jsonb->>'$type','Docker'),binding.dockerdaemonid FROM platforms platform JOIN edgeagentbindings binding ON binding.platformid=platform.id WHERE platform.id=$1 AND binding.agentid=$2 AND binding.lastconnectedatutc=$3 AND binding.revokedatutc IS NULL").bind(platform_id).bind(session.agent_id).bind(session.connected_at).fetch_one(&pool).await?;
    let mut platform_type =
        citadel_adapters::persistence::postgres::platforms::classification::platform_kind(
            &platform_type,
        )?;
    // Workers do not expose manager-only Swarm inventory APIs.
    if session.target.node_id.is_some() {
        platform_type = citadel_platforms::PlatformKind::Docker;
    }
    let store = PostgresEdgeStore::new(pool.clone()).with_node_policy(settings.node_policy);
    let mut events = session.command(
        EdgeCommandKind::PlatformDaemonEventsStream,
        vec![],
        Duration::from_secs(3600),
        true,
    )?;
    use citadel_platforms::jobs::{RecoveryReason, RuntimeRecoveryCoordinator};
    let request = |refresh| super::platforms::event_refresh::ScopedEventRequest {
        platform_id,
        refresh,
    };
    if let Some(refresh) =
        RuntimeRecoveryCoordinator::request(RecoveryReason::Bootstrap, platform_type)
    {
        if refreshes
            .send(request(refresh), cancellation)
            .await
            .is_err()
        {
            return Ok(());
        }
    }
    let safety = Duration::from_secs(6 * 3600);
    let mut timer = tokio::time::interval_at(tokio::time::Instant::now() + safety, safety);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Keep decode/receive time inside the pinned stream: timer cancellation
    // never discards a received event or changes its observation timestamp.
    let decoded = async_stream::stream! {
        loop {
            let result: Result<_, Box<dyn std::error::Error + Send + Sync>> = async {
                let Some(bytes) = events.next(cancellation).await? else { return Ok(None) };
                let event = citadel_adapters::connectors::agent::client::decode_daemon_event(bytes.as_slice())?;
                Ok(Some(event.map(|event| (event, chrono::Utc::now().timestamp_millis()))))
            }.await;
            match result {
                Ok(Some(Some(event))) => yield Ok(event),
                Ok(Some(None)) => continue,
                Ok(None) => break,
                Err(error) => { yield Err(error); break; }
            }
        }
    };
    let decoded = decoded.fuse();
    tokio::pin!(decoded);
    let mut pending = None;
    loop {
        let event = if let Some(event) = pending.take() {
            event
        } else {
            tokio::select! {
                ()=cancellation.cancelled()=>return Ok(()),
                ()=session.closed()=>return Ok(()),
                _ = timer.tick() => {
                    let _ = refreshes.send(request(EventRefresh::Platform), cancellation).await;
                    continue;
                }
                event=decoded.next()=>{
                    let Some(event)=event else { return Ok(()); };
                    event
                }
            }
        };
        // The stream can become ready with its cancellation error in the
        // same poll as shutdown. Shutdown is a clean completion, not a retry.
        if cancellation.is_cancelled() || session.is_closed() {
            return Ok(());
        }
        let (event, observed) = event?;
        if edge_state_event(&event) {
            use super::container_batch;
            let Some((batch, carry)) = container_batch::gather(
                Ok((event, observed)),
                decoded.as_mut(),
                |_, next| {
                    next.as_ref()
                        .is_ok_and(|(event, _)| edge_state_event(event))
                },
                cancellation,
            )
            .await
            else {
                return Ok(());
            };
            pending = carry;
            let batch = container_batch::coalesce(
                batch.into_iter().map(Result::unwrap).collect(),
                |(event, _)| event.container_id.clone().unwrap(),
                |(_, time)| *time,
            );
            let deltas: Vec<_> = batch
                .into_iter()
                .map(|(event, time)| {
                    let RuntimeEventKind::Container(change) = event.kind else {
                        unreachable!()
                    };
                    citadel_platforms::jobs::ContainerStateDelta {
                        docker_id: event.container_id.unwrap(),
                        state: change.state_delta().unwrap(),
                        observed_at: time / 1000,
                        observed_at_millis: time,
                    }
                })
                .collect();
            let unavailable = match store
                .persist_container_state_deltas(&session, &deltas)
                .await
            {
                Ok(results) => {
                    container_batch::publish(realtime.as_ref(), platform_id, &results);
                    results.iter().any(|r| !r.accepted)
                }
                Err(error) => {
                    tracing::warn!(%error, %platform_id, "Edge state batch failed; requesting container scope");
                    true
                }
            };
            container_batch::refresh(
                platform_id,
                platform_type == citadel_platforms::PlatformKind::DockerSwarm,
                unavailable,
                &refreshes,
                cancellation,
            )
            .await;
            continue;
        }
        let mut outcome = DeltaOutcome::Unavailable;
        if matches!(event.kind, RuntimeEventKind::Container(_)) {
            match store
                .persist_container_event_at(&session, &event, observed)
                .await
            {
                Ok(updated) => {
                    if updated.accepted()
                        || event.kind == RuntimeEventKind::Container(ContainerChange::Tombstone)
                    {
                        outcome = DeltaOutcome::Applied;
                    }
                    if updated.changed()
                        && let Some(hub) = &realtime
                    {
                        hub.publish_container_observation(
                            platform_id,
                            &event.action,
                            event.container_id.clone().unwrap_or_default(),
                        );
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, %platform_id, "Edge delta failed; requesting only its resource scope")
                }
            }
        }
        if let Some(delta) = &event.resource {
            match store.persist_resource_event(&session, &event).await {
                Ok(updated) => {
                    if updated.accepted() {
                        outcome = DeltaOutcome::Applied;
                    }
                    if updated.changed()
                        && let Some(hub) = &realtime
                    {
                        hub.publish_resource_observation(
                            platform_id,
                            session.target.node_id.as_deref(),
                            &event.action,
                            delta,
                        );
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, %platform_id, "Edge resource delta failed; requesting its scope")
                }
            }
        }
        let refresh = match event_decision(
            event.kind,
            platform_type == citadel_platforms::PlatformKind::DockerSwarm,
            event.swarm_scope,
            outcome,
        ) {
            ReconciliationDecision::Reconcile(scope) => EventRefresh::Resource(scope),
            ReconciliationDecision::SwarmDirty => EventRefresh::Swarm,
            ReconciliationDecision::None | ReconciliationDecision::ApplyDelta => continue,
        };
        let request = super::platforms::event_refresh::ScopedEventRequest {
            platform_id,
            refresh,
        };
        if refreshes.send(request, cancellation).await.is_err() {
            return Ok(());
        }
        if platform_type == citadel_platforms::PlatformKind::DockerSwarm
            && matches!(event.kind, RuntimeEventKind::Container(_))
            && refresh != EventRefresh::Swarm
            && refreshes
                .send(
                    super::platforms::event_refresh::ScopedEventRequest {
                        platform_id,
                        refresh: EventRefresh::Swarm,
                    },
                    cancellation,
                )
                .await
                .is_err()
        {
            return Ok(());
        }
    }
}

fn edge_state_event(event: &citadel_adapters::connectors::agent::client::AgentDaemonEvent) -> bool {
    matches!(event.kind, RuntimeEventKind::Container(change) if change.state_delta().is_some())
        && event.container_id.as_ref().is_some_and(|id| !id.is_empty())
}

#[cfg(test)]
#[path = "edge_event_tests.rs"]
mod event_tests;

mod reconciliation;
use reconciliation::observe;
