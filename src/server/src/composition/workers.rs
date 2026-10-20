//! Worker dependency construction; running workers never receive a database pool.
use citadel_runtime::RuntimeSignal;
use std::sync::Arc;
use std::time::Duration;

use crate::workers::platforms::*;
use citadel_adapters::connectors::agent::client::AgentClient;
use citadel_adapters::persistence::postgres::notifications::PostgresNotifications;
use citadel_adapters::persistence::postgres::platforms::inventory::store::PostgresInventoryProjectionStore;
use citadel_alerts::{AlertDeliveryService, AlertEventSink};
use citadel_automation::AutomationService;
use citadel_backups::BackupService;
use citadel_builds::BuildService;
use citadel_deployments::DeploymentService;
use citadel_docker::client::DockerClient;
use citadel_git::GitRepositoryExecutionService;
use citadel_runtime::{QueueOverflowPolicy, TaskSupervisor, bounded_channel};
use citadel_stacks::StackService;
use citadel_swarm_services::SwarmServiceService;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use citadel_adapters::connectors::routing::platforms::registry::PlatformRuntimeRegistry;

use crate::metrics::Metrics;
use crate::realtime::RealtimeHub;

pub struct WorkerSettings {
    pub node_agent_policy: citadel_platforms::node_agents::NodeAgentReconciliationPolicy,
    pub stats_flush_interval: Duration,
    pub retention_interval: Duration,
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
    pub image_updates: Arc<citadel_platforms::image_updates::ImageUpdateService>,
    pub targets: Arc<PlatformRuntimeRegistry>,
    pub volume_content:
        Arc<citadel_adapters::connectors::routing::volumes::content::VolumeContentAdapter>,
    pub containers: Arc<citadel_platforms::containers::ContainerMutationService>,
    pub docker: DockerClient,
    pub pool: PgPool,
    pub metrics: Arc<Metrics>,
    pub agent: Option<AgentClient>,
    pub realtime: Option<RealtimeHub>,
    pub deployments: Arc<DeploymentService>,
    pub swarm_services: Arc<SwarmServiceService>,
    pub stacks: Arc<StackService>,
    pub git: Arc<GitRepositoryExecutionService>,
    pub automation: Arc<AutomationService>,
    pub builds: Arc<BuildService>,
    pub backups: Arc<BackupService>,
    pub alerts: Arc<citadel_adapters::persistence::postgres::alerts::PostgresAlertRepository>,
    pub alert_deliveries: Arc<AlertDeliveryService>,
}

