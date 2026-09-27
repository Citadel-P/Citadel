use chrono::Utc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::RuntimeCapabilityError;
use crate::RuntimeInventorySnapshot;
use crate::RuntimeSwarmInventory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryCollectionTarget {
    pub platform_id: Uuid,
    pub platform_type: crate::PlatformKind,
}

/// Manager setup and post-mutation convergence only. Empty standalone fields
/// are compatibility DTO fields and must never be sent to a full-set writer.
pub async fn collect_swarm_snapshot<
    T: crate::PlatformInfoPort + crate::NetworkInventoryPort + crate::SwarmInventoryPort + ?Sized,
>(
    runtime: &T,
    target: &InventoryCollectionTarget,
    cancellation: &CancellationToken,
) -> Result<RuntimeInventorySnapshot, RuntimeCapabilityError> {
    let observed_at = Utc::now();
    let info = super::PlatformReconciler::collect(runtime, cancellation).await?;
    let swarm = collect_swarm_inventory(runtime, target, cancellation).await?;
    let networks = runtime.list_network_topology(cancellation).await?;
    Ok(RuntimeInventorySnapshot {
        platform_id: target.platform_id,
        info,
        swarm,
        networks,
        containers: vec![],
        images: vec![],
        volumes: vec![],
        observed_at,
    })
}

pub(super) async fn collect_swarm_inventory<T: crate::SwarmInventoryPort + ?Sized>(
    runtime: &T,
    target: &InventoryCollectionTarget,
    cancellation: &CancellationToken,
) -> Result<Option<RuntimeSwarmInventory>, RuntimeCapabilityError> {
    if target.platform_type == crate::PlatformKind::DockerSwarm {
        let nodes = runtime.list_swarm_nodes(cancellation).await?;
        let mut services = runtime.list_swarm_services(cancellation).await?;
        let mut tasks = runtime.list_swarm_tasks(cancellation).await?;
        let configs = runtime.list_swarm_configs(cancellation).await?;
        let secrets = runtime.list_swarm_secrets(cancellation).await?;
        reconcile_running_counts(&mut services, &tasks);
        let running_task_count = tasks
            .iter()
            .filter(|task| {
                task.desired_state.eq_ignore_ascii_case("running")
                    && task.state.eq_ignore_ascii_case("running")
            })
            .count();
        // Apply the active Task projection limit. Services/nodes remain
        // complete so absence and replica-count reconciliation stay authoritative.
        tasks.sort_by_key(|task| std::cmp::Reverse(task.status_timestamp));
        tasks.truncate(500);
        Ok(Some(RuntimeSwarmInventory {
            running_task_count,
            nodes,
            services,
            tasks,
            configs,
            secrets,
        }))
    } else {
        Ok(None)
    }
}

