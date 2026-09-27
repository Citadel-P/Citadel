use citadel_runtime::RuntimeSignal;
mod agents;
use agents::*;

mod stats;
use stats::*;

pub(crate) mod event_refresh;
use event_refresh::*;
pub(crate) mod scoped_reconciler;
pub(crate) mod swarm_reconciliation;
use scoped_reconciler::*;

mod events;
use events::*;

#[cfg(test)]
mod state_delta_tests;

mod health;
use health::*;
mod lifecycle;
use lifecycle::*;

mod inventory;
use inventory::*;

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use citadel_adapters::connectors::agent::client::AgentClient;
use citadel_adapters::connectors::docker::DockerClient;
use citadel_adapters::connectors::docker::DockerError;
use citadel_adapters::connectors::docker::events::{ContainerChange, RuntimeEventKind};
use citadel_adapters::persistence::postgres::platforms::inventory::store::PostgresInventoryProjectionStore;
use citadel_alerts::{AlertDeliveryService, AlertEventSink, AlertObservation};
use citadel_automation::AutomationService;
use citadel_backups::BackupService;
use citadel_builds::BuildService;
use citadel_deployments::DeploymentService;
use citadel_git::GitRepositoryExecutionService;
use citadel_platforms::jobs::{DeltaOutcome, EventRefresh, ReconciliationDecision, event_decision};
use citadel_platforms::{PlatformInventoryPort, RuntimeCapabilityError};
use citadel_runtime::{
    BoundedReceiver, BoundedSender, QueueOverflowPolicy, TaskSupervisor, bounded_channel,
};
use citadel_stacks::StackService;
use citadel_swarm_services::SwarmServiceService;
use futures_util::StreamExt;
use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;

use crate::runtime_targets::{
    HEALTH_CONCURRENCY, PlatformRuntimeRegistry, PlatformTarget as ReconciliationTarget,
    STATS_CONCURRENCY,
};
use citadel_platforms::PlatformHealthPort;
use citadel_runtime::{IoBudget, runtime_metrics::RuntimeWork};

use crate::metrics::Metrics;
use crate::realtime::RealtimeHub;