pub async fn register(
    supervisor: &mut TaskSupervisor,
    cancellation: &CancellationToken,
    dependencies: WorkerDependencies,
    settings: WorkerSettings,
) -> Result<crate::workers::statistics::StatsIngress, sqlx::Error> {
    // Subscribe before starting inventory/event producers, so committed changes
    // cannot race past the alert/drift/pruning consumers.
    let coverage_listener=citadel_adapters::persistence::postgres::backups::coverage::changes::PostgresCoverageChanges::connect(&dependencies.pool).await?;
    supervisor.spawn(
        "backup-coverage-realtime",
        crate::workers::backup_coverage::realtime_updates(
            Box::new(coverage_listener),
            dependencies.realtime.clone(),
            cancellation.child_token(),
        ),
    );
    let unmanaged_listener =
        PostgresNotifications::connect(&dependencies.pool, "citadel_container_created").await?;
    let drift_listener =
        PostgresNotifications::connect(&dependencies.pool, "citadel_stack_drift").await?;
    let pruning_listener =
        PostgresNotifications::connect(&dependencies.pool, "citadel_swarm_prune").await?;
    let job_alert_listener =
        PostgresNotifications::connect(&dependencies.pool, "citadel_job_alerts").await?;
    let mut target_listener =
        PostgresNotifications::connect(&dependencies.pool, "citadel_platform_targets").await?;
    for signal in citadel_runtime::RuntimeSignal::ALL {
        target_listener.listen(signal.channel()).await?;
    }
    target_listener
        .listen(citadel_adapters::persistence::postgres::connection_events::CHANNEL)
        .await?;
    let notifications = crate::workers::notifications::DatabaseNotificationHub::new();
    supervisor.spawn(
        "connection-realtime-updates",
        crate::workers::connection_events::realtime_updates(
            notifications.connections.subscribe(),
            dependencies.realtime.clone(),
            cancellation.child_token(),
        ),
    );
    supervisor.spawn(
        "database-notifications",
        notifications
            .clone()
            .run(Box::new(target_listener), cancellation.child_token()),
    );
    let WorkerDependencies {
        image_updates,
        targets,
        volume_content,
        containers,
        docker,
        pool,
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
        "inventory-image-updates",
        crate::workers::image_updates::run(cancellation.child_token(), image_updates),
    );
    supervisor.spawn(
        "alert-configuration-cache",
        alerts.clone().watch_configuration(
            notifications.subscribe(RuntimeSignal::AlertRules),
            cancellation.child_token(),
        ),
    );
    let projections = Arc::new(
        PostgresInventoryProjectionStore::new(pool.clone())
            .with_node_policy(settings.node_agent_policy.clone()),
    );
    let edge =
        citadel_adapters::persistence::postgres::platforms::edge::store::PostgresEdgeStore::new(
            pool.clone(),
        );
    let container_reader = Arc::new(citadel_adapters::persistence::postgres::platforms::background::PostgresContainerMaintenanceReader(pool.clone()));
    let metric_alerts = Arc::new(citadel_adapters::persistence::postgres::alerts::platform_metrics::PostgresPlatformAlertBatchStore::new(pool.clone(), env!("CITADEL_BUILD_VERSION")));
    let maintenance = Arc::new(
        citadel_adapters::persistence::postgres::maintenance::PostgresMaintenanceStore(
            pool.clone(),
        ),
    );
    let stats_store = Arc::new(citadel_adapters::persistence::postgres::platforms::statistics::batch::PostgresStatsBatchStore::new(pool.clone()));
    let job_alerts = alerts.clone();
    let alerts: Arc<dyn AlertEventSink> = alerts;
    let stats = crate::workers::statistics::register(
        supervisor,
        cancellation,
        (stats_store.clone(), stats_store),
        realtime.clone(),
        settings.queue_capacity,
        settings.stats_batch_size,
        settings.stats_flush_interval,
    );
    supervisor.spawn(
        "platform-targets",
        targets.clone().run(
            cancellation.child_token(),
            notifications.subscribe(citadel_runtime::RuntimeSignal::Targets),
        ),
    );
    supervisor.spawn(
        "job-alert-observations",
        crate::workers::alerts::observations(
            cancellation.child_token(),
            job_alerts,
            Box::new(job_alert_listener),
        ),
    );
    supervisor.spawn(
        "volume-helper-recovery",
        crate::workers::volume_helpers::reconcile(cancellation.child_token(), volume_content),
    );
    supervisor.spawn(
        "container-operation-reconciliation",
        crate::workers::containers::reconcile(
            cancellation.child_token(),
            containers.clone(),
            notifications.subscribe(RuntimeSignal::ContainerRecovery),
        ),
    );
    supervisor.spawn(
        "swarm-task-pruning",
        crate::workers::pruning::run(
            cancellation.child_token(),
            container_reader.clone(),
            containers,
            Box::new(pruning_listener),
        ),
    );
    supervisor.spawn(
        "stack-event-drift",
        crate::workers::stacks::event_drift(
            cancellation.child_token(),
            stacks.clone(),
            Box::new(drift_listener),
        ),
    );
    let git_completed = git.subscribe_completion();
    supervisor.spawn(
        "git-repository-sync",
        crate::workers::git::git_repository_sync(
            cancellation.child_token(),
            git,
            notifications.subscribe(RuntimeSignal::Git),
        ),
    );
    supervisor.spawn(
        "automation-runs",
        crate::workers::automation::automation_runs(
            cancellation.child_token(),
            Arc::clone(&automation),
            notifications.subscribe(citadel_runtime::RuntimeSignal::Automation),
        ),
    );
    supervisor.spawn(
        "automation-scheduler",
        crate::workers::automation::automation_scheduler(cancellation.child_token(), automation),
    );
    supervisor.spawn(
        "platform-threshold-alerts",
        crate::workers::stats_alerts::run(
            cancellation.child_token(),
            metric_alerts,
            alerts.clone(),
            settings.stats_flush_interval,
            i64::try_from(settings.stats_batch_size).unwrap_or(i64::MAX),
            notifications.subscribe(RuntimeSignal::PlatformStats),
        ),
    );
    supervisor.spawn(
        "alert-deliveries",
        crate::workers::alerts::deliveries(
            cancellation.child_token(),
            alert_deliveries,
            notifications.subscribe(RuntimeSignal::AlertDelivery),
        ),
    );
    supervisor.spawn(
        "build-runs",
        crate::workers::builds::build_runs(
            cancellation.child_token(),
            Arc::clone(&builds),
            settings.build_parallel_runs,
            notifications.subscribe(citadel_runtime::RuntimeSignal::Builds),
        ),
    );
    supervisor.spawn(
        "build-consumers",
        crate::workers::builds::build_consumers(
            cancellation.child_token(),
            citadel_builds::BuildCompletionService::new(
                Arc::new(
                    citadel_adapters::persistence::postgres::builds::completion::PostgresBuildCompletionRepository::new(
                        pool.clone(),
                    ),
                ),
                Arc::new(crate::workers::build_consumers::BuildConsumers {
                    deployments: deployments.clone(),
                    stacks: stacks.clone(),
                    realtime: realtime.clone(),
                }),
                builds.clone(),
            ),
            notifications.subscribe(citadel_runtime::RuntimeSignal::BuildCompletion),
        ),
    );
    supervisor.spawn(
        "build-pool-health",
        crate::workers::builds::pool_health(
            cancellation.child_token(),
            builds,
            Arc::new(edge.clone()),
        ),
    );
    supervisor.spawn(
        "backup-runs",
        crate::workers::backups::backup_runs(
            cancellation.child_token(),
            Arc::clone(&backups),
            settings.backup_workers.clone(),
            notifications.subscribe(citadel_runtime::RuntimeSignal::Backups),
        ),
    );
    supervisor.spawn(
        "backup-restore-runs",
        crate::workers::backups::restore_runs(
            cancellation.child_token(),
            Arc::clone(&backups),
            settings.backup_workers.clone(),
            notifications.subscribe(citadel_runtime::RuntimeSignal::Restores),
        ),
    );
    supervisor.spawn(
        "backup-policy-scheduler",
        crate::workers::backups::policy_scheduler(
            cancellation.child_token(),
            backups,
            settings.backup_workers.clone(),
        ),
    );
    supervisor.spawn(
        "deployment-apply-reconciliation",
        crate::workers::deployments::deployment_apply_reconciliation(
            cancellation.child_token(),
            Arc::clone(&deployments),
            notifications.subscribe(RuntimeSignal::DeploymentRecovery),
        ),
    );
    supervisor.spawn(
        "deployment-image-updates",
        crate::workers::deployments::image_updates(cancellation.child_token(), deployments),
    );
    supervisor.spawn(
        "swarm-service-operation-reconciliation",
        crate::workers::swarm_services::swarm_service_operation_reconciliation(
            cancellation.child_token(),
            Arc::clone(&swarm_services),
            notifications.subscribe(RuntimeSignal::SwarmServiceRecovery),
        ),
    );
    supervisor.spawn(
        "swarm-service-active-operations",
        crate::workers::swarm_services::observe_active_operations(
            cancellation.child_token(),
            Arc::clone(&swarm_services),
            notifications.subscribe(RuntimeSignal::SwarmServiceOperations),
        ),
    );
    supervisor.spawn(
        "swarm-service-image-updates",
        crate::workers::swarm_services::swarm_service_image_updates(
            cancellation.child_token(),
            swarm_services,
        ),
    );
    supervisor.spawn(
        "stack-operation-reconciliation",
        crate::workers::stacks::stack_operation_reconciliation(
            cancellation.child_token(),
            Arc::clone(&stacks),
            notifications.subscribe(RuntimeSignal::StackRecovery),
        ),
    );
    supervisor.spawn(
        "stack-drift-monitor",
        crate::workers::stacks::stack_drift_monitor(
            cancellation.child_token(),
            Arc::clone(&stacks),
        ),
    );
    supervisor.spawn(
        "stack-webhooks",
        crate::workers::stacks::stack_webhooks(
            cancellation.child_token(),
            stacks.clone(),
            notifications.subscribe(RuntimeSignal::StackWebhooks),
        ),
    );
    supervisor.spawn(
        "stack-updates",
        crate::workers::stacks::stack_updates(
            cancellation.child_token(),
            stacks.clone(),
            git_completed,
        ),
    );
    supervisor.spawn(
        "unmanaged-container-alerts",
        crate::workers::unmanaged::run(
            cancellation.child_token(),
            container_reader,
            alerts.clone(),
            Box::new(unmanaged_listener),
        ),
    );
    supervisor.spawn(
        "maintenance",
        crate::workers::maintenance::run(
            cancellation.child_token(),
            maintenance,
            realtime.clone(),
            settings.build_retention_days,
            settings.retention_interval,
        ),
    );
    let (sender, receiver) = bounded_channel(settings.queue_capacity, QueueOverflowPolicy::Wait);
    let (local_reconcile_sender, local_reconcile_receiver) =
        bounded_channel(1, QueueOverflowPolicy::Reject);
    let (agent_reconcile_sender, agent_reconcile_receiver) =
        bounded_channel(256, QueueOverflowPolicy::Reject);

    let agent_overflow = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let agent_reconcile_sender = AgentReconciliationSignal {
        queue: agent_reconcile_sender,
        full: agent_overflow.clone(),
    };
    let (health_transitions, health_receiver) = bounded_channel(256, QueueOverflowPolicy::Wait);
    supervisor.spawn(
        "platform-health-monitor",
        platform_health_monitor(
            cancellation.child_token(),
            HealthWorker {
                docker: docker.clone(),
                targets: targets.clone(),
                edge: Arc::new(edge),
                transitions: health_transitions,
            },
        ),
    );
    // Lifecycle and missed-commit recovery are independent of probe scheduling.
    let (event_refresh_sender, event_refresh_receiver) = event_refresh_channels(256);
    supervisor.spawn(
        "platform-lifecycle-coordinator",
        platform_lifecycle(
            cancellation.child_token(),
            health_receiver,
            LifecycleWorker {
                targets: targets.clone(),
                store: projections.clone(),
                realtime: realtime.clone(),
                refreshes: event_refresh_sender.clone(),
                alerts: alerts.clone(),
            },
        ),
    );
    supervisor.spawn(
        "platform-deployment-catch-up",
        deployment_catch_up(
            cancellation.child_token(),
            targets.clone(),
            projections.clone(),
            realtime.clone(),
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
                    targets: targets.clone(),
                    ingress: stats.clone(),
                    metrics: metrics.clone(),
                    fetch_interval: settings.probe_interval,
                },
                settings.agent_reconnect_delay,
            ),
        );
    }
    supervisor.spawn(
        "event-resource-refresh",
        event_resource_refresh(
            cancellation.child_token(),
            event_refresh_receiver,
            EventRefreshWorker {
                refreshes: event_refresh_sender.clone(),
                targets: targets.clone(),
                docker: docker.clone(),
                store: projections.clone(),
                realtime: realtime.clone(),
                retry_delay: settings.agent_reconnect_delay,
                swarm_interval: settings.reconciliation_interval,
            },
        ),
    );
    supervisor.spawn(
        "docker-event-consumer",
        event_consumer(
            cancellation.child_token(),
            receiver,
            EventWorker {
                metrics: metrics.clone(),
                docker: docker.clone(),
                store: projections.clone(),
                realtime: realtime.clone(),
                targets: targets.clone(),
                event_refresh: event_refresh_sender.clone(),
                local_reconciliation: local_reconcile_sender,
                agent_reconciliation: agent_reconcile_sender,
            },
        ),
    );
    supervisor.spawn(
        "platform-inventory-reconciliation",
        inventory_reconciliation(
            cancellation.child_token(),
            InventoryReconciliationWorker {
                targets: targets.clone(),
                refreshes: event_refresh_sender,
                local_triggers: local_reconcile_receiver,
                agent_triggers: agent_reconcile_receiver,
                agent_overflow,
            },
        ),
    );
    supervisor.spawn(
        "local-container-stats",
        local_container_stats(
            cancellation.child_token(),
            docker,
            StatsWorkerContext {
                targets: targets.clone(),
                ingress: stats.clone(),
                metrics,
                fetch_interval: settings.probe_interval,
            },
        ),
    );
    Ok(stats)
}
