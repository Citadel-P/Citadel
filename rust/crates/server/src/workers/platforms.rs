use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use citadel_adapters::agent::AgentClient;
use citadel_adapters::container_stats_store::PostgresContainerStatsStore;
use citadel_adapters::docker::{DockerClient, DockerError};
use citadel_adapters::inventory_projection_store::PostgresInventoryProjectionStore;
use citadel_alerts::{AlertDeliveryService, AlertEventSink, AlertObservation};
use citadel_application::{
    BoundedReceiver, BoundedSender, QueueOverflowPolicy, TaskSupervisor, bounded_channel,
};
use citadel_automation::AutomationService;
use citadel_backups::BackupService;
use citadel_builds::BuildService;
use citadel_deployments::DeploymentService;
use citadel_git::GitRepositoryExecutionService;
use citadel_platforms::jobs::{
    InventoryCollectionTarget, collect_inventory, collect_running_container_stats,
    triggers_inventory_reconciliation,
};
use citadel_platforms::{
    InventoryProjectionStore, PlatformInventoryPort, PlatformRuntimePort, RuntimeCapabilityError,
};
use citadel_stacks::StackService;
use citadel_swarm_services::ManagedSwarmServiceService;
use futures_util::StreamExt;
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
    pub volume_content: Arc<citadel_adapters::volume_content::VolumeContentAdapter>,
    pub containers: Arc<citadel_platforms::containers::ContainerMutationService>,
    pub docker: DockerClient,
    pub pool: PgPool,
    pub readiness: Arc<Readiness>,
    pub metrics: Arc<Metrics>,
    pub agent: Option<AgentClient>,
    pub realtime: Option<RealtimeHub>,
    pub deployments: Arc<DeploymentService>,
    pub swarm_services: Arc<ManagedSwarmServiceService>,
    pub stacks: Arc<StackService>,
    pub git: Arc<GitRepositoryExecutionService>,
    pub automation: Arc<AutomationService>,
    pub builds: Arc<BuildService>,
    pub backups: Arc<BackupService>,
    pub alerts: Arc<dyn AlertEventSink>,
    pub alert_deliveries: Arc<AlertDeliveryService>,
}

