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
    pub node_agent_policy:
        citadel_adapters::node_agent_reconciliation::NodeAgentReconciliationPolicy,
    pub stats_flush_interval: Duration,
    pub stats_batch_size: usize,
    pub build_parallel_runs: usize,
    pub build_retention_days: Option<i32>,
    pub backup_workers: crate::config::BackupWorkerConfig,
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

pub async fn register(
    supervisor: &mut TaskSupervisor,
    cancellation: &CancellationToken,
    dependencies: WorkerDependencies,
    settings: WorkerSettings,
) -> Result<(), sqlx::Error> {
    // Subscribe before starting inventory/event producers, so initial discovery
    // cannot race past the alert/drift/pruning consumers.
    let unmanaged_listener =
        super::listener(&dependencies.pool, "citadel_container_created").await?;
    let drift_listener = super::listener(&dependencies.pool, "citadel_stack_drift").await?;
    let pruning_listener = super::listener(&dependencies.pool, "citadel_swarm_prune").await?;
    let job_alert_listener = super::listener(&dependencies.pool, "citadel_job_alerts").await?;
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
        "job-alert-observations",
        super::alerts::observations(
            cancellation.child_token(),
            alerts.clone(),
            job_alert_listener,
        ),
    );
    supervisor.spawn(
        "volume-helper-recovery",
        super::volume_helpers::reconcile(cancellation.child_token(), volume_content),
    );
    supervisor.spawn(
        "container-operation-reconciliation",
        super::containers::reconcile(cancellation.child_token(), containers.clone()),
    );
    supervisor.spawn(
        "swarm-task-pruning",
        super::pruning::run(
            cancellation.child_token(),
            pool.clone(),
            containers,
            pruning_listener,
        ),
    );
    supervisor.spawn(
        "stack-event-drift",
        super::stacks::event_drift(cancellation.child_token(), stacks.clone(), drift_listener),
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
        "platform-threshold-alerts",
        super::stats_alerts::run(
            cancellation.child_token(),
            pool.clone(),
            alerts.clone(),
            settings.stats_flush_interval,
            i64::try_from(settings.stats_batch_size).unwrap_or(i64::MAX),
        ),
    );
    supervisor.spawn(
        "alert-deliveries",
        super::alerts::deliveries(cancellation.child_token(), alert_deliveries),
    );
    supervisor.spawn(
        "build-runs",
        super::builds::build_runs(
            cancellation.child_token(),
            Arc::clone(&builds),
            settings.build_parallel_runs,
        ),
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
        super::backups::backup_runs(
            cancellation.child_token(),
            Arc::clone(&backups),
            settings.backup_workers.clone(),
        ),
    );
    supervisor.spawn(
        "backup-restore-runs",
        super::backups::restore_runs(
            cancellation.child_token(),
            Arc::clone(&backups),
            settings.backup_workers.clone(),
        ),
    );
    supervisor.spawn(
        "backup-policy-scheduler",
        super::backups::policy_scheduler(
            cancellation.child_token(),
            backups,
            settings.backup_workers.clone(),
        ),
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
    supervisor.spawn(
        "unmanaged-container-alerts",
        super::unmanaged::run(
            cancellation.child_token(),
            pool.clone(),
            alerts.clone(),
            unmanaged_listener,
        ),
    );
    supervisor.spawn(
        "maintenance",
        super::maintenance::run(
            cancellation.child_token(),
            pool.clone(),
            realtime.clone(),
            settings.build_retention_days,
        ),
    );
    let (sender, receiver) = bounded_channel(settings.queue_capacity, QueueOverflowPolicy::Wait);
    let (local_reconcile_sender, local_reconcile_receiver) =
        bounded_channel(1, QueueOverflowPolicy::Reject);
    let (agent_reconcile_sender, agent_reconcile_receiver) =
        bounded_channel(256, QueueOverflowPolicy::Reject);

    supervisor.spawn(
        "platform-resource-health",
        resource_health(
            cancellation.child_token(),
            docker.clone(),
            agent.clone(),
            pool.clone(),
            realtime.clone(),
            local_reconcile_sender.clone(),
            agent_reconcile_sender.clone(),
            alerts.clone(),
        ),
    );
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
            "agent-subscriptions",
            agent_subscriptions(
                cancellation.child_token(),
                agent,
                sender,
                StatsWorkerContext {
                    pool: pool.clone(),
                    metrics: metrics.clone(),
                    realtime: realtime.clone(),
                    fetch_interval: settings.probe_interval,
                },
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
            docker.clone(),
            pool.clone(),
            realtime.clone(),
            local_reconcile_sender,
            agent_reconcile_sender,
        ),
    );
    supervisor.spawn(
        "platform-inventory-reconciliation",
        inventory_reconciliation(
            cancellation.child_token(),
            InventoryReconciliationWorker {
                node_agent_policy: settings.node_agent_policy.clone(),
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
    supervisor.spawn(
        "local-container-stats",
        local_container_stats(
            cancellation.child_token(),
            docker,
            StatsWorkerContext {
                pool,
                metrics,
                realtime,
                fetch_interval: settings.probe_interval,
            },
        ),
    );
    Ok(())
}

#[derive(Clone)]
struct StatsWorkerContext {
    pool: PgPool,
    metrics: Arc<Metrics>,
    realtime: Option<RealtimeHub>,
    fetch_interval: Duration,
}

#[derive(Debug, Clone, Copy)]
enum ReconciliationTrigger {
    LocalEvent,
    AgentEvent,
}

#[derive(Debug)]
struct InventoryEvent {
    platform_id: Option<uuid::Uuid>,
    container_id: Option<String>,
    container_state: Option<String>,
    container_name: Option<String>,
    container: Option<citadel_platforms::RuntimeContainerSummary>,
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
    node_agent_policy: citadel_adapters::node_agent_reconciliation::NodeAgentReconciliationPolicy,
    docker: DockerClient,
    agent: Option<AgentClient>,
    pool: PgPool,
    local_triggers: BoundedReceiver<()>,
    agent_triggers: BoundedReceiver<Option<uuid::Uuid>>,
    realtime: Option<RealtimeHub>,
    alerts: Arc<dyn AlertEventSink>,
    interval: Duration,
    retry_delay: Duration,
}

async fn inventory_reconciliation(
    cancellation: CancellationToken,
    mut worker: InventoryReconciliationWorker,
) -> Result<(), std::convert::Infallible> {
    let store = PostgresInventoryProjectionStore::new(worker.pool.clone())
        .with_node_policy(worker.node_agent_policy.clone());
    let mut ticker = tokio::time::interval_at(
        tokio::time::Instant::now() + worker.interval,
        worker.interval,
    );
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut scope = None;
    let mut agent_ids = std::collections::BTreeSet::new();
    let mut all_agents = false;

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
                while let Some(id) = worker.agent_triggers.try_recv() {
                    if let Some(id) = id {
                        agent_ids.insert(id);
                    } else {
                        all_agents = true;
                    }
                }
            }
        }

        if let Err(error) = reconcile_inventory(
            &cancellation,
            &worker,
            &store,
            scope,
            if all_agents { None } else { Some(&agent_ids) },
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
                agent_ids.clear();
                all_agents = trigger.flatten().is_none();
                if let Some(id)=trigger.flatten() {agent_ids.insert(id);}
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
    agent_ids: Option<&std::collections::BTreeSet<uuid::Uuid>>,
) -> Result<(), RuntimeCapabilityError> {
    let targets = reconciliation_targets(&worker.pool).await?;
    let mut refreshes = futures_util::stream::iter(targets).map(|target| async move {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        match scope {
            Some(ReconciliationTrigger::LocalEvent)
                if !target.connector_type.eq_ignore_ascii_case("Local") =>
            {
                return Ok(());
            }
            Some(ReconciliationTrigger::AgentEvent)
                if !target.connector_type.eq_ignore_ascii_case("Agent") || agent_ids.is_some_and(|ids| !ids.contains(&target.id)) =>
            {
                return Ok(());
            }
            _ => {}
        }
        let selected_agent;
        let runtime: &dyn PlatformInventoryPort =
            if target.connector_type.eq_ignore_ascii_case("Local") {
                &worker.docker
            } else if target.connector_type.eq_ignore_ascii_case("Agent") {
                let Some(agent) = worker.agent.as_ref() else {
                    return Ok(());
                };
                selected_agent = agent.at_address(&target.address)?;
                &selected_agent
            } else {
                // Edge Agents use an inbound command session. That resolver is registered
                // when the Edge transport owns a live session; disconnected targets keep
                // their last bounded projection instead of being overwritten.
                return Ok(());
            };

        let started_at = chrono::Utc::now();
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
                let change = match store.persist(&snapshot).await {
                    Ok(change) => change,
                    Err(error) => {
                        tracing::warn!(%error, platform_id=%target.id, "Platform inventory rejected");
                        if error.kind == citadel_platforms::RuntimeErrorKind::Conflict {
                            if let Err(error) =
                                citadel_adapters::resource_status_store::platform_offline(
                                    &worker.pool,
                                    target.id,
                                )
                                .await
                            {
                                tracing::warn!(%error, "Rejected Platform status update failed");
                            }
                        }
                        return Ok(());
                    }
                };
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
                if target.platform_type.eq_ignore_ascii_case("DockerSwarm") {
                    PostgresInventoryProjectionStore::new(worker.pool.clone())
                        .mark_swarm_stale(target.id, started_at).await?;
                }
                // Health monitoring owns availability transitions and alerts. An
                // inventory error must not bypass its failure/success thresholds.
            }
        }
        Ok::<(), RuntimeCapabilityError>(())
    }).buffer_unordered(8);
    while let Some(result) = refreshes.next().await {
        if let Err(error) = result {
            tracing::warn!(%error, "platform refresh failed; continuing remaining targets");
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
            sample = docker.platform_stats() => sample.ok(),
        };
        match persist_stats_retry(&store, platform_id, &stats, disk.as_ref(), &cancellation).await {
            Ok(0) if !stats.is_empty() => continue,
            Ok(_) => {
                context.metrics.local_stats_sampled();
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
    platform: uuid::Uuid,
    agent: AgentClient,
    context: StatsWorkerContext,
    reconnect_delay: Duration,
) -> Result<(), RuntimeCapabilityError> {
    let _task = context.metrics.task_guard();
    let store = PostgresContainerStatsStore::new(context.pool.clone());
    loop {
        let (platform_id, agent) = match agent_subscription(&context.pool, &agent, platform).await {
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
            Err(error) => {
                context.metrics.agent_stream_reconnected();
                tracing::warn!(%error, "Agent stats stream connection failed");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
        };
        let mut disk = super::disk::LatestPlatformStats::new(context.fetch_interval);
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
                    match persist_stats_retry(
                        &store,
                        platform_id,
                        &stats,
                        disk.get(),
                        &cancellation,
                    )
                    .await
                    {
                        Ok(0) if !stats.is_empty() => continue,
                        Ok(_) => {
                            context.metrics.agent_stats_sampled();
                            if let Some(realtime) = context.realtime.as_ref() {
                                realtime.publish_container_stats(platform_id, &stats);
                            }
                        }
                        Err(error) => {
                            tracing::warn!(%error, %platform_id, "Agent container statistics persistence failed");
                        }
                    }
                }
                Some(Err(error)) => {
                    context.metrics.agent_stream_reconnected();
                    tracing::warn!(%error, "Agent stats stream interrupted");
                    break;
                }
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

pub(super) async fn persist_stats_retry(
    store: &PostgresContainerStatsStore,
    platform: uuid::Uuid,
    stats: &[citadel_platforms::RuntimeContainerStat],
    disk: Option<&citadel_platforms::RuntimePlatformStats>,
    cancel: &CancellationToken,
) -> Result<usize, RuntimeCapabilityError> {
    let mut delay = Duration::from_secs(2);
    loop {
        let result = tokio::select! {
            ()=cancel.cancelled()=>return Err(RuntimeCapabilityError::new(citadel_platforms::RuntimeErrorKind::Cancelled,"Statistics writer stopped",false)),
            result=store.persist_with_platform_stats(platform,stats,disk)=>result,
        };
        match result {
            Ok(inserted) => return Ok(inserted),
            Err(error) if !error.retryable => return Err(error),
            Err(error) => {
                tracing::warn!(%error,%platform,"Statistics persistence failed; retaining the batch for retry")
            }
        }
        tokio::select! {
            ()=cancel.cancelled()=>return Err(RuntimeCapabilityError::new(citadel_platforms::RuntimeErrorKind::Cancelled,"Statistics writer stopped",false)),
            _=tokio::time::sleep(delay)=>{},
        }
        delay = (delay * 2).min(Duration::from_secs(30));
    }
}

#[cfg(test)]
pub(super) async fn observe_platform_metrics(
    pool: &PgPool,
    alerts: &dyn AlertEventSink,
    platform_id: uuid::Uuid,
) {
    let row = match sqlx::query(
        "SELECT p.name,p.agentversion,s.cpuusage,s.memoryusage,s.diskusage,s.diskusedbytes,s.disktotalbytes FROM platforms p JOIN LATERAL (SELECT * FROM platformstats WHERE platformid=p.id ORDER BY created DESC LIMIT 1) s ON true WHERE p.id=$1",
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
    let _ = observe_metric_row(alerts, platform_id, row).await;
}

pub(super) async fn observe_metric_row(
    alerts: &dyn AlertEventSink,
    platform_id: uuid::Uuid,
    row: sqlx::postgres::PgRow,
) -> Result<(), citadel_alerts::AlertError> {
    let name = match row.try_get::<String, _>("name") {
        Ok(name) => name,
        Err(error) => {
            tracing::warn!(%error, %platform_id, "Platform Alert metric mapping failed");
            return Err(citadel_alerts::AlertError::Storage(error.to_string()));
        }
    };
    let version = row
        .try_get::<Option<String>, _>("agentversion")
        .ok()
        .flatten()
        .unwrap_or_default();
    let compatibility = env!("CITADEL_BUILD_VERSION")
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
    if !version.is_empty() {
        let compatible = version
            .strip_prefix(&compatibility)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '-', '+']));
        let observation = AlertObservation {
            alert_type: "PlatformVersionMismatch".into(),
            info: serde_json::json!({"PlatformName":name,"AgentVersion":version,"CoreVersion":compatibility,"HumanMessage":format!("Platform '{name}' Agent version {version} differs from Core compatibility version {compatibility}.")}),
            resource_id: platform_id,
            resource_name: name.clone(),
            resource_type: "Platform".into(),
            deduplication_component: "version".into(),
            observed_at: chrono::Utc::now(),
            value: None,
            matched: !compatible,
        };
        alerts.observe(&observation).await?;
    }
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
        alerts.observe(&observation).await?;
    }
    Ok(())
}

async fn local_platform_id(pool: &PgPool) -> Result<Option<uuid::Uuid>, RuntimeCapabilityError> {
    sqlx::query_scalar("SELECT id FROM platforms WHERE connectortype = 'Local' ORDER BY id LIMIT 1")
        .fetch_optional(pool)
        .await
        .map_err(worker_storage)
}

// Own one pair of independently reconnecting streams per persisted Direct Agent.
async fn agent_subscriptions(
    cancellation: CancellationToken,
    base: AgentClient,
    sender: BoundedSender<InventoryEvent>,
    context: StatsWorkerContext,
    reconnect_delay: Duration,
) -> Result<(), std::convert::Infallible> {
    let mut active =
        std::collections::HashMap::<uuid::Uuid, (String, CancellationToken, tokio::task::Id)>::new(
        );
    let mut tasks = tokio::task::JoinSet::new();
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => break,
            result = tasks.join_next_with_id(), if !tasks.is_empty() => {
                let task_id = match result {
                    Some(Ok((id, ())))=>Some(id),
                    Some(Err(error))=>{tracing::warn!(%error,"Agent subscription task failed");Some(error.id())},
                    None=>None,
                };
                if let Some(platform) = active.iter().find(|(_,(_,_,id))|Some(*id)==task_id).map(|(platform,_)|*platform) {
                    if let Some((_,token,_))=active.remove(&platform) { token.cancel(); }
                }
            }
            _ = ticker.tick() => {
                let targets: Vec<(uuid::Uuid, String)> = match sqlx::query_as(
                    "SELECT id,address FROM platforms WHERE connectortype='Agent'")
                    .fetch_all(&context.pool).await {
                        Ok(targets) => targets,
                        Err(error) => { tracing::warn!(%error, "Agent subscription lookup failed"); continue; }
                    };
                active.retain(|id, (address, token, _)| {
                    let keep = targets.iter().any(|(target, endpoint)| target == id && endpoint == address);
                    if !keep { token.cancel(); }
                    keep
                });
                for (id, address) in targets {
                    if active.contains_key(&id) { continue; }
                    let token = cancellation.child_token();
                    let owned_token = token.clone();
                    let base = base.clone();
                    let context = context.clone();
                    let sender = sender.clone();
                    let task = tasks.spawn(async move {
                        // Endpoint failures stay inside this platform's retry loop.
                        loop {
                            let events = agent_event_source(token.clone(), id, base.clone(), context.pool.clone(), sender.clone(), context.metrics.clone(), reconnect_delay);
                            let stats = agent_container_stats(token.clone(), id, base.clone(), context.clone(), reconnect_delay);
                            let (_, result) = tokio::join!(events, stats);
                            if let Err(error) = result { tracing::warn!(%error, %id, "Agent subscription failed"); }
                            if wait_to_reconnect(&token, reconnect_delay).await { break; }
                        }
                    });
                    active.insert(id, (address, owned_token, task.id()));
                }
            }
        }
    }
    for (_, token, _) in active.values() {
        token.cancel();
    }
    while tasks.join_next().await.is_some() {}
    Ok(())
}

