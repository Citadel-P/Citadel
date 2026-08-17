use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::Utc;
use citadel_adapters::agent::AgentClient;
use citadel_adapters::container_stats_store::PostgresContainerStatsStore;
use citadel_adapters::docker::{DockerClient, DockerError};
use citadel_adapters::inventory_projection_store::PostgresInventoryProjectionStore;
use citadel_application::{
    BoundedReceiver, BoundedSender, QueueOverflowPolicy, TaskSupervisor, bounded_channel,
};
use citadel_platforms::{
    ContainerStatsStore, InventoryProjectionStore, PlatformInventoryPort, PlatformRuntimePort,
    RuntimeCapabilityError, RuntimeInventorySnapshot, RuntimeSwarmInventory,
};
use futures_util::{StreamExt, stream};
use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;

use crate::Readiness;
use crate::metrics::Metrics;
use crate::realtime::RealtimeHub;

pub struct WorkerSettings {
    pub queue_capacity: usize,
    pub probe_interval: Duration,
    pub reconciliation_interval: Duration,
    pub agent_reconnect_delay: Duration,
}

pub struct WorkerDependencies {
    pub docker: DockerClient,
    pub pool: PgPool,
    pub readiness: Arc<Readiness>,
    pub metrics: Arc<Metrics>,
    pub agent: Option<AgentClient>,
    pub realtime: Option<RealtimeHub>,
}

pub fn register(
    supervisor: &mut TaskSupervisor,
    cancellation: &CancellationToken,
    dependencies: WorkerDependencies,
    settings: WorkerSettings,
) {
    let WorkerDependencies {
        docker,
        pool,
        readiness,
        metrics,
        agent,
        realtime,
    } = dependencies;
    let (sender, receiver) = bounded_channel(settings.queue_capacity, QueueOverflowPolicy::Wait);
    let (local_reconcile_sender, local_reconcile_receiver) =
        bounded_channel(1, QueueOverflowPolicy::Reject);
    let (agent_reconcile_sender, agent_reconcile_receiver) =
        bounded_channel(1, QueueOverflowPolicy::Reject);

    supervisor.spawn(
        "docker-event-source",
        event_source(
            cancellation.child_token(),
            docker.clone(),
            sender.clone(),
            Arc::clone(&metrics),
        ),
    );
    if let Some(agent) = agent.clone() {
        supervisor.spawn(
            "agent-event-source",
            agent_event_source(
                cancellation.child_token(),
                agent,
                sender,
                Arc::clone(&metrics),
                settings.agent_reconnect_delay,
            ),
        );
    }
    supervisor.spawn(
        "docker-event-consumer",
        event_consumer(
            cancellation.child_token(),
            receiver,
            Arc::clone(&metrics),
            local_reconcile_sender,
            agent_reconcile_sender,
        ),
    );
    supervisor.spawn(
        "platform-inventory-reconciliation",
        inventory_reconciliation(
            cancellation.child_token(),
            InventoryReconciliationWorker {
                docker: docker.clone(),
                agent: agent.clone(),
                pool: pool.clone(),
                local_triggers: local_reconcile_receiver,
                agent_triggers: agent_reconcile_receiver,
                realtime: realtime.clone(),
                interval: settings.reconciliation_interval,
                retry_delay: settings.agent_reconnect_delay,
            },
        ),
    );
    supervisor.spawn(
        "readiness-probe",
        readiness_probe(
            cancellation.child_token(),
            docker.clone(),
            pool.clone(),
            readiness,
            Arc::clone(&metrics),
            settings.probe_interval,
        ),
    );
    if let Some(agent) = agent {
        supervisor.spawn(
            "agent-container-stats",
            agent_container_stats(
                cancellation.child_token(),
                agent,
                pool.clone(),
                Arc::clone(&metrics),
                realtime.clone(),
                settings.probe_interval,
                settings.agent_reconnect_delay,
            ),
        );
    }
    supervisor.spawn(
        "local-container-stats",
        local_container_stats(
            cancellation.child_token(),
            docker,
            pool,
            Arc::clone(&metrics),
            realtime,
            settings.probe_interval,
        ),
    );
}