/// Some Engines include retired-but-still-running tasks in ServiceStatus.
/// Correct only when the task response contains the entire desired service set;
/// a bounded Agent response must never lower counts merely because it is partial.
fn reconcile_running_counts(
    services: &mut [crate::RuntimeSwarmService],
    tasks: &[crate::RuntimeSwarmTask],
) {
    for service in services {
        if service.desired_task_count <= 0
            || !matches!(
                service.mode.to_ascii_lowercase().as_str(),
                "replicated" | "global"
            )
        {
            continue;
        }
        let mut desired = 0_i32;
        let mut running = 0_i32;
        for task in tasks.iter().filter(|task| {
            task.service_id == service.id && task.desired_state.eq_ignore_ascii_case("running")
        }) {
            desired = desired.saturating_add(1);
            if task.state.eq_ignore_ascii_case("running") {
                running = running.saturating_add(1);
            }
        }
        if desired >= service.desired_task_count {
            service.running_task_count = service.running_task_count.min(running);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use futures_util::future::BoxFuture;

    use super::*;
    use crate::{
        RuntimeContainerSummary, RuntimeImageSummary, RuntimeNetworkSummary, RuntimePlatformInfo,
        RuntimeStatsStream, RuntimeSwarmConfig, RuntimeSwarmNode, RuntimeSwarmSecret,
        RuntimeSwarmService, RuntimeSwarmTask, RuntimeVolumeSummary,
    };

    #[tokio::test]
    async fn standalone_inventory_does_not_request_swarm_resources() {
        let runtime = CountingRuntime::default();
        let snapshot =
            collect_swarm_inventory(&runtime, &target("Docker"), &CancellationToken::new())
                .await
                .unwrap();

        assert!(snapshot.is_none());
        assert_eq!(runtime.common.load(Ordering::Relaxed), 0);
        assert_eq!(
            runtime.peak.load(Ordering::SeqCst),
            0,
            "one target must not multiply the inventory I/O budget"
        );
        assert_eq!(runtime.swarm.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn swarm_inventory_collects_each_bounded_resource_set_once() {
        let runtime = CountingRuntime::default();
        let snapshot =
            collect_swarm_snapshot(&runtime, &target("DockerSwarm"), &CancellationToken::new())
                .await
                .unwrap();

        let swarm = snapshot.swarm.unwrap();
        assert_eq!(
            swarm.tasks.len(),
            500,
            "active task projection stays bounded"
        );
        assert_eq!(
            swarm.running_task_count, 598,
            "count before truncation, excluding retired and pending tasks"
        );
        assert_eq!(swarm.tasks.first().unwrap().id, "599");
        assert_eq!(
            swarm.tasks.last().unwrap().id,
            "100",
            "newest task observations survive the bound"
        );
        assert_eq!(runtime.common.load(Ordering::Relaxed), 2);
        assert_eq!(
            runtime.peak.load(Ordering::SeqCst),
            1,
            "one target must not multiply the inventory I/O budget"
        );
        assert_eq!(runtime.swarm.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn retired_running_tasks_do_not_hide_pending_replacements_and_partial_lists_preserve_counts() {
        let mut services = vec![RuntimeSwarmService {
            id: "web".into(),
            mode: "Replicated".into(),
            desired_task_count: 2,
            running_task_count: 2,
            ..Default::default()
        }];
        let task = |state: &str, desired: &str| RuntimeSwarmTask {
            service_id: "web".into(),
            state: state.into(),
            desired_state: desired.into(),
            ..Default::default()
        };
        reconcile_running_counts(&mut services, &[task("running", "running")]);
        assert_eq!(
            services[0].running_task_count, 2,
            "partial Agent result cannot prove a lower count"
        );
        reconcile_running_counts(
            &mut services,
            &[
                task("running", "running"),
                task("running", "shutdown"),
                task("pending", "running"),
            ],
        );
        assert_eq!(services[0].running_task_count, 1);
    }

    #[tokio::test]
    async fn metadata_reads_info_only() {
        let runtime = CountingRuntime::default();
        super::super::collect_event_scope(
            crate::jobs::ResourceCollector::for_scope(
                &runtime,
                super::super::EventRefresh::Platform,
            ),
            Uuid::now_v7(),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(*runtime.calls.lock().unwrap(), ["get_info"]);
    }

    #[tokio::test]
    async fn registration_pins_identity_without_collecting_resource_sets() {
        use crate::registration::*;
        use std::sync::Arc;
        struct Resolver(Arc<CountingRuntime>);
        impl PlatformRegistrationRuntime for Resolver {
            fn info_for(
                &self,
                _: PlatformConnectorType,
                _: &str,
            ) -> Result<Arc<dyn crate::PlatformInfoPort>, PlatformRegistrationError> {
                Ok(self.0.clone())
            }
        }
        struct Repository;
        impl PlatformRegistrationRepository for Repository {
            fn ensure_available<'a>(
                &'a self,
                _: &'a str,
                _: &'a str,
            ) -> BoxFuture<'a, Result<(), PlatformRegistrationError>> {
                Box::pin(async { Ok(()) })
            }
            fn create<'a>(
                &'a self,
                _: citadel_primitives::ActorId,
                value: &'a PlatformRegistration,
            ) -> BoxFuture<'a, Result<Uuid, PlatformRegistrationError>> {
                Box::pin(async move {
                    assert_eq!(value.snapshot.info.daemon_id, "fixture");
                    assert!(
                        value.snapshot.containers.is_empty() && value.snapshot.images.is_empty()
                    );
                    assert!(
                        value.snapshot.networks.is_empty() && value.snapshot.volumes.is_empty()
                    );
                    assert!(value.snapshot.swarm.is_none());
                    Ok(value.id)
                })
            }
        }
        let runtime = Arc::new(CountingRuntime::default());
        let service = PlatformRegistrationService::new(
            Arc::new(Repository),
            Arc::new(Resolver(runtime.clone())),
        );
        service
            .create(
                citadel_primitives::ActorId::new(Uuid::now_v7()),
                CreatePlatformInput {
                    name: "fixture".into(),
                    address: None,
                    description: None,
                    platform_type: PlatformType::Docker,
                    connector_type: PlatformConnectorType::Local,
                    prune_historical_swarm_task_containers: true,
                    tag_ids: vec![],
                },
                &CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(*runtime.calls.lock().unwrap(), ["get_info"]);
    }

    #[tokio::test]
    async fn event_refreshes_read_only_the_requested_resource() {
        use crate::jobs::{EventRefresh, ReconciliationScope, collect_event_scope};
        for (scope, expected) in [
            (ReconciliationScope::Containers, "list_containers"),
            (ReconciliationScope::Images, "list_images"),
            (ReconciliationScope::Networks, "list_networks"),
            (ReconciliationScope::Volumes, "list_volumes"),
        ] {
            let runtime = CountingRuntime::default();
            collect_event_scope(
                crate::jobs::ResourceCollector::for_scope(&runtime, EventRefresh::Resource(scope)),
                Uuid::now_v7(),
                &CancellationToken::new(),
            )
            .await
            .unwrap();
            assert_eq!(*runtime.calls.lock().unwrap(), [expected]);
        }
        let runtime = CountingRuntime::default();
        collect_event_scope(
            crate::jobs::ResourceCollector::for_scope(&runtime, EventRefresh::Swarm),
            Uuid::now_v7(),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(
            *runtime.calls.lock().unwrap(),
            [
                "list_swarm_nodes",
                "list_swarm_services",
                "list_swarm_tasks",
                "list_swarm_configs",
                "list_swarm_secrets",
                "list_networks"
            ]
        );
    }

    fn target(platform_type: &str) -> InventoryCollectionTarget {
        InventoryCollectionTarget {
            platform_id: Uuid::now_v7(),
            platform_type: match platform_type {
                "DockerSwarm" => crate::PlatformKind::DockerSwarm,
                "Docker" => crate::PlatformKind::Docker,
                _ => panic!("Invalid test platform"),
            },
        }
    }

    #[derive(Default)]
    struct CountingRuntime {
        calls: std::sync::Mutex<Vec<&'static str>>,
        common: AtomicUsize,
        swarm: AtomicUsize,
        active: AtomicUsize,
        peak: AtomicUsize,
    }

    impl CountingRuntime {
        async fn operation(&self, method: &'static str) {
            self.calls.lock().unwrap().push(method);
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(active, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(2)).await;
            self.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl crate::PlatformInfoPort for CountingRuntime {
        fn get_info<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
            self.common.fetch_add(1, Ordering::Relaxed);
            Box::pin(async move {
                self.operation("get_info").await;
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
                    swarm: None,
                })
            })
        }
    }

    impl crate::ContainerInventoryPort for CountingRuntime {
        fn list_containers<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
            self.common.fetch_add(1, Ordering::Relaxed);
            Box::pin(async move {
                self.operation("list_containers").await;
                Ok(Vec::new())
            })
        }
    }

    impl crate::PlatformStatsPort for CountingRuntime {
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
                Box::pin(async move {
                    self.operation(stringify!($name)).await;
                    Ok(Vec::new())
                })
            }
        };
    }

    impl crate::ImageInventoryPort for CountingRuntime {
        counted_list!(list_images, RuntimeImageSummary, common);
    }
    impl crate::NetworkInventoryPort for CountingRuntime {
        counted_list!(list_networks, RuntimeNetworkSummary, common);
    }
    impl crate::VolumeInventoryPort for CountingRuntime {
        counted_list!(list_volumes, RuntimeVolumeSummary, common);
    }
    impl crate::SwarmInventoryPort for CountingRuntime {
        counted_list!(list_swarm_nodes, RuntimeSwarmNode, swarm);
        counted_list!(list_swarm_services, RuntimeSwarmService, swarm);
        fn list_swarm_tasks<'a>(
            &'a self,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmTask>, RuntimeCapabilityError>> {
            self.swarm.fetch_add(1, Ordering::Relaxed);
            Box::pin(async move {
                self.operation("list_swarm_tasks").await;
                Ok((0..600)
                    .map(|index| RuntimeSwarmTask {
                        id: index.to_string(),
                        desired_state: if index == 0 { "shutdown" } else { "RUNNING" }.into(),
                        state: if index == 1 { "pending" } else { "Running" }.into(),
                        status_timestamp: chrono::DateTime::from_timestamp(index, 0),
                        ..Default::default()
                    })
                    .collect())
            })
        }
        counted_list!(list_swarm_configs, RuntimeSwarmConfig, swarm);
        counted_list!(list_swarm_secrets, RuntimeSwarmSecret, swarm);
    }

    impl crate::NetworkObservationPort for CountingRuntime {
        fn inspect_network<'a>(
            &'a self,
            _id: &'a str,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimeNetworkSummary, RuntimeCapabilityError>> {
            unreachable!()
        }
    }

    impl crate::VolumeObservationPort for CountingRuntime {
        fn inspect_volume<'a>(
            &'a self,
            _name: &'a str,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
            unreachable!()
        }
    }
}