async fn agent_subscription(
    pool: &PgPool,
    base: &AgentClient,
    platform: uuid::Uuid,
) -> Result<Option<(uuid::Uuid, AgentClient)>, RuntimeCapabilityError> {
    // This worker owns the existing direct-Agent subscription. Resolve its
    // persisted address instead of permanently pinning it to startup settings.
    let target: Option<(uuid::Uuid, String)> =
        sqlx::query_as("SELECT id,address FROM platforms WHERE connectortype = 'Agent' AND id=$1")
            .bind(platform)
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
                        platform_id: None,
                        container_id: Some(event.actor.id),
                        container_state: None,
                        container_name: None,
                        container: None,
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
    platform: uuid::Uuid,
    agent: AgentClient,
    pool: PgPool,
    sender: BoundedSender<InventoryEvent>,
    metrics: Arc<Metrics>,
    reconnect_delay: Duration,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    loop {
        let (platform_id, agent) = match agent_subscription(&pool, &agent, platform).await {
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
                                platform_id: Some(platform_id),
                                container_id: event.container_id,
                                container_state: event.container_state,
                                container_name: event.container_name,
                                container: event.container,
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

#[derive(Default)]
struct HealthState {
    online: Option<bool>,
    successes: u8,
    failures: u8,
}
impl HealthState {
    fn observe(&mut self, healthy: bool) -> Option<bool> {
        if healthy {
            self.failures = 0;
            self.successes = self.successes.saturating_add(1);
        } else {
            self.successes = 0;
            self.failures = self.failures.saturating_add(1);
        }
        let confirmed = if healthy {
            self.successes >= 2
        } else {
            self.failures >= 3
        };
        if confirmed && self.online != Some(healthy) {
            self.online = Some(healthy);
            Some(healthy)
        } else {
            None
        }
    }
}

async fn probe_health(
    runtime: &dyn PlatformRuntimePort,
    token: &CancellationToken,
    timeout: Duration,
) -> bool {
    matches!(
        tokio::time::timeout(timeout, runtime.get_info(token)).await,
        Ok(Ok(_))
    )
}

async fn edge_health(pool: &PgPool, platform: uuid::Uuid) -> Result<Option<bool>, sqlx::Error> {
    // No binding (pending enrollment) and revoked bindings must not raise an
    // unreachable alert. Node bindings do not determine the manager's health.
    sqlx::query_scalar("SELECT COALESCE(connectionstatus='Connected' AND lastheartbeatatutc>CURRENT_TIMESTAMP-INTERVAL '90 seconds',false) FROM edgeagentbindings WHERE platformid=$1 AND resourcetype='Platform' AND dockernodeid IS NULL AND revokedatutc IS NULL AND connectionstatus<>'Revoked'")
        .bind(platform).fetch_optional(pool).await
}

async fn resource_health(
    cancellation: CancellationToken,
    docker: DockerClient,
    agent: Option<AgentClient>,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    local: BoundedSender<()>,
    agent_trigger: BoundedSender<Option<uuid::Uuid>>,
    alerts: Arc<dyn AlertEventSink>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut states = std::collections::HashMap::new();
    let parallelism = std::thread::available_parallelism().map_or(1, usize::from);
    loop {
        tokio::select! { ()=cancellation.cancelled()=>return Ok(()), _=ticker.tick()=>{} }
        let Ok(targets) = reconciliation_targets(&pool).await else {
            continue;
        };
        states.retain(|key: &(uuid::Uuid, String), _| {
            targets.iter().any(|t| t.id == key.0 && t.address == key.1)
        });
        let probes = futures_util::stream::iter(targets.into_iter().map(|target| {
            let docker = docker.clone();
            let agent = agent.clone();
            let cancellation = cancellation.clone();
            let pool = pool.clone();
            async move {
                if target.connector_type == "EdgeAgent" {
                    let health = match edge_health(&pool, target.id).await {
                        Ok(health) => health,
                        Err(error) => {
                            tracing::warn!(%error,"Edge health lookup failed");
                            None
                        }
                    };
                    return (target, health);
                }
                let selected;
                let runtime: &dyn PlatformRuntimePort = if target.connector_type == "Local" {
                    &docker
                } else {
                    let Some(agent) = agent else {
                        return (target, None);
                    };
                    let Ok(client) = agent.at_address(&target.address) else {
                        return (target, Some(false));
                    };
                    selected = client;
                    &selected
                };
                let healthy = probe_health(runtime, &cancellation, Duration::from_secs(2)).await;
                (target, Some(healthy))
            }
        }))
        .buffer_unordered(parallelism);
        tokio::pin!(probes);
        loop {
            let next = tokio::select! { ()=cancellation.cancelled()=>return Ok(()), next=probes.next()=>next };
            let Some((target, Some(healthy))) = next else {
                if next.is_none() {
                    break;
                }
                continue;
            };
            let state: &mut HealthState = states
                .entry((target.id, target.address.clone()))
                .or_default();
            let Some(online) = state.observe(healthy) else {
                continue;
            };
            if online {
                if target.connector_type != "EdgeAgent" {
                    if target.connector_type == "Local" {
                        let _ = local.try_send(());
                    } else {
                        let _ = agent_trigger.try_send(Some(target.id));
                    }
                }
            } else if let Err(error) =
                citadel_adapters::resource_status_store::platform_offline(&pool, target.id).await
            {
                tracing::warn!(%error, "Offline resource synchronization failed");
                state.online = None; // Retry persistence on the next confirmed sample.
                continue;
            }
            let observation = if online {
                platform_reachable_observation(&target)
            } else {
                platform_unreachable_observation(
                    &target,
                    &RuntimeCapabilityError::new(
                        citadel_platforms::RuntimeErrorKind::Unavailable,
                        "Platform health checks failed",
                        true,
                    ),
                )
            };
            if let Err(error) = alerts.observe(&observation).await {
                tracing::warn!(%error, "Platform health Alert evaluation failed");
                state.online = None;
            }
            if let Some(hub) = &realtime {
                hub.publish_runtime_change(
                    target.id,
                    "platformInventory",
                    if online { "reachable" } else { "offline" },
                    target.id.to_string(),
                );
            }
        }
    }
}

async fn event_consumer(
    cancellation: CancellationToken,
    mut receiver: BoundedReceiver<InventoryEvent>,
    metrics: Arc<Metrics>,
    docker: DockerClient,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
    local_reconciliation: BoundedSender<()>,
    agent_reconciliation: BoundedSender<Option<uuid::Uuid>>,
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
        match apply_container_event(&event, &docker, &pool, realtime.as_ref(), &cancellation).await
        {
            Ok(true) if !matches!(event.action.as_str(), "update" | "rename" | "create") => {
                continue;
            }
            Ok(true) => {}
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(%error, "Container event update failed; scheduling reconciliation")
            }
        }
        queue_inventory_reconciliation(&event, &local_reconciliation, &agent_reconciliation);
    }
}

async fn apply_container_event(
    event: &InventoryEvent,
    docker: &DockerClient,
    pool: &PgPool,
    realtime: Option<&RealtimeHub>,
    cancellation: &CancellationToken,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if !event.resource_type.eq_ignore_ascii_case("container") {
        return Ok(false);
    }
    let Some(id) = event.container_id.as_deref().filter(|id| !id.is_empty()) else {
        return Ok(false);
    };
    if event.action.starts_with("exec_") || event.action == "attach" || event.action == "top" {
        return Ok(true);
    }
    let destroyed = event.action.eq_ignore_ascii_case("destroy");
    let (state, name, container) = if destroyed {
        (None, None, None)
    } else if matches!(event.source, ReconciliationTrigger::LocalEvent) {
        let inspected = tokio::select! {
            () = cancellation.cancelled() => return Ok(true),
            result = docker.inspect_container_document(id) => result?,
        };
        let container = citadel_adapters::docker::container_observation(inspected)?;
        (
            Some(container.state.clone()),
            Some(container.name.clone()),
            Some(container),
        )
    } else {
        if event.container_state.is_none() {
            return Ok(false);
        }
        (
            event.container_state.clone(),
            event.container_name.clone(),
            event.container.clone(),
        )
    };
    let mut handled = false;
    for target in reconciliation_targets(pool).await? {
        let matches = event.platform_id.map_or_else(
            || target.connector_type.eq_ignore_ascii_case("Local"),
            |id| id == target.id && target.connector_type.eq_ignore_ascii_case("Agent"),
        );
        if !matches {
            continue;
        }
        let updated = if let Some(container) = &container {
            citadel_adapters::resource_status_store::container_observation(
                pool,
                target.id,
                None,
                container,
                chrono::Utc::now().timestamp(),
            )
            .await?
        } else {
            citadel_adapters::resource_status_store::container_event(
                pool,
                target.id,
                None,
                id,
                state.as_deref(),
                name.as_deref(),
                chrono::Utc::now().timestamp(),
            )
            .await?
        };
        handled |= updated;
        if updated && let Some(hub) = realtime {
            hub.publish_runtime_change(target.id, "container", &event.action, id.to_owned());
        }
    }
    Ok(handled)
}

fn queue_inventory_reconciliation(
    event: &InventoryEvent,
    local: &BoundedSender<()>,
    agent: &BoundedSender<Option<uuid::Uuid>>,
) {
    if !triggers_inventory_reconciliation(&event.resource_type, &event.action) {
        return;
    }
    match event.source {
        ReconciliationTrigger::LocalEvent => {
            let _ = local.try_send(());
        }
        ReconciliationTrigger::AgentEvent => {
            let _ = agent.try_send(event.platform_id);
        }
    }
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

    #[tokio::test]
    #[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
    async fn agent_container_event_uses_the_bound_platform_without_scanning_local_docker() {
        let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
        citadel_database::MigrationRunner::migrate(&url)
            .await
            .unwrap();
        let pool = PgPool::connect(&url).await.unwrap();
        let platform = uuid::Uuid::now_v7();
        let deployment = uuid::Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,name,cpucount,imagecount,memtotal,networkcount,volumecount,address,connectortype,platformdescriptor,status) VALUES($1,$2,0,0,0,0,0,$3,'Agent','{\"$type\":\"Docker\"}','Online')")
            .bind(platform).bind(format!("event-{platform}")).bind(format!("https://{platform}.invalid")).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$2,$3,'{}','Healthy',$4)")
            .bind(deployment).bind(format!("event-{deployment}")).bind(platform).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO containers(id,platformid,deploymentid,dockercontainerid,dockerimageid,name,created,updated,state,ports) VALUES($1,$2,$3,'container-event','image','web',1,1,'Running','[]')")
            .bind(uuid::Uuid::now_v7()).bind(platform).bind(deployment).execute(&pool).await.unwrap();
        let docker = DockerClient::new(
            "/nonexistent-citadel-event-test.sock",
            Duration::from_millis(100),
        )
        .unwrap();
        let event = InventoryEvent {
            platform_id: Some(platform),
            container_id: Some("container-event".into()),
            container_state: Some("exited".into()),
            container_name: Some("web".into()),
            container: None,
            source: ReconciliationTrigger::AgentEvent,
            resource_type: "container".into(),
            action: "die".into(),
            event_time_millis: None,
        };
        assert!(
            apply_container_event(&event, &docker, &pool, None, &CancellationToken::new())
                .await
                .unwrap()
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT status FROM deployments WHERE id=$1")
                .bind(deployment)
                .fetch_one(&pool)
                .await
                .unwrap(),
            "Stopped"
        );
        pool.close().await;
    }
    #[test]
    fn local_event_bursts_cannot_hide_an_agent_reconciliation_trigger() {
        let (local, mut local_receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
        let (agent, mut agent_receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
        let local_event = InventoryEvent {
            platform_id: None,
            container_id: None,
            container_state: None,
            container_name: None,
            container: None,
            source: ReconciliationTrigger::LocalEvent,
            resource_type: "container".into(),
            action: "start".into(),
            event_time_millis: None,
        };
        let agent_event = InventoryEvent {
            platform_id: None,
            container_id: None,
            container_state: None,
            container_name: None,
            container: None,
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
        assert_eq!(agent_receiver.try_recv(), Some(None));
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

#[cfg(test)]
mod health_tests {
    use super::HealthState;
    #[test]
    fn transient_failures_do_not_flap_and_confirmed_transitions_emit_once() {
        let mut state = HealthState::default();
        assert_eq!(state.observe(true), None);
        assert_eq!(state.observe(true), Some(true));
        assert_eq!(state.observe(false), None);
        assert_eq!(state.observe(false), None);
        assert_eq!(state.observe(true), None);
        assert_eq!(state.observe(false), None);
        assert_eq!(state.observe(false), None);
        assert_eq!(state.observe(false), Some(false));
        assert_eq!(state.observe(false), None);
        assert_eq!(state.observe(true), None);
        assert_eq!(state.observe(true), Some(true));
        assert_eq!(state.observe(true), None);
    }
}

#[cfg(test)]
#[path = "subscription_tests.rs"]
mod subscription_tests;

#[cfg(test)]
#[path = "inventory_tests.rs"]
mod inventory_tests;