#[derive(Debug, Clone, Copy)]
enum ReconciliationTrigger {
    LocalEvent,
    AgentEvent,
}

#[derive(Debug)]
struct InventoryEvent {
    source: ReconciliationTrigger,
    resource_type: String,
    action: String,
    event_time_millis: Option<u128>,
}

#[derive(Debug)]
struct ReconciliationTarget {
    id: uuid::Uuid,
    address: String,
    connector_type: String,
    platform_type: String,
}

struct InventoryReconciliationWorker {
    docker: DockerClient,
    agent: Option<AgentClient>,
    pool: PgPool,
    local_triggers: BoundedReceiver<()>,
    agent_triggers: BoundedReceiver<()>,
    realtime: Option<RealtimeHub>,
    interval: Duration,
    retry_delay: Duration,
}

async fn inventory_reconciliation(
    cancellation: CancellationToken,
    mut worker: InventoryReconciliationWorker,
) -> Result<(), std::convert::Infallible> {
    let store = PostgresInventoryProjectionStore::new(worker.pool.clone());
    let mut ticker = tokio::time::interval_at(
        tokio::time::Instant::now() + worker.interval,
        worker.interval,
    );
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut scope = None;

    loop {
        if scope.is_some() {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                () = tokio::time::sleep(Duration::from_millis(500)) => {}
            }
            if matches!(scope, Some(ReconciliationTrigger::LocalEvent)) {
                while worker.local_triggers.try_recv().is_some() {}
            } else {
                while worker.agent_triggers.try_recv().is_some() {}
            }
        }

        if let Err(error) = reconcile_inventory(
            &cancellation,
            &worker.docker,
            worker.agent.as_ref(),
            &worker.pool,
            &store,
            worker.realtime.as_ref(),
            scope,
        )
        .await
        {
            tracing::warn!(%error, "platform inventory reconciliation failed");
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                () = tokio::time::sleep(worker.retry_delay) => {}
            }
            continue;
        }

        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            trigger = worker.local_triggers.recv(&cancellation) => {
                if trigger.is_none() {
                    return Ok(());
                }
                scope = Some(ReconciliationTrigger::LocalEvent);
            }
            trigger = worker.agent_triggers.recv(&cancellation) => {
                if trigger.is_none() {
                    return Ok(());
                }
                scope = Some(ReconciliationTrigger::AgentEvent);
            }
            _ = ticker.tick() => scope = None,
        }
    }
}

async fn reconcile_inventory(
    cancellation: &CancellationToken,
    docker: &DockerClient,
    agent: Option<&AgentClient>,
    pool: &PgPool,
    store: &dyn InventoryProjectionStore,
    realtime: Option<&RealtimeHub>,
    scope: Option<ReconciliationTrigger>,
) -> Result<(), RuntimeCapabilityError> {
    let targets = reconciliation_targets(pool).await?;
    for target in targets {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        match scope {
            Some(ReconciliationTrigger::LocalEvent)
                if !target.connector_type.eq_ignore_ascii_case("Local") =>
            {
                continue;
            }
            Some(ReconciliationTrigger::AgentEvent)
                if !target.connector_type.eq_ignore_ascii_case("Agent") =>
            {
                continue;
            }
            _ => {}
        }
        let runtime: &dyn PlatformInventoryPort =
            if target.connector_type.eq_ignore_ascii_case("Local") {
                docker
            } else if target.connector_type.eq_ignore_ascii_case("Agent") {
                let Some(agent) = agent else {
                    continue;
                };
                if agent.address().trim_end_matches('/') != target.address.trim_end_matches('/') {
                    continue;
                }
                agent
            } else {
                // Edge Agents use an inbound command session. That resolver is registered
                // when the Edge transport owns a live session; disconnected targets keep
                // their last bounded projection instead of being overwritten.
                continue;
            };

        match collect_inventory(runtime, &target, cancellation).await {
            Ok(snapshot) => {
                let change = store.persist(&snapshot).await?;
                if let Some(realtime) = realtime {
                    realtime.publish_runtime_change(
                        target.id,
                        "platformInventory",
                        "reconciled",
                        change.platform_id.to_string(),
                    );
                }
            }
            Err(error) if error.kind == citadel_platforms::RuntimeErrorKind::Cancelled => {
                return Ok(());
            }
            Err(error) => {
                tracing::warn!(
                    platform_id = %target.id,
                    connector_type = %target.connector_type,
                    %error,
                    "platform inventory target failed"
                );
            }
        }
    }
    Ok(())
}