pub fn register(
    supervisor: &mut TaskSupervisor,
    cancellation: &CancellationToken,
    dependencies: WorkerDependencies,
    settings: WorkerSettings,
) {
    let WorkerDependencies {
        volume_content,
        containers,
        docker,
        pool,
        readiness,
        metrics,
        agent,
        realtime,
        deployments,
        swarm_services,
        stacks,
        git,
        automation,
        builds,
        backups,
        alerts,
        alert_deliveries,
    } = dependencies;
    supervisor.spawn(
        "volume-helper-recovery",
        super::volume_helpers::reconcile(cancellation.child_token(), volume_content),
    );
    supervisor.spawn(
        "container-operation-reconciliation",
        super::containers::reconcile(cancellation.child_token(), containers),
    );
    supervisor.spawn(
        "git-repository-sync",
        super::git::git_repository_sync(cancellation.child_token(), git),
    );
    supervisor.spawn(
        "automation-runs",
        super::automation::automation_runs(cancellation.child_token(), Arc::clone(&automation)),
    );
    supervisor.spawn(
        "automation-scheduler",
        super::automation::automation_scheduler(cancellation.child_token(), automation),
    );
    supervisor.spawn(
        "alert-deliveries",
        super::alerts::deliveries(cancellation.child_token(), alert_deliveries),
    );
    supervisor.spawn(
        "build-runs",
        super::builds::build_runs(cancellation.child_token(), Arc::clone(&builds)),
    );
    supervisor.spawn(
        "build-consumers",
        super::builds::build_consumers(
            cancellation.child_token(),
            citadel_builds::BuildCompletionService::new(
                Arc::new(
                    citadel_adapters::build_completion_store::PostgresBuildCompletionStore::new(
                        pool.clone(),
                    ),
                ),
                Arc::new(super::build_consumers::BuildConsumers {
                    deployments: deployments.clone(),
                    stacks: stacks.clone(),
                    realtime: realtime.clone(),
                }),
                builds.clone(),
            ),
        ),
    );
    supervisor.spawn(
        "build-pool-health",
        super::builds::pool_health(cancellation.child_token(), builds),
    );
    supervisor.spawn(
        "backup-runs",
        super::backups::backup_runs(cancellation.child_token(), Arc::clone(&backups)),
    );
    supervisor.spawn(
        "backup-restore-runs",
        super::backups::restore_runs(cancellation.child_token(), Arc::clone(&backups)),
    );
    supervisor.spawn(
        "backup-policy-scheduler",
        super::backups::policy_scheduler(cancellation.child_token(), backups),
    );
    supervisor.spawn(
        "deployment-apply-reconciliation",
        super::deployments::deployment_apply_reconciliation(
            cancellation.child_token(),
            Arc::clone(&deployments),
        ),
    );
    supervisor.spawn(
        "deployment-image-updates",
        super::deployments::image_updates(cancellation.child_token(), deployments),
    );
    supervisor.spawn(
        "swarm-service-operation-reconciliation",
        super::swarm_services::swarm_service_operation_reconciliation(
            cancellation.child_token(),
            Arc::clone(&swarm_services),
        ),
    );
    supervisor.spawn(
        "swarm-service-image-updates",
        super::swarm_services::swarm_service_image_updates(
            cancellation.child_token(),
            swarm_services,
        ),
    );
    supervisor.spawn(
        "stack-operation-reconciliation",
        super::stacks::stack_operation_reconciliation(
            cancellation.child_token(),
            Arc::clone(&stacks),
        ),
    );
    supervisor.spawn(
        "stack-drift-monitor",
        super::stacks::stack_drift_monitor(cancellation.child_token(), Arc::clone(&stacks)),
    );
    supervisor.spawn(
        "stack-webhooks",
        super::stacks::stack_webhooks(cancellation.child_token(), stacks.clone()),
    );
    supervisor.spawn(
        "stack-updates",
        super::stacks::stack_updates(cancellation.child_token(), stacks.clone()),
    );
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
                pool.clone(),
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
                alerts: Arc::clone(&alerts),
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
        let context = StatsWorkerContext {
            pool: pool.clone(),
            metrics: Arc::clone(&metrics),
            realtime: realtime.clone(),
            alerts: Arc::clone(&alerts),
            fetch_interval: settings.probe_interval,
        };
        supervisor.spawn(
            "agent-container-stats",
            agent_container_stats(
                cancellation.child_token(),
                agent,
                context,
                settings.agent_reconnect_delay,
            ),
        );
    }
    supervisor.spawn(
        "local-container-stats",
        local_container_stats(
            cancellation.child_token(),
            docker,
            StatsWorkerContext {
                pool,
                metrics,
                realtime,
                alerts,
                fetch_interval: settings.probe_interval,
            },
        ),
    );
}

#[derive(Clone)]
struct StatsWorkerContext {
    pool: PgPool,
    metrics: Arc<Metrics>,
    realtime: Option<RealtimeHub>,
    alerts: Arc<dyn AlertEventSink>,
    fetch_interval: Duration,
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
    name: String,
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
    alerts: Arc<dyn AlertEventSink>,
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

