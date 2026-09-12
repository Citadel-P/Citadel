//! Start workers in one supervisor; application shutdown cancels and joins them.
use crate::state::AppState;
use citadel_application::{LicenseTransitionMonitor, ServiceAccountLastUsedWorker, TaskSupervisor};
use citadel_server::{config::Config, license_realtime, workers};
use std::sync::Arc;
use std::time::Duration;

/// These inputs are owned, because a queue receiver can only be started once.
pub struct Jobs {
    pub last_used_worker: ServiceAccountLastUsedWorker,
    pub license_transition_monitor: LicenseTransitionMonitor,
    pub license_realtime_hub: license_realtime::LicenseRealtimeHub,
}

pub fn spawn_all(state: &AppState, jobs: Jobs, config: &Config) -> TaskSupervisor {
    let AppState {
        cancellation,
        edge_registry,
        pool,
        realtime_hub,
        alert_store,
        platform_state,
        docker,
        readiness,
        metrics,
        agent,
        deployments,
        swarm_services,
        stacks,
        git_execution,
        automation,
        builds,
        backups,
        alert_deliveries,
        ..
    } = state;
    let Jobs {
        last_used_worker,
        license_transition_monitor,
        license_realtime_hub,
    } = jobs;
    let mut supervisor = TaskSupervisor::new(cancellation.clone());
    supervisor.spawn(
        "edge-platform-inventory",
        workers::edge::run(
            cancellation.child_token(),
            edge_registry.clone(),
            pool.clone(),
            realtime_hub.clone(),
            alert_store.clone(),
        ),
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
    workers::register(
        &mut supervisor,
        cancellation,
        workers::WorkerDependencies {
            volume_content: platform_state.volume_content.clone(),
            containers: platform_state.containers.clone(),
            docker: docker.clone(),
            pool: pool.clone(),
            readiness: Arc::clone(readiness),
            metrics: Arc::clone(metrics),
            agent: agent.clone(),
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
            queue_capacity: config.event_queue_capacity,
            probe_interval: config.probe_interval,
            reconciliation_interval: config.reconciliation_interval,
            agent_reconnect_delay: config
                .agent
                .as_ref()
                .map_or(Duration::from_secs(10), |agent| agent.reconnect_delay),
        },
    );

    supervisor
}