async fn reconciliation_targets(
    pool: &PgPool,
) -> Result<Vec<ReconciliationTarget>, RuntimeCapabilityError> {
    let rows = sqlx::query(
        r#"
SELECT id, address, connectortype,
       COALESCE(platformdescriptor->>'$type', 'Docker') AS platformtype
FROM platforms
WHERE connectortype IN ('Local', 'Agent', 'EdgeAgent')
ORDER BY id
"#,
    )
    .fetch_all(pool)
    .await
    .map_err(worker_storage)?;
    rows.into_iter()
        .map(|row| {
            Ok(ReconciliationTarget {
                id: row.try_get("id").map_err(worker_storage)?,
                address: row.try_get("address").map_err(worker_storage)?,
                connector_type: row.try_get("connectortype").map_err(worker_storage)?,
                platform_type: row.try_get("platformtype").map_err(worker_storage)?,
            })
        })
        .collect()
}

async fn collect_inventory(
    runtime: &dyn PlatformInventoryPort,
    target: &ReconciliationTarget,
    cancellation: &CancellationToken,
) -> Result<RuntimeInventorySnapshot, RuntimeCapabilityError> {
    let (info, containers, images, networks, volumes) = tokio::try_join!(
        runtime.get_info(cancellation),
        runtime.list_containers(cancellation),
        runtime.list_images(cancellation),
        runtime.list_networks(cancellation),
        runtime.list_volumes(cancellation),
    )?;
    let swarm = if target.platform_type.eq_ignore_ascii_case("DockerSwarm") {
        let (nodes, services, tasks, configs, secrets) = tokio::try_join!(
            runtime.list_swarm_nodes(cancellation),
            runtime.list_swarm_services(cancellation),
            runtime.list_swarm_tasks(cancellation),
            runtime.list_swarm_configs(cancellation),
            runtime.list_swarm_secrets(cancellation),
        )?;
        Some(RuntimeSwarmInventory {
            nodes,
            services,
            tasks,
            configs,
            secrets,
        })
    } else {
        None
    };
    Ok(RuntimeInventorySnapshot {
        platform_id: target.id,
        info,
        containers,
        images,
        networks,
        volumes,
        swarm,
        observed_at: Utc::now(),
    })
}

fn worker_storage(error: impl std::fmt::Display) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        citadel_platforms::RuntimeErrorKind::Remote,
        error.to_string(),
        true,
    )
}

async fn local_container_stats(
    cancellation: CancellationToken,
    docker: DockerClient,
    pool: PgPool,
    metrics: Arc<Metrics>,
    realtime: Option<RealtimeHub>,
    fetch_interval: Duration,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    let store = PostgresContainerStatsStore::new(pool.clone());
    let mut ticker = tokio::time::interval(fetch_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {}
        }
        let platform_id = match local_platform_id(&pool).await {
            Ok(Some(id)) => id,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(%error, "local Platform lookup for statistics failed");
                continue;
            }
        };
        let containers = match PlatformRuntimePort::list_containers(&docker, &cancellation).await {
            Ok(containers) => containers,
            Err(error) => {
                if cancellation.is_cancelled() {
                    return Ok(());
                }
                tracing::warn!(%error, "local container inventory for statistics failed");
                continue;
            }
        };
        let stats = stream::iter(
            containers
                .into_iter()
                .filter(|container| container.state == "running")
                .take(1_024)
                .map(|container| {
                    let docker = docker.clone();
                    let cancellation = cancellation.clone();
                    async move {
                        docker
                            .sample_container_stats(&container.id, &cancellation)
                            .await
                    }
                }),
        )
        .buffer_unordered(8)
        .filter_map(|result| async move {
            match result {
                Ok(stat) => Some(stat),
                Err(error) => {
                    tracing::debug!(%error, "local container statistics sample failed");
                    None
                }
            }
        })
        .collect::<Vec<_>>()
        .await;
        if stats.is_empty() {
            continue;
        }
        match store.persist(platform_id, &stats).await {
            Ok(_) => {
                metrics.local_stats_sampled();
                if let Some(realtime) = realtime.as_ref() {
                    realtime.publish_container_stats(platform_id, &stats);
                }
            }
            Err(error) => {
                tracing::warn!(%error, %platform_id, "local container statistics persistence failed");
            }
        }
    }
}

