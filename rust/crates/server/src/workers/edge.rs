use crate::realtime::RealtimeHub;
use citadel_adapters::edge::{EdgeRegistry, EdgeRuntime, EdgeSession, PostgresEdgeStore};
use citadel_contracts::citadel::edge::v1::EdgeCommandKind;
use citadel_platforms::jobs::{InventoryCollectionTarget, collect_inventory};
use futures_util::StreamExt;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::Semaphore, task::JoinSet};
use tokio_util::sync::CancellationToken;

/// One supervised task per authenticated Edge Platform or Node. Reconnects
/// replace the task; event bursts are coalesced and full scans are bounded.
pub async fn run(
    cancellation: CancellationToken,
    registry: EdgeRegistry,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    alerts: Arc<dyn citadel_alerts::AlertEventSink>,
) -> Result<(), std::convert::Infallible> {
    let mut active = HashMap::new();
    let mut tasks = JoinSet::new();
    let scans = Arc::new(Semaphore::new(4));
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
                    let pool=pool.clone(); let realtime=realtime.clone(); let alerts=alerts.clone(); let scans=scans.clone(); let cancellation=cancellation.child_token();
                    let task = tasks.spawn(async move {
                        tokio::select! {
                            ()=cancellation.cancelled()=>{},
                            ()=session.closed()=>{},
                            result=monitor(session.clone(),pool,realtime,scans,alerts,&cancellation)=>{
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
    scans: Arc<Semaphore>,
    alerts: Arc<dyn citadel_alerts::AlertEventSink>,
    cancellation: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::try_join!(
        observe(
            session.clone(),
            pool.clone(),
            realtime.clone(),
            scans,
            cancellation
        ),
        observe_stats(session, pool, realtime, alerts, cancellation),
    )?;
    Ok(())
}

async fn observe_stats(
    session: Arc<EdgeSession>,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    alerts: Arc<dyn citadel_alerts::AlertEventSink>,
    cancellation: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runtime = EdgeRuntime {
        session: session.clone(),
    };
    let store = PostgresEdgeStore::new(pool.clone());
    let mut stream = runtime.stream_container_stats(Duration::from_secs(10), cancellation)?;
    let interval = Duration::from_secs(10);
    let mut disk = super::disk::LatestDisk::new(interval);
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
        let persisted = store
            .persist_stats_with_disk(&session, &stats, disk.get())
            .await?;
        if session.target.node_id.is_none() && (persisted > 0 || stats.is_empty()) {
            super::platforms::observe_platform_metrics(
                &pool,
                alerts.as_ref(),
                session.target.platform_id,
            )
            .await;
        }
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
    scans: Arc<Semaphore>,
    cancellation: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let platform_id = session.target.platform_id;
    let (mut platform_type,expected_daemon):(String,String)=sqlx::query_as("SELECT CASE WHEN platform.platformdescriptor::jsonb->>'$type'='DockerSwarm' THEN 'DockerSwarm' ELSE 'Docker' END,binding.dockerdaemonid FROM platforms platform JOIN edgeagentbindings binding ON binding.platformid=platform.id WHERE platform.id=$1 AND binding.agentid=$2 AND binding.lastconnectedatutc=$3 AND binding.revokedatutc IS NULL").bind(platform_id).bind(session.agent_id).bind(session.connected_at).fetch_one(&pool).await?;
    // Workers do not expose manager-only Swarm inventory APIs.
    if session.target.node_id.is_some() {
        platform_type = "Docker".into();
    }
    let target = InventoryCollectionTarget {
        platform_id,
        platform_type,
    };
    let runtime = EdgeRuntime {
        session: session.clone(),
    };
    let store = PostgresEdgeStore::new(pool);
    let mut events = session.command(
        EdgeCommandKind::PlatformDaemonEventsStream,
        vec![],
        Duration::from_secs(3600),
        true,
    )?;
    loop {
        {
            let _permit = scans.acquire().await?;
            let snapshot = collect_inventory(&runtime, &target, cancellation).await?;
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
        tokio::select! {
            ()=cancellation.cancelled()=>return Ok(()),
            ()=session.closed()=>return Ok(()),
            ()=tokio::time::sleep(Duration::from_secs(1800))=>{},
            event=events.next(cancellation)=>{ if event?.is_none() { return Ok(()); } }
        }
        // Bound scan frequency under daemon event bursts without making the
        // source queue unbounded. Overflow terminates/reopens with a full scan.
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
