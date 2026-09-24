//! Start workers in one supervisor; application shutdown cancels and joins them.
use crate::composition::ServerComponents;
use citadel_licensing::LicenseTransitionMonitor;
use citadel_runtime::{ServiceAccountLastUsedWorker, TaskSupervisor};
use citadel_server::{config::Config, license_realtime, workers};
use std::{sync::Arc, time::Duration};

/// Background-only services and worker inputs that must be started once.
pub struct Jobs {
    pub runtime_targets: Arc<citadel_server::runtime_targets::PlatformRuntimeRegistry>,
    pub image_scanner: Arc<citadel_adapters::connectors::routing::images::scanner::ImageScanner>,
    pub alert_deliveries: Arc<citadel_alerts::AlertDeliveryService>,
    pub last_used_worker: ServiceAccountLastUsedWorker,
    pub license_transition_monitor: LicenseTransitionMonitor,
    pub license_realtime_hub: license_realtime::LicenseRealtimeHub,
}

pub async fn spawn_all(
    state: &ServerComponents,
    jobs: Jobs,
    config: &Config,
) -> Result<TaskSupervisor, sqlx::Error> {
    let ServerComponents {
        cancellation,
        edge_registry,
        pool,
        realtime_hub,
        alert_store,
        platform_state,
        readiness,
        metrics,
        deployments,
        swarm_services,
        stacks,
        git_execution,
        automation,
        builds,
        backups,
        ..
    } = state;
    let Jobs {
        runtime_targets,
        image_scanner,
        alert_deliveries,
        last_used_worker,
        license_transition_monitor,
        license_realtime_hub,
    } = jobs;
    let mut supervisor = TaskSupervisor::new(cancellation.clone());
    supervisor.spawn(
        "service-account-last-used",
        last_used_worker.run(cancellation.child_token()),
    );
    supervisor.spawn(
        "license-transition-monitor",
        license_realtime::run_license_transition_monitor(
            cancellation.child_token(),
            license_transition_monitor,
            license_realtime_hub,
            alert_store.clone(),
        ),
    );
    workers::register(
        &mut supervisor,
        cancellation,
        workers::WorkerDependencies {
            targets: runtime_targets.clone(),
            volume_content: platform_state.volume_content.clone(),
            containers: platform_state.containers.clone(),
            docker: platform_state.docker.clone(),
            pool: pool.clone(),
            readiness: Arc::clone(readiness),
            metrics: Arc::clone(metrics),
            agent: platform_state.agent.clone(),
            realtime: realtime_hub.clone(),
            deployments: Arc::clone(deployments),
            swarm_services: Arc::clone(swarm_services),
            stacks: Arc::clone(stacks),
            git: Arc::clone(git_execution),
            automation: Arc::clone(automation),
            builds: Arc::clone(builds),
            backups: Arc::clone(backups),
            alerts: alert_store.clone(),
            alert_deliveries: alert_deliveries.clone(),
        },
        workers::WorkerSettings {
            node_agent_policy: config.node_agent_policy.clone(),
            stats_flush_interval: config.stats_flush_interval,
            retention_interval: config.retention_interval,
            stats_batch_size: config.stats_batch_size,
            build_parallel_runs: config.build_parallel_runs,
            build_retention_days: config.build_retention_days,
            backup_workers: config.backup_workers.clone(),
            queue_capacity: config.event_queue_capacity,
            probe_interval: config.probe_interval,
            reconciliation_interval: config.reconciliation_interval,
            agent_reconnect_delay: config
                .agent
                .as_ref()
                .map_or(Duration::from_secs(10), |agent| agent.reconnect_delay),
        },
    )
    .await?;

    supervisor.spawn(
        "edge-platform-inventory",
        workers::edge::run(
            cancellation.child_token(),
            edge_registry.clone(),
            pool.clone(),
            realtime_hub.clone(),
            workers::edge::InventorySettings {
                node_policy: config.node_agent_policy.clone(),
                reconciliation_interval: config.reconciliation_interval,
            },
            runtime_targets.inventory_budget.clone(),
        ),
    );
    supervisor.spawn(
        "image-scanner",
        workers::image_scanning::run(
            cancellation.child_token(),
            image_scanner.clone(),
            deployments.clone(),
            stacks.clone(),
            swarm_services.clone(),
        ),
    );
    Ok(supervisor)
}