async fn agent_container_stats(
    cancellation: CancellationToken,
    agent: AgentClient,
    pool: PgPool,
    metrics: Arc<Metrics>,
    realtime: Option<RealtimeHub>,
    fetch_interval: Duration,
    reconnect_delay: Duration,
) -> Result<(), RuntimeCapabilityError> {
    let _task = metrics.task_guard();
    let store = PostgresContainerStatsStore::new(pool.clone());
    loop {
        if let Err(error) = agent.get_info(&cancellation).await {
            if cancellation.is_cancelled() {
                return Ok(());
            }
            metrics.agent_handshake_failed();
            if !error.retryable {
                return Err(error);
            }
            tracing::warn!(%error, "Agent handshake failed");
            if wait_to_reconnect(&cancellation, reconnect_delay).await {
                return Ok(());
            }
            continue;
        }
        let platform_id = match agent_platform_id(&pool, agent.address()).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
            Err(error) => {
                tracing::warn!(%error, "Agent Platform lookup for statistics failed");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
        };
        let mut stream = match agent
            .stream_container_stats(fetch_interval, &cancellation)
            .await
        {
            Ok(stream) => stream,
            Err(_error) if cancellation.is_cancelled() => return Ok(()),
            Err(error) if error.retryable => {
                metrics.agent_stream_reconnected();
                tracing::warn!(%error, "Agent stats stream connection failed");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
            Err(error) => return Err(error),
        };
        loop {
            match stream.next().await {
                Some(Ok(stats)) => {
                    if stats.is_empty() {
                        continue;
                    }
                    match store.persist(platform_id, &stats).await {
                        Ok(_) => {
                            metrics.agent_stats_sampled();
                            if let Some(realtime) = realtime.as_ref() {
                                realtime.publish_container_stats(platform_id, &stats);
                            }
                        }
                        Err(error) => {
                            tracing::warn!(%error, %platform_id, "Agent container statistics persistence failed");
                        }
                    }
                }
                Some(Err(error)) if error.retryable => {
                    metrics.agent_stream_reconnected();
                    tracing::warn!(%error, "Agent stats stream interrupted");
                    break;
                }
                Some(Err(error)) => return Err(error),
                None if cancellation.is_cancelled() => return Ok(()),
                None => {
                    metrics.agent_stream_reconnected();
                    break;
                }
            }
        }
        if wait_to_reconnect(&cancellation, reconnect_delay).await {
            return Ok(());
        }
    }
}

async fn local_platform_id(pool: &PgPool) -> Result<Option<uuid::Uuid>, RuntimeCapabilityError> {
    sqlx::query_scalar("SELECT id FROM platforms WHERE connectortype = 'Local' ORDER BY id LIMIT 1")
        .fetch_optional(pool)
        .await
        .map_err(worker_storage)
}

async fn agent_platform_id(
    pool: &PgPool,
    address: &str,
) -> Result<Option<uuid::Uuid>, RuntimeCapabilityError> {
    sqlx::query_scalar(
        "SELECT id FROM platforms WHERE connectortype = 'Agent' AND rtrim(address, '/') = rtrim($1, '/') ORDER BY id LIMIT 1",
    )
    .bind(address)
    .fetch_optional(pool)
    .await
    .map_err(worker_storage)
}

async fn wait_to_reconnect(cancellation: &CancellationToken, reconnect_delay: Duration) -> bool {
    tokio::select! {
        biased;
        () = cancellation.cancelled() => true,
        () = tokio::time::sleep(reconnect_delay) => false,
    }
}

async fn event_source(
    cancellation: CancellationToken,
    docker: DockerClient,
    sender: BoundedSender<InventoryEvent>,
    metrics: Arc<Metrics>,
) -> Result<(), DockerError> {
    let _task = metrics.task_guard();
    let mut retry = Duration::from_millis(250);
    let mut since = None;
    let filters = std::collections::HashMap::from([(
        "type".to_owned(),
        [
            "container",
            "image",
            "network",
            "volume",
            "node",
            "service",
            "task",
            "secret",
            "config",
            "builder",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
    )]);
    loop {
        let stream = tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            result = docker.events(since, Some(&filters)) => result,
        };
        let mut stream = match stream {
            Ok(stream) => {
                retry = Duration::from_millis(250);
                stream
            }
            Err(error) => {
                metrics.stream_reconnected();
                tracing::warn!(%error, "Docker event stream connection failed");
                tokio::select! {
                    () = cancellation.cancelled() => return Ok(()),
                    () = tokio::time::sleep(retry) => {}
                }
                retry = std::cmp::min(retry.saturating_mul(2), Duration::from_secs(5));
                continue;
            }
        };

        loop {
            let event = tokio::select! {
                () = cancellation.cancelled() => return Ok(()),
                event = stream.next() => event,
            };
            match event {
                Some(Ok(event)) => {
                    if event.time > 0 {
                        // Docker's `since` bound is inclusive. Replaying the final second
                        // after reconnect is preferable to dropping sibling events; the
                        // projection trigger is coalesced and persistence is idempotent.
                        since = Some(event.time);
                    }
                    let event_time_millis = if event.time_nano > 0 {
                        Some(u128::try_from(event.time_nano).unwrap_or_default() / 1_000_000)
                    } else if event.time > 0 {
                        Some(u128::try_from(event.time).unwrap_or_default() * 1_000)
                    } else {
                        None
                    };
                    let event = InventoryEvent {
                        source: ReconciliationTrigger::LocalEvent,
                        resource_type: event.resource_type,
                        action: event.action,
                        event_time_millis,
                    };
                    if sender.send(event, &cancellation).await.is_err() {
                        return Ok(());
                    }
                    metrics.event_enqueued();
                }
                Some(Err(error)) => {
                    metrics.stream_reconnected();
                    tracing::warn!(%error, "Docker event stream interrupted");
                    break;
                }
                None => {
                    metrics.stream_reconnected();
                    break;
                }
            }
        }
    }
}

async fn agent_event_source(
    cancellation: CancellationToken,
    agent: AgentClient,
    sender: BoundedSender<InventoryEvent>,
    metrics: Arc<Metrics>,
    reconnect_delay: Duration,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    loop {
        let stream = agent.stream_daemon_events(&cancellation).await;
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) if !error.retryable => {
                tracing::error!(%error, "Agent daemon event stream rejected permanently");
                return Ok(());
            }
            Err(error) => {
                if cancellation.is_cancelled() {
                    return Ok(());
                }
                metrics.agent_stream_reconnected();
                tracing::warn!(%error, "Agent daemon event stream connection failed");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
        };
        loop {
            let event = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                event = stream.next() => event,
            };
            match event {
                Some(Ok(event)) => {
                    if sender
                        .send(
                            InventoryEvent {
                                source: ReconciliationTrigger::AgentEvent,
                                resource_type: event.resource_type.to_owned(),
                                action: event.action,
                                event_time_millis: None,
                            },
                            &cancellation,
                        )
                        .await
                        .is_err()
                    {
                        return Ok(());
                    }
                    metrics.event_enqueued();
                }
                Some(Err(error)) => {
                    if !error.retryable {
                        tracing::error!(%error, "Agent daemon event stream failed permanently");
                        return Ok(());
                    }
                    metrics.agent_stream_reconnected();
                    tracing::warn!(%error, "Agent daemon event stream interrupted");
                    break;
                }
                None => {
                    metrics.agent_stream_reconnected();
                    break;
                }
            }
        }
        if wait_to_reconnect(&cancellation, reconnect_delay).await {
            return Ok(());
        }
    }
}

