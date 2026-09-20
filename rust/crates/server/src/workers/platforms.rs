#[path = "platforms/agents.rs"]
mod agents;
use agents::*;

#[path = "platforms/stats.rs"]
mod stats;
use stats::*;

#[path = "platforms/events.rs"]
mod events;
use events::*;

#[path = "platforms/health.rs"]
mod health;
use health::*;

#[path = "platforms/inventory.rs"]
mod inventory;
use inventory::*;

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
    InventoryCollectionTarget, collect_inventory, triggers_inventory_reconciliation,
};
use citadel_platforms::{
    InventoryProjectionStore, PlatformInventoryPort, PlatformRuntimePort, RuntimeCapabilityError,
};
use citadel_stacks::StackService;
use citadel_swarm_services::SwarmServiceService;
use futures_util::StreamExt;
use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;

use crate::Readiness;
use crate::runtime_targets::{
    HEALTH_CONCURRENCY, INVENTORY_CONCURRENCY, PlatformRuntimeRegistry,
    PlatformTarget as ReconciliationTarget, STATS_CONCURRENCY,
};
use citadel_application::{IoBudget, runtime_metrics::RuntimeWork};
use citadel_platforms::PlatformHealthPort;

use crate::metrics::Metrics;
use crate::realtime::RealtimeHub;

pub struct WorkerSettings {
    pub node_agent_policy:
        citadel_adapters::node_agent_reconciliation::NodeAgentReconciliationPolicy,
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
    pub targets: Arc<PlatformRuntimeRegistry>,
    pub volume_content: Arc<citadel_adapters::volume_content::VolumeContentAdapter>,
    pub containers: Arc<citadel_platforms::containers::ContainerMutationService>,
    pub docker: DockerClient,
    pub pool: PgPool,
    pub readiness: Arc<Readiness>,
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
    let mut target_listener =
        super::listener(&dependencies.pool, "citadel_platform_targets").await?;
    for signal in citadel_application::RuntimeSignal::ALL {
        target_listener.listen(signal.channel()).await?;
    }
    let notifications = super::notifications::DatabaseNotificationHub::new();
    supervisor.spawn(
        "database-notifications",
        notifications
            .clone()
            .run(target_listener, cancellation.child_token()),
    );
    let WorkerDependencies {
        targets,
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
        "platform-targets",
        targets.clone().run(
            cancellation.child_token(),
            notifications.subscribe(citadel_application::RuntimeSignal::Targets),
        ),
    );
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
        super::automation::automation_runs(
            cancellation.child_token(),
            Arc::clone(&automation),
            notifications.subscribe(citadel_application::RuntimeSignal::Automation),
        ),
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
            notifications.subscribe(citadel_application::RuntimeSignal::Builds),
        ),
    );
    supervisor.spawn(
        "build-consumers",
        super::builds::build_consumers(
            cancellation.child_token(),
            citadel_builds::BuildCompletionService::new(
                Arc::new(
                    citadel_adapters::postgres::builds::completion::PostgresBuildCompletionRepository::new(
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
            notifications.subscribe(citadel_application::RuntimeSignal::BuildCompletion),
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
            notifications.subscribe(citadel_application::RuntimeSignal::Backups),
        ),
    );
    supervisor.spawn(
        "backup-restore-runs",
        super::backups::restore_runs(
            cancellation.child_token(),
            Arc::clone(&backups),
            settings.backup_workers.clone(),
            notifications.subscribe(citadel_application::RuntimeSignal::Restores),
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
    supervisor.spawn(
        "platform-resource-health",
        resource_health(
            cancellation.child_token(),
            HealthWorker {
                docker: docker.clone(),
                targets: targets.clone(),
                pool: pool.clone(),
                realtime: realtime.clone(),
                local: local_reconcile_sender.clone(),
                agent_trigger: agent_reconcile_sender.clone(),
                alerts: alerts.clone(),
            },
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
            EventWorker {
                metrics: metrics.clone(),
                docker: docker.clone(),
                pool: pool.clone(),
                realtime: realtime.clone(),
                targets: targets.clone(),
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
                budget: targets.inventory_budget.clone(),
                node_agent_policy: settings.node_agent_policy.clone(),
                docker: docker.clone(),
                pool: pool.clone(),
                local_triggers: local_reconcile_receiver,
                agent_triggers: agent_reconcile_receiver,
                agent_overflow,
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
    supervisor.spawn(
        "local-container-stats",
        local_container_stats(
            cancellation.child_token(),
            docker,
            StatsWorkerContext {
                targets: targets.clone(),
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
    targets: Arc<PlatformRuntimeRegistry>,
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

#[derive(Clone)]
struct AgentReconciliationSignal {
    queue: BoundedSender<Option<uuid::Uuid>>,
    full: Arc<std::sync::atomic::AtomicBool>,
}
impl AgentReconciliationSignal {
    fn request(&self, platform: Option<uuid::Uuid>) {
        if self.queue.try_send(platform).is_err() {
            // The queue is already waking the consumer. Upgrade that pass to all
            // persisted Agents instead of silently losing a platform scope.
            self.full.store(true, std::sync::atomic::Ordering::Release);
            // If the consumer drained between try_send and the flag write,
            // ensure it receives another wake instead of waiting for the timer.
            let _ = self.queue.try_send(None);
        }
    }
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

async fn wait_to_reconnect(cancellation: &CancellationToken, reconnect_delay: Duration) -> bool {
    tokio::select! {
        biased;
        () = cancellation.cancelled() => true,
        () = tokio::time::sleep(reconnect_delay) => false,
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
        let targets = PlatformRuntimeRegistry::new(pool.clone(), None);
        targets.refresh().await.unwrap();
        assert!(
            apply_container_event(
                &event,
                &docker,
                &pool,
                None,
                &CancellationToken::new(),
                &targets
            )
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
        let agent = AgentReconciliationSignal {
            queue: agent,
            full: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
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
    fn overflowing_agent_scopes_upgrade_the_pending_pass_to_all_agents() {
        let (queue, mut receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
        let signal = AgentReconciliationSignal {
            queue,
            full: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        let first = uuid::Uuid::now_v7();
        signal.request(Some(first));
        signal.request(Some(uuid::Uuid::now_v7()));
        assert_eq!(receiver.try_recv(), Some(Some(first)));
        assert!(signal.full.swap(false, std::sync::atomic::Ordering::AcqRel));
        assert!(receiver.try_recv().is_none());
    }

    #[test]
    fn platform_inventory_failure_maps_to_the_existing_alert_contract() {
        let id = uuid::Uuid::now_v7();
        let target = ReconciliationTarget {
            agent: None,
            id,
            name: "worker-01".into(),
            address: "http://agent:8080".into(),
            connector_type: citadel_platforms::ConnectorKind::Agent,
            platform_type: citadel_platforms::PlatformKind::Docker,
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
