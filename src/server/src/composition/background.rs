//! Start workers in one supervisor; application shutdown cancels and joins them.
use super::ServerComponents;
use citadel_adapters::persistence::postgres::identity::service_accounts::usage::ServiceAccountLastUsedWorker;
use citadel_licensing::LicenseTransitionMonitor;
use citadel_runtime::TaskSupervisor;
use citadel_server::{config::Config, license_realtime, workers};
use std::{sync::Arc, time::Duration};

/// Background-only services and worker inputs that must be started once.
pub struct BackgroundServices {
    pub docker: citadel_docker::client::DockerClient,
    pub agent: Option<citadel_adapters::connectors::agent::client::AgentClient>,
    pub volume_content:
        Arc<citadel_adapters::connectors::routing::volumes::content::VolumeContentAdapter>,
    pub runtime_targets:
        Arc<citadel_adapters::connectors::routing::platforms::registry::PlatformRuntimeRegistry>,
    pub image_scanner: Arc<citadel_adapters::connectors::routing::images::scanner::ImageScanner>,
    pub alert_deliveries: Arc<citadel_alerts::AlertDeliveryService>,
    pub last_used_worker: ServiceAccountLastUsedWorker,
    pub license_transition_monitor: LicenseTransitionMonitor,
    pub license_realtime_hub: license_realtime::LicenseRealtimeHub,
}

pub async fn spawn_all(
    state: &ServerComponents,
    services: BackgroundServices,
    config: &Config,
) -> Result<TaskSupervisor, sqlx::Error> {
    let ServerComponents {
        cancellation,
        edge_registry,
        pool,
        realtime_hub,
        alert_store,
        platform_state,
        metrics,
        deployments,
        swarm_services,
        stacks,
        git_execution,
        scripts,
        builds,
        backups,
        copies,
        ..
    } = state;
    let BackgroundServices {
        docker,
        agent,
        volume_content,
        runtime_targets,
        image_scanner,
        alert_deliveries,
        last_used_worker,
        license_transition_monitor,
        license_realtime_hub,
    } = services;
    let mut supervisor = TaskSupervisor::new(cancellation.clone());
    supervisor.spawn(
        "update-checker",
        state
            .updates
            .clone()
            .run(state.identity.clone(), cancellation.child_token()),
    );
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
    let copy_changes = citadel_adapters::persistence::postgres::resource_copies::changes::PostgresCopyChanges::connect(pool).await?;
    supervisor.spawn(
        "resource-copy-realtime",
        workers::resource_copies::realtime_updates(
            Box::new(copy_changes),
            realtime_hub.clone(),
            cancellation.child_token(),
        ),
    );
    supervisor.spawn(
        "resource-copies",
        workers::resource_copies::run(cancellation.child_token(), copies.clone()),
    );
    let stats = workers::register(
        &mut supervisor,
        cancellation,
        workers::WorkerDependencies {
            image_updates: platform_state.image_updates.clone(),
            targets: runtime_targets.clone(),
            volume_content,
            containers: platform_state.containers.clone(),
            docker,
            pool: pool.clone(),
            metrics: Arc::clone(metrics),
            agent,
            realtime: realtime_hub.clone(),
            deployments: Arc::clone(deployments),
            swarm_services: Arc::clone(swarm_services),
            stacks: Arc::clone(stacks),
            git: Arc::clone(git_execution),
            scripts: Arc::clone(scripts),
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
            build_tracking_slots: config.build_tracking_slots,
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
            workers::edge::EdgeObservationStores {
                edge: Arc::new(citadel_adapters::persistence::postgres::platforms::edge::store::PostgresEdgeStore::new(pool.clone()).with_node_policy(config.node_agent_policy.clone())),
                projections: Arc::new(citadel_adapters::persistence::postgres::platforms::inventory::store::PostgresInventoryProjectionStore::new(pool.clone()).with_node_policy(config.node_agent_policy.clone())),
            },
            realtime_hub.clone(),
            workers::edge::InventorySettings {
                monitoring_interval: config.probe_interval,
                reconciliation_interval: config.reconciliation_interval,
            },
            runtime_targets.inventory_budget.clone(),
            stats,
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