        if let Err(error) = reconcile_inventory(&cancellation, &worker, &store, scope).await {
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
    worker: &InventoryReconciliationWorker,
    store: &dyn InventoryProjectionStore,
    scope: Option<ReconciliationTrigger>,
) -> Result<(), RuntimeCapabilityError> {
    let targets = reconciliation_targets(&worker.pool).await?;
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
        let selected_agent;
        let runtime: &dyn PlatformInventoryPort =
            if target.connector_type.eq_ignore_ascii_case("Local") {
                &worker.docker
            } else if target.connector_type.eq_ignore_ascii_case("Agent") {
                let Some(agent) = worker.agent.as_ref() else {
                    continue;
                };
                selected_agent = agent.at_address(&target.address)?;
                &selected_agent
            } else {
                // Edge Agents use an inbound command session. That resolver is registered
                // when the Edge transport owns a live session; disconnected targets keep
                // their last bounded projection instead of being overwritten.
                continue;
            };

        match collect_inventory(
            runtime,
            &InventoryCollectionTarget {
                platform_id: target.id,
                platform_type: target.platform_type.clone(),
            },
            cancellation,
        )
        .await
        {
            Ok(snapshot) => {
                let change = store.persist(&snapshot).await?;
                let observation = platform_reachable_observation(&target);
                if let Err(alert_error) = worker.alerts.observe(&observation).await {
                    tracing::error!(
                        %alert_error,
                        platform_id = %target.id,
                        "platform recovery Alert evaluation failed"
                    );
                }
                if let Some(realtime) = worker.realtime.as_ref() {
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
                let observation = platform_unreachable_observation(&target, &error);
                if let Err(alert_error) = worker.alerts.observe(&observation).await {
                    tracing::error!(
                        %alert_error,
                        platform_id = %target.id,
                        "platform failure Alert evaluation failed"
                    );
                }
            }
        }
    }
    Ok(())
}

fn platform_reachable_observation(target: &ReconciliationTarget) -> AlertObservation {
    AlertObservation {
        alert_type: "PlatformUnreachable".to_owned(),
        info: serde_json::json!({
            "PlatformName": target.name,
            "Id": target.id,
            "Address": target.address,
            "HumanMessage": format!("Platform '{}' is reachable.", target.name),
        }),
        resource_id: target.id,
        resource_name: target.name.clone(),
        resource_type: "Platform".to_owned(),
        deduplication_component: "inventory".to_owned(),
        observed_at: chrono::Utc::now(),
        value: None,
        matched: false,
    }
}

fn platform_unreachable_observation(
    target: &ReconciliationTarget,
    error: &RuntimeCapabilityError,
) -> AlertObservation {
    let message = format!(
        "Could not synchronize platform '{}': {}",
        target.name, error
    );
    AlertObservation {
        alert_type: "PlatformUnreachable".to_owned(),
        info: serde_json::json!({
            "PlatformName": target.name,
            "Id": target.id,
            "Address": target.address,
            "ErrorMessage": error.to_string(),
            "HumanMessage": message,
        }),
        resource_id: target.id,
        resource_name: target.name.clone(),
        resource_type: "Platform".to_owned(),
        deduplication_component: "inventory".to_owned(),
        observed_at: chrono::Utc::now(),
        value: None,
        matched: true,
    }
}

async fn reconciliation_targets(
    pool: &PgPool,
) -> Result<Vec<ReconciliationTarget>, RuntimeCapabilityError> {
    let rows = sqlx::query(
        r#"
SELECT id, name, address, connectortype,
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
                name: row.try_get("name").map_err(worker_storage)?,
                address: row.try_get("address").map_err(worker_storage)?,
                connector_type: row.try_get("connectortype").map_err(worker_storage)?,
                platform_type: row.try_get("platformtype").map_err(worker_storage)?,
            })
        })
        .collect()
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
    context: StatsWorkerContext,
) -> Result<(), std::convert::Infallible> {
    let _task = context.metrics.task_guard();
    let store = PostgresContainerStatsStore::new(context.pool.clone());
    let mut ticker = tokio::time::interval(context.fetch_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {}
        }
        let platform_id = match local_platform_id(&context.pool).await {
            Ok(Some(id)) => id,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(%error, "local Platform lookup for statistics failed");
                continue;
            }
        };
        let batch = match collect_running_container_stats(&docker, &cancellation).await {
            Ok(batch) => batch,
            Err(error) => {
                if cancellation.is_cancelled() {
                    return Ok(());
                }
                tracing::warn!(%error, "local container statistics collection failed");
                continue;
            }
        };
        if batch.failed_samples != 0 {
            tracing::warn!(
                failed_samples = batch.failed_samples,
                %platform_id,
                "some local container statistics samples failed"
            );
        }
        if batch.stats.is_empty() && batch.failed_samples != 0 {
            continue;
        }
        let stats = batch.stats;
        let disk = tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            disk = docker.host_disk_usage() => disk,
        };
        match store.persist_with_disk(platform_id, &stats, disk).await {
            Ok(0) if !stats.is_empty() => continue,
            Ok(_) => {
                context.metrics.local_stats_sampled();
                observe_platform_metrics(&context.pool, context.alerts.as_ref(), platform_id).await;
                if let Some(realtime) = context.realtime.as_ref() {
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
    context: StatsWorkerContext,
    reconnect_delay: Duration,
) -> Result<(), RuntimeCapabilityError> {
    let _task = context.metrics.task_guard();
    let store = PostgresContainerStatsStore::new(context.pool.clone());
    loop {
        let (platform_id, agent) = match agent_subscription(&context.pool, &agent).await {
            Ok(Some(target)) => target,
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
        if let Err(error) = agent.get_info(&cancellation).await {
            if cancellation.is_cancelled() {
                return Ok(());
            }
            context.metrics.agent_handshake_failed();
            if !error.retryable {
                return Err(error);
            }
            tracing::warn!(%error, "Agent handshake failed");
            if wait_to_reconnect(&cancellation, reconnect_delay).await {
                return Ok(());
            }
            continue;
        }
        let mut stream = match agent
            .stream_container_stats(context.fetch_interval, &cancellation)
            .await
        {
            Ok(stream) => stream,
            Err(_error) if cancellation.is_cancelled() => return Ok(()),
            Err(error) if error.retryable => {
                context.metrics.agent_stream_reconnected();
                tracing::warn!(%error, "Agent stats stream connection failed");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
            Err(error) => return Err(error),
        };
        let mut disk = super::disk::LatestDisk::new(context.fetch_interval);
        let mut disk_updates =
            super::disk::samples(agent.clone(), context.fetch_interval, cancellation.clone());
        let changed = wait_for_agent_reconfiguration(&context.pool, platform_id, agent.address());
        tokio::pin!(changed);
        loop {
            let next = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                () = &mut changed => break,
                update = disk_updates.next() => { disk.record(update.flatten()); continue; },
                next = stream.next() => next,
            };
            match next {
                Some(Ok(stats)) => {
                    match store
                        .persist_with_disk(platform_id, &stats, disk.get())
                        .await
                    {
                        Ok(0) if !stats.is_empty() => continue,
                        Ok(_) => {
                            context.metrics.agent_stats_sampled();
                            observe_platform_metrics(
                                &context.pool,
                                context.alerts.as_ref(),
                                platform_id,
                            )
                            .await;
                            if let Some(realtime) = context.realtime.as_ref() {
                                realtime.publish_container_stats(platform_id, &stats);
                            }
                        }
                        Err(error) => {
                            tracing::warn!(%error, %platform_id, "Agent container statistics persistence failed");
                        }
                    }
                }
                Some(Err(error)) if error.retryable => {
                    context.metrics.agent_stream_reconnected();
                    tracing::warn!(%error, "Agent stats stream interrupted");
                    break;
                }
                Some(Err(error)) => return Err(error),
                None if cancellation.is_cancelled() => return Ok(()),
                None => {
                    context.metrics.agent_stream_reconnected();
                    break;
                }
            }
        }
        if wait_to_reconnect(&cancellation, reconnect_delay).await {
            return Ok(());
        }
    }
}

pub(super) async fn observe_platform_metrics(
    pool: &PgPool,
    alerts: &dyn AlertEventSink,
    platform_id: uuid::Uuid,
) {
    let row = match sqlx::query(
        "SELECT p.name,s.cpuusage,s.memoryusage,s.diskusage,s.diskusedbytes,s.disktotalbytes FROM platforms p JOIN LATERAL (SELECT * FROM platformstats WHERE platformid=p.id ORDER BY created DESC LIMIT 1) s ON true WHERE p.id=$1",
    )
    .bind(platform_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return,
        Err(error) => {
            tracing::warn!(%error, %platform_id, "Platform Alert metric lookup failed");
            return;
        }
    };
    let name = match row.try_get::<String, _>("name") {
        Ok(name) => name,
        Err(error) => {
            tracing::warn!(%error, %platform_id, "Platform Alert metric mapping failed");
            return;
        }
    };
    let disk = (|| {
        citadel_platforms::HostDiskUsage::new(
            row.try_get::<Option<i64>, _>("diskusedbytes").ok()??,
            row.try_get::<Option<i64>, _>("disktotalbytes").ok()??,
            row.try_get::<Option<f64>, _>("diskusage").ok()??,
        )
    })();
    let values = [
        (
            "PlatformCpuHigh",
            "CPU",
            row.try_get::<f64, _>("cpuusage").ok(),
        ),
        (
            "PlatformRamHigh",
            "RAM",
            row.try_get::<f64, _>("memoryusage").ok(),
        ),
        (
            "PlatformDiskHigh",
            "disk",
            disk.map(|sample| sample.usage_percent),
        ),
    ];
    for (alert_type, label, value) in values {
        let Some(value) = value.filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
        else {
            continue;
        };
        let observation = AlertObservation {
            alert_type: alert_type.to_owned(),
            info: serde_json::json!({
                "PlatformName": name,
                "Value": value,
                "DiskUsagePercent": if alert_type == "PlatformDiskHigh" { Some(value) } else { None },
                "DiskUsedBytes": row.try_get::<Option<i64>, _>("diskusedbytes").ok().flatten(),
                "DiskTotalBytes": row.try_get::<Option<i64>, _>("disktotalbytes").ok().flatten(),
                "HumanMessage": format!("Platform '{name}' {label} usage is {value:.1}%."),
            }),
            resource_id: platform_id,
            resource_name: name.clone(),
            resource_type: "Platform".to_owned(),
            deduplication_component: "utilization".to_owned(),
            observed_at: chrono::Utc::now(),
            value: Some(value),
            matched: true,
        };
        if let Err(error) = alerts.observe(&observation).await {
            tracing::warn!(%error, %platform_id, alert_type, "Platform Alert evaluation failed");
        }
    }
}

async fn local_platform_id(pool: &PgPool) -> Result<Option<uuid::Uuid>, RuntimeCapabilityError> {
    sqlx::query_scalar("SELECT id FROM platforms WHERE connectortype = 'Local' ORDER BY id LIMIT 1")
        .fetch_optional(pool)
        .await
        .map_err(worker_storage)
}

async fn agent_subscription(
    pool: &PgPool,
    base: &AgentClient,
) -> Result<Option<(uuid::Uuid, AgentClient)>, RuntimeCapabilityError> {
    // This worker owns the existing direct-Agent subscription. Resolve its
    // persisted address instead of permanently pinning it to startup settings.
    let target: Option<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id,address FROM platforms WHERE connectortype = 'Agent' ORDER BY (rtrim(address, '/') = rtrim($1, '/')) DESC,id LIMIT 1",
    )
    .bind(base.address())
    .fetch_optional(pool)
    .await
    .map_err(worker_storage)?;
    target
        .map(|(id, address)| base.at_address(&address).map(|client| (id, client)))
        .transpose()
}

async fn wait_for_agent_reconfiguration(pool: &PgPool, id: uuid::Uuid, address: &str) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let unchanged: Result<bool, _> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM platforms WHERE id=$1 AND connectortype='Agent' AND rtrim(address,'/')=rtrim($2,'/'))",
        ).bind(id).bind(address).fetch_one(pool).await;
        match unchanged {
            Ok(false) => return,
            Ok(true) => {}
            Err(error) => tracing::warn!(%error, "Agent subscription configuration lookup failed"),
        }
    }
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
    pool: PgPool,
    sender: BoundedSender<InventoryEvent>,
    metrics: Arc<Metrics>,
    reconnect_delay: Duration,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    loop {
        let (platform_id, agent) = match agent_subscription(&pool, &agent).await {
            Ok(Some(target)) => target,
            result => {
                if let Err(error) = result {
                    tracing::warn!(%error, "Agent Platform lookup for events failed");
                }
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
        };
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
        let changed = wait_for_agent_reconfiguration(&pool, platform_id, agent.address());
        tokio::pin!(changed);
        loop {
            let event = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                () = &mut changed => break,
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

    #[test]
    fn platform_inventory_failure_maps_to_the_existing_alert_contract() {
        let id = uuid::Uuid::now_v7();
        let target = ReconciliationTarget {
            id,
            name: "worker-01".into(),
            address: "http://agent:8080".into(),
            connector_type: "Agent".into(),
            platform_type: "Docker".into(),
        };
        let error = RuntimeCapabilityError::new(
            citadel_platforms::RuntimeErrorKind::Remote,
            "connection refused",
            true,
        );

        let observation = platform_unreachable_observation(&target, &error);

        assert_eq!(observation.alert_type, "PlatformUnreachable");
        assert_eq!(observation.resource_id, id);
        assert_eq!(observation.resource_type, "Platform");
        assert_eq!(observation.info["PlatformName"], "worker-01");
        assert_eq!(observation.info["Address"], "http://agent:8080");
        assert_eq!(observation.deduplication_component, "inventory");
        assert!(observation.matched);

        let recovered = platform_reachable_observation(&target);
        assert_eq!(recovered.alert_type, observation.alert_type);
        assert_eq!(recovered.deduplication_component, "inventory");
        assert!(!recovered.matched);
    }
}