async fn event_consumer(
    cancellation: CancellationToken,
    mut receiver: BoundedReceiver<InventoryEvent>,
    metrics: Arc<Metrics>,
    local_reconciliation: BoundedSender<()>,
    agent_reconciliation: BoundedSender<()>,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    loop {
        let event = match receiver.recv(&cancellation).await {
            Some(event) => event,
            None => return Ok(()),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let lag = event
            .event_time_millis
            .map_or(0, |event_time| now.saturating_sub(event_time))
            .min(u128::from(u64::MAX)) as u64;
        metrics.event_consumed(lag);
        queue_inventory_reconciliation(&event, &local_reconciliation, &agent_reconciliation);
    }
}

fn queue_inventory_reconciliation(
    event: &InventoryEvent,
    local: &BoundedSender<()>,
    agent: &BoundedSender<()>,
) {
    if !triggers_inventory_reconciliation(&event.resource_type, &event.action) {
        return;
    }
    let sender = match event.source {
        ReconciliationTrigger::LocalEvent => local,
        ReconciliationTrigger::AgentEvent => agent,
    };
    let _ = sender.try_send(());
}

fn triggers_inventory_reconciliation(resource_type: &str, action: &str) -> bool {
    matches!(
        resource_type.to_ascii_lowercase().as_str(),
        "container"
            | "image"
            | "network"
            | "volume"
            | "node"
            | "service"
            | "task"
            | "secret"
            | "config"
            | "builder"
    ) && !action.is_empty()
}

async fn readiness_probe(
    cancellation: CancellationToken,
    docker: DockerClient,
    pool: PgPool,
    readiness: Arc<Readiness>,
    metrics: Arc<Metrics>,
    interval: Duration,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {
                let database_ready = sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(&pool)
                    .await
                    .is_ok();
                let docker_ready = docker.ping().await.is_ok();
                let setup = if readiness.is_setup() {
                    true
                } else {
                    sqlx::query_scalar::<_, bool>(
                        "SELECT initializedat IS NOT NULL FROM instancesetupstates ORDER BY id LIMIT 1"
                    )
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or(false)
                };
                readiness.set(database_ready, docker_ready);
                readiness.set_setup(setup);
                if !database_ready || !docker_ready {
                    metrics.readiness_failed();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_platforms::{
        RuntimeContainerSummary, RuntimeImageSummary, RuntimeNetworkSummary, RuntimePlatformInfo,
        RuntimeStatsStream, RuntimeSwarmConfig, RuntimeSwarmNode, RuntimeSwarmSecret,
        RuntimeSwarmService, RuntimeSwarmTask, RuntimeVolumeSummary,
    };
    use futures_util::future::BoxFuture;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn only_inventory_events_trigger_reconciliation() {
        for resource_type in [
            "container",
            "image",
            "network",
            "volume",
            "node",
            "service",
            "task",
            "secret",
            "config",
            "builder",
        ] {
            assert!(triggers_inventory_reconciliation(resource_type, "update"));
        }
        assert!(!triggers_inventory_reconciliation("plugin", "enable"));
        assert!(!triggers_inventory_reconciliation("service", ""));
    }

    #[test]
    fn local_event_bursts_cannot_hide_an_agent_reconciliation_trigger() {
        let (local, mut local_receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
        let (agent, mut agent_receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
        let local_event = InventoryEvent {
            source: ReconciliationTrigger::LocalEvent,
            resource_type: "container".into(),
            action: "start".into(),
            event_time_millis: None,
        };
        let agent_event = InventoryEvent {
            source: ReconciliationTrigger::AgentEvent,
            resource_type: "service".into(),
            action: "update".into(),
            event_time_millis: None,
        };

        queue_inventory_reconciliation(&local_event, &local, &agent);
        queue_inventory_reconciliation(&local_event, &local, &agent);
        queue_inventory_reconciliation(&agent_event, &local, &agent);

        assert_eq!(local_receiver.try_recv(), Some(()));
        assert_eq!(local_receiver.try_recv(), None);
        assert_eq!(agent_receiver.try_recv(), Some(()));
    }

    #[tokio::test]
    async fn standalone_inventory_does_not_request_swarm_resources() {
        let runtime = CountingRuntime::default();
        let snapshot = collect_inventory(&runtime, &target("Docker"), &CancellationToken::new())
            .await
            .unwrap();

        assert!(snapshot.swarm.is_none());
        assert_eq!(runtime.common.load(Ordering::Relaxed), 5);
        assert_eq!(runtime.swarm.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn swarm_inventory_collects_each_bounded_resource_set_once() {
        let runtime = CountingRuntime::default();
        let snapshot =
            collect_inventory(&runtime, &target("DockerSwarm"), &CancellationToken::new())
                .await
                .unwrap();

        assert!(snapshot.swarm.is_some());
        assert_eq!(runtime.common.load(Ordering::Relaxed), 5);
        assert_eq!(runtime.swarm.load(Ordering::Relaxed), 5);
    }

    fn target(platform_type: &str) -> ReconciliationTarget {
        ReconciliationTarget {
            id: uuid::Uuid::now_v7(),
            address: "fixture".into(),
            connector_type: "Local".into(),
            platform_type: platform_type.into(),
        }
    }

    #[derive(Default)]
    struct CountingRuntime {
        common: AtomicUsize,
        swarm: AtomicUsize,
    }

    impl PlatformRuntimePort for CountingRuntime {
        fn get_info<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
            self.common.fetch_add(1, Ordering::Relaxed);
            Box::pin(async {
                Ok(RuntimePlatformInfo {
                    daemon_id: "fixture".into(),
                    server_version: "fixture".into(),
                    operating_system: "linux".into(),
                    os_type: "linux".into(),
                    architecture: "x86_64".into(),
                    cpu_count: 1,
                    memory_total: 1,
                    container_count: 0,
                    containers_running: 0,
                    containers_paused: 0,
                    containers_stopped: 0,
                    api_version: "1.49".into(),
                    minimum_api_version: "1.41".into(),
                    agent_version: None,
                })
            })
        }

        fn list_containers<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
            self.common.fetch_add(1, Ordering::Relaxed);
            Box::pin(async { Ok(Vec::new()) })
        }

        fn stream_stats<'a>(
            &'a self,
            _fetch_interval: Duration,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimeStatsStream, RuntimeCapabilityError>> {
            unreachable!()
        }
    }

    macro_rules! counted_list {
        ($name:ident, $type:ty, $counter:ident) => {
            fn $name<'a>(
                &'a self,
                _cancellation: &'a CancellationToken,
            ) -> BoxFuture<'a, Result<Vec<$type>, RuntimeCapabilityError>> {
                self.$counter.fetch_add(1, Ordering::Relaxed);
                Box::pin(async { Ok(Vec::new()) })
            }
        };
    }

    impl PlatformInventoryPort for CountingRuntime {
        counted_list!(list_images, RuntimeImageSummary, common);
        counted_list!(list_networks, RuntimeNetworkSummary, common);
        counted_list!(list_volumes, RuntimeVolumeSummary, common);
        counted_list!(list_swarm_nodes, RuntimeSwarmNode, swarm);
        counted_list!(list_swarm_services, RuntimeSwarmService, swarm);
        counted_list!(list_swarm_tasks, RuntimeSwarmTask, swarm);
        counted_list!(list_swarm_configs, RuntimeSwarmConfig, swarm);
        counted_list!(list_swarm_secrets, RuntimeSwarmSecret, swarm);

        fn inspect_network<'a>(
            &'a self,
            _id: &'a str,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimeNetworkSummary, RuntimeCapabilityError>> {
            unreachable!()
        }

        fn inspect_volume<'a>(
            &'a self,
            _name: &'a str,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
            unreachable!()
        }
    }
}