pub struct WorkerSettings {
    pub node_agent_policy:
        citadel_adapters::persistence::postgres::platforms::node_agents::reconciliation::NodeAgentReconciliationPolicy,
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
) -> Result<super::statistics::StatsIngress, sqlx::Error> {
    // Subscribe before starting inventory/event producers, so initial discovery
    // cannot race past the alert/drift/pruning consumers.
    let unmanaged_listener =
        super::listener(&dependencies.pool, "citadel_container_created").await?;
    let drift_listener = super::listener(&dependencies.pool, "citadel_stack_drift").await?;
    let pruning_listener = super::listener(&dependencies.pool, "citadel_swarm_prune").await?;
    let job_alert_listener = super::listener(&dependencies.pool, "citadel_job_alerts").await?;
    let mut target_listener =
        super::listener(&dependencies.pool, "citadel_platform_targets").await?;
    for signal in citadel_runtime::RuntimeSignal::ALL {
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
        "alert-configuration-cache",
        alerts.clone().watch_configuration(
            notifications.subscribe(RuntimeSignal::AlertRules),
            cancellation.child_token(),
        ),
    );
    let alerts: Arc<dyn AlertEventSink> = alerts;
    let stats = super::statistics::register(
        supervisor,
        cancellation,
        pool.clone(),
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
        super::containers::reconcile(
            cancellation.child_token(),
            containers.clone(),
            notifications.subscribe(RuntimeSignal::ContainerRecovery),
        ),
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
        super::git::git_repository_sync(
            cancellation.child_token(),
            git,
            notifications.subscribe(RuntimeSignal::Git),
        ),
    );
    supervisor.spawn(
        "automation-runs",
        super::automation::automation_runs(
            cancellation.child_token(),
            Arc::clone(&automation),
            notifications.subscribe(citadel_runtime::RuntimeSignal::Automation),
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
            notifications.subscribe(RuntimeSignal::PlatformStats),
        ),
    );
    supervisor.spawn(
        "alert-deliveries",
        super::alerts::deliveries(
            cancellation.child_token(),
            alert_deliveries,
            notifications.subscribe(RuntimeSignal::AlertDelivery),
        ),
    );
    supervisor.spawn(
        "build-runs",
        super::builds::build_runs(
            cancellation.child_token(),
            Arc::clone(&builds),
            settings.build_parallel_runs,
            notifications.subscribe(citadel_runtime::RuntimeSignal::Builds),
        ),
    );
    supervisor.spawn(
        "build-consumers",
        super::builds::build_consumers(
            cancellation.child_token(),
            citadel_builds::BuildCompletionService::new(
                Arc::new(
                    citadel_adapters::persistence::postgres::builds::completion::PostgresBuildCompletionRepository::new(
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
            notifications.subscribe(citadel_runtime::RuntimeSignal::BuildCompletion),
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
            notifications.subscribe(citadel_runtime::RuntimeSignal::Backups),
        ),
    );
    supervisor.spawn(
        "backup-restore-runs",
        super::backups::restore_runs(
            cancellation.child_token(),
            Arc::clone(&backups),
            settings.backup_workers.clone(),
            notifications.subscribe(citadel_runtime::RuntimeSignal::Restores),
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
            notifications.subscribe(RuntimeSignal::DeploymentRecovery),
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
            notifications.subscribe(RuntimeSignal::SwarmServiceRecovery),
        ),
    );
    supervisor.spawn(
        "swarm-service-active-operations",
        super::swarm_services::observe_active_operations(
            cancellation.child_token(),
            Arc::clone(&swarm_services),
            notifications.subscribe(RuntimeSignal::SwarmServiceOperations),
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
            notifications.subscribe(RuntimeSignal::StackRecovery),
        ),
    );
    supervisor.spawn(
        "stack-drift-monitor",
        super::stacks::stack_drift_monitor(cancellation.child_token(), Arc::clone(&stacks)),
    );
    supervisor.spawn(
        "stack-webhooks",
        super::stacks::stack_webhooks(
            cancellation.child_token(),
            stacks.clone(),
            notifications.subscribe(RuntimeSignal::StackWebhooks),
        ),
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
    let (health_transitions, health_receiver) = bounded_channel(256, QueueOverflowPolicy::Wait);
    supervisor.spawn(
        "platform-health-monitor",
        platform_health_monitor(
            cancellation.child_token(),
            HealthWorker {
                docker: docker.clone(),
                targets: targets.clone(),
                pool: pool.clone(),
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
                pool: pool.clone(),
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
            pool.clone(),
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
                pool: pool.clone(),
                realtime: realtime.clone(),
                node_policy: settings.node_agent_policy.clone(),
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
                pool: pool.clone(),
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

#[derive(Clone)]
struct StatsWorkerContext {
    targets: Arc<PlatformRuntimeRegistry>,
    ingress: super::statistics::StatsIngress,
    metrics: Arc<Metrics>,
    fetch_interval: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub resource: Option<citadel_platforms::jobs::ResourceDelta>,
    stream_recovered: bool,
    swarm_scope: bool,
    kind: RuntimeEventKind,
    platform_id: Option<uuid::Uuid>,
    container_id: Option<String>,
    container_state: Option<String>,
    container_name: Option<String>,
    container: Option<citadel_platforms::RuntimeContainerSummary>,
    source: ReconciliationTrigger,
    action: String,
    event_time_millis: Option<u128>,
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
            resource: None,
            stream_recovered: false,
            swarm_scope: false,
            kind: RuntimeEventKind::Container(ContainerChange::Exited),
            platform_id: Some(platform),
            container_id: Some("container-event".into()),
            container_state: Some("exited".into()),
            container_name: Some("web".into()),
            container: None,
            source: ReconciliationTrigger::AgentEvent,
            action: "die".into(),
            event_time_millis: None,
        };
        let targets = PlatformRuntimeRegistry::new(pool.clone(), None);
        targets.refresh().await.unwrap();
        let hub =
            crate::realtime::RealtimeHub::new(16, Arc::new(crate::metrics::Metrics::default()));
        let mut updates = hub.subscribe();
        assert!(
            apply_container_event(
                &event,
                &docker,
                &pool,
                Some(&hub),
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
        tokio::time::timeout(Duration::from_secs(2), updates.recv())
            .await
            .unwrap()
            .unwrap();
        let revision: i64 =
            sqlx::query_scalar("SELECT rowversion FROM containers WHERE platformid=$1")
                .bind(platform)
                .fetch_one(&pool)
                .await
                .unwrap();
        // Holding the owner lock proves a duplicate does not attempt reconciliation.
        let mut owner = pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM deployments WHERE id=$1 FOR NO KEY UPDATE")
            .bind(deployment)
            .fetch_one(&mut *owner)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(
                Duration::from_secs(1),
                apply_container_event(
                    &event,
                    &docker,
                    &pool,
                    Some(&hub),
                    &CancellationToken::new(),
                    &targets,
                )
            )
            .await
            .expect("duplicate must not wait for the deployment lock")
            .unwrap(),
            "no-op is handled without fallback enumeration"
        );
        owner.rollback().await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT rowversion FROM containers WHERE platformid=$1")
                .bind(platform)
                .fetch_one(&pool)
                .await
                .unwrap(),
            revision
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(200), updates.recv())
                .await
                .is_err(),
            "no duplicate realtime event after the coalescing window"
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
            resource: None,
            stream_recovered: true,
            swarm_scope: false,
            kind: RuntimeEventKind::Container(ContainerChange::Running),
            platform_id: None,
            container_id: None,
            container_state: None,
            container_name: None,
            container: None,
            source: ReconciliationTrigger::LocalEvent,
            action: "start".into(),
            event_time_millis: None,
        };
        let agent_event = InventoryEvent {
            resource: None,
            stream_recovered: true,
            swarm_scope: false,
            kind: RuntimeEventKind::SwarmDirty(
                citadel_adapters::connectors::docker::events::SwarmResource::Service,
            ),
            platform_id: None,
            container_id: None,
            container_state: None,
            container_name: None,
            container: None,
            source: ReconciliationTrigger::AgentEvent,
            action: "update".into(),
            event_time_millis: None,
        };

        queue_stream_recovery(&local_event, &local, &agent);
        queue_stream_recovery(&local_event, &local, &agent);
        queue_stream_recovery(&agent_event, &local, &agent);

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
mod subscription_tests;

#[cfg(test)]
mod inventory_tests;

#[cfg(test)]
mod event_tests;

#[cfg(test)]
mod event_policy_tests;
