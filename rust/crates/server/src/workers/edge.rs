use crate::realtime::RealtimeHub;
use citadel_adapters::connectors::edge::EdgeRegistry;
use citadel_adapters::connectors::edge::EdgeRuntime;
use citadel_adapters::connectors::edge::EdgeSession;
use citadel_adapters::persistence::postgres::platforms::edge::store::PostgresEdgeStore;
use citadel_contracts::citadel::edge::v1::EdgeCommandKind;
use citadel_platforms::jobs::{InventoryCollectionTarget, collect_inventory};
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
                    let task = tasks.spawn(async move {
                        tokio::select! {
                            ()=cancellation.cancelled()=>{},
                            ()=session.closed()=>{},
                            result=monitor(session.clone(),pool,realtime,settings,scans,&cancellation)=>{
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
    cancellation: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::try_join!(
        observe(
            session.clone(),
            pool.clone(),
            realtime.clone(),
            scans,
            settings,
            cancellation
        ),
        observe_stats(session, pool, realtime, cancellation),
    )?;
    Ok(())
}

async fn observe_stats(
    session: Arc<EdgeSession>,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    cancellation: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runtime = EdgeRuntime {
        session: session.clone(),
    };
    let store = PostgresEdgeStore::new(pool.clone());
    let mut stream = runtime.stream_container_stats(Duration::from_secs(10), cancellation)?;
    let interval = Duration::from_secs(10);
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
        let sample_disk = disk.get();
        let mut delay = Duration::from_secs(2);
        let persisted = loop {
            let result = tokio::select! {
                ()=cancellation.cancelled()=>return Ok(()),
                result=store.persist_stats_with_platform_stats(&session,&stats,sample_disk)=>result,
            };
            match result {
                Ok(inserted) => break inserted,
                Err(citadel_adapters::persistence::postgres::platforms::edge::store::EdgeStoreError::Storage(error)) => {
                    tracing::warn!(%error,platform_id=%session.target.platform_id,"Edge statistics persistence failed; retaining batch for retry");
                    tokio::select! {()=cancellation.cancelled()=>return Ok(()),_=tokio::time::sleep(delay)=>{}}
                    delay = (delay * 2).min(Duration::from_secs(30));
                }
                Err(error) => return Err(error.into()),
            }
        };
        if (persisted > 0 || (session.target.node_id.is_none() && stats.is_empty()))
            && let Some(hub) = &realtime
        {
            hub.publish_scoped_container_stats(
                session.target.platform_id,
                session.target.node_id.as_deref(),
                &stats,
            );
        }
    }
    Err("Edge statistics stream ended.".into())
}

async fn observe(
    session: Arc<EdgeSession>,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    scans: IoBudget,
    settings: InventorySettings,
    cancellation: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let platform_id = session.target.platform_id;
    let (platform_type,expected_daemon):(String,String)=sqlx::query_as("SELECT COALESCE(platform.platformdescriptor::jsonb->>'$type','Docker'),binding.dockerdaemonid FROM platforms platform JOIN edgeagentbindings binding ON binding.platformid=platform.id WHERE platform.id=$1 AND binding.agentid=$2 AND binding.lastconnectedatutc=$3 AND binding.revokedatutc IS NULL").bind(platform_id).bind(session.agent_id).bind(session.connected_at).fetch_one(&pool).await?;
    let mut platform_type =
        citadel_adapters::persistence::postgres::platforms::classification::platform_kind(
            &platform_type,
        )?;
    // Workers do not expose manager-only Swarm inventory APIs.
    if session.target.node_id.is_some() {
        platform_type = citadel_platforms::PlatformKind::Docker;
    }
    let target = InventoryCollectionTarget {
        platform_id,
        platform_type,
    };
    let runtime = EdgeRuntime {
        session: session.clone(),
    };
    let store = PostgresEdgeStore::new(pool.clone()).with_node_policy(settings.node_policy);
    let mut events = session.command(
        EdgeCommandKind::PlatformDaemonEventsStream,
        vec![],
        Duration::from_secs(3600),
        true,
    )?;
    loop {
        {
            let Some(_permit) = scans.enter(cancellation).await else {
                return Ok(());
            };
            let started = chrono::Utc::now();
            let snapshot = match collect_inventory(&runtime, &target, cancellation).await {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    if target.platform_type == citadel_platforms::PlatformKind::DockerSwarm
                        && !cancellation.is_cancelled()
                    {
                        citadel_adapters::persistence::postgres::platforms::inventory::store::PostgresInventoryProjectionStore::new(pool.clone())
                            .mark_swarm_stale(platform_id,started).await?;
                    }
                    return Err(error.into());
                }
            };
            // Enrollment binds a daemon; reject transport identity changes before
            // they can overwrite another daemon's persisted inventory.
            if expected_daemon.is_empty() || snapshot.info.daemon_id != expected_daemon {
                return Err("Edge Agent Docker identity does not match its binding.".into());
            }
            store.persist_inventory(&session, &snapshot).await?;
            if let Some(hub) = &realtime {
                hub.publish_runtime_change(
                    platform_id,
                    "platform",
                    "update",
                    platform_id.to_string(),
                );
            }
        }
        // Stay on the event path after a targeted update. Only discovery, other
        // resource events, or the periodic deadline require a full scan.
        let deadline = tokio::time::Instant::now() + settings.reconciliation_interval;
        loop {
            let bytes = tokio::select! {
                ()=cancellation.cancelled()=>return Ok(()),
                ()=session.closed()=>return Ok(()),
                ()=tokio::time::sleep_until(deadline)=>break,
                event=events.next(cancellation)=>{ let Some(bytes)=event? else { return Ok(()); }; bytes }
            };
            let Some(event) =
                citadel_adapters::connectors::agent::client::decode_daemon_event(bytes.as_slice())?
            else {
                continue;
            };
            if event.resource_type == "container"
                && store.persist_container_event(&session, &event).await?
            {
                if let Some(hub) = &realtime {
                    hub.publish_runtime_change(
                        platform_id,
                        "container",
                        &event.action,
                        event.container_id.unwrap_or_default(),
                    );
                }
            } else {
                break;
            }
        }
        // Bound scan frequency under daemon event bursts without making the
        // source queue unbounded. Overflow terminates/reopens with a full scan.
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
