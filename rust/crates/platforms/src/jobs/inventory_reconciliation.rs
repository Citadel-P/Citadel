use chrono::Utc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    PlatformInventoryPort, RuntimeCapabilityError, RuntimeInventorySnapshot, RuntimePlatformInfo,
    RuntimeSwarmInventory,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryCollectionTarget {
    pub platform_id: Uuid,
    pub platform_type: String,
}

pub async fn collect_inventory(
    runtime: &dyn PlatformInventoryPort,
    target: &InventoryCollectionTarget,
    cancellation: &CancellationToken,
) -> Result<RuntimeInventorySnapshot, RuntimeCapabilityError> {
    let observed_at = Utc::now();
    // One external operation at a time per target. The orchestration layer bounds
    // target concurrency; nested try_join fan-out would multiply that budget.
    let info = runtime.get_info(cancellation).await?;
    let containers = runtime.list_containers(cancellation).await?;
    let images = runtime.list_images(cancellation).await?;
    let networks = runtime.list_networks(cancellation).await?;
    let volumes = runtime.list_volumes(cancellation).await?;
    let swarm = collect_swarm_inventory(runtime, target, cancellation).await?;
    Ok(RuntimeInventorySnapshot {
        platform_id: target.platform_id,
        info,
        containers,
        images,
        networks,
        volumes,
        swarm,
        observed_at,
    })
}

pub async fn collect_inventory_from_info(
    runtime: &dyn PlatformInventoryPort,
    target: &InventoryCollectionTarget,
    info: RuntimePlatformInfo,
    cancellation: &CancellationToken,
) -> Result<RuntimeInventorySnapshot, RuntimeCapabilityError> {
    let observed_at = Utc::now();
    let containers = runtime.list_containers(cancellation).await?;
    let images = runtime.list_images(cancellation).await?;
    let networks = runtime.list_networks(cancellation).await?;
    let volumes = runtime.list_volumes(cancellation).await?;
    let swarm = collect_swarm_inventory(runtime, target, cancellation).await?;
    Ok(RuntimeInventorySnapshot {
        platform_id: target.platform_id,
        info,
        containers,
        images,
        networks,
        volumes,
        swarm,
        observed_at,
    })
}

async fn collect_swarm_inventory(
    runtime: &dyn PlatformInventoryPort,
    target: &InventoryCollectionTarget,
    cancellation: &CancellationToken,
) -> Result<Option<RuntimeSwarmInventory>, RuntimeCapabilityError> {
    if target.platform_type.eq_ignore_ascii_case("DockerSwarm") {
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
        // Match the .NET active Task projection limit. Services/nodes remain
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
        PlatformRuntimePort, RuntimeContainerSummary, RuntimeImageSummary, RuntimeNetworkSummary,
        RuntimePlatformInfo, RuntimeStatsStream, RuntimeSwarmConfig, RuntimeSwarmNode,
        RuntimeSwarmSecret, RuntimeSwarmService, RuntimeSwarmTask, RuntimeVolumeSummary,
    };

    #[tokio::test]
    async fn standalone_inventory_does_not_request_swarm_resources() {
        let runtime = CountingRuntime::default();
        let snapshot = collect_inventory(&runtime, &target("Docker"), &CancellationToken::new())
            .await
            .unwrap();

        assert!(snapshot.swarm.is_none());
        assert_eq!(runtime.common.load(Ordering::Relaxed), 5);
        assert_eq!(
            runtime.peak.load(Ordering::SeqCst),
            1,
            "one target must not multiply the inventory I/O budget"
        );
        assert_eq!(runtime.swarm.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn swarm_inventory_collects_each_bounded_resource_set_once() {
        let runtime = CountingRuntime::default();
        let snapshot =
            collect_inventory(&runtime, &target("DockerSwarm"), &CancellationToken::new())
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
        assert_eq!(runtime.common.load(Ordering::Relaxed), 5);
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

    fn target(platform_type: &str) -> InventoryCollectionTarget {
        InventoryCollectionTarget {
            platform_id: Uuid::now_v7(),
            platform_type: platform_type.into(),
        }
    }

    #[derive(Default)]
    struct CountingRuntime {
        common: AtomicUsize,
        swarm: AtomicUsize,
        active: AtomicUsize,
        peak: AtomicUsize,
    }

    impl CountingRuntime {
        async fn operation(&self) {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(active, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(2)).await;
            self.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl PlatformRuntimePort for CountingRuntime {
        fn get_info<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
            self.common.fetch_add(1, Ordering::Relaxed);
            Box::pin(async move {
                self.operation().await;
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

        fn list_containers<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
            self.common.fetch_add(1, Ordering::Relaxed);
            Box::pin(async move {
                self.operation().await;
                Ok(Vec::new())
            })
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
                Box::pin(async move {
                    self.operation().await;
                    Ok(Vec::new())
                })
            }
        };
    }

    impl PlatformInventoryPort for CountingRuntime {
        counted_list!(list_images, RuntimeImageSummary, common);
        counted_list!(list_networks, RuntimeNetworkSummary, common);
        counted_list!(list_volumes, RuntimeVolumeSummary, common);
        counted_list!(list_swarm_nodes, RuntimeSwarmNode, swarm);
        counted_list!(list_swarm_services, RuntimeSwarmService, swarm);
        fn list_swarm_tasks<'a>(
            &'a self,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmTask>, RuntimeCapabilityError>> {
            self.swarm.fetch_add(1, Ordering::Relaxed);
            Box::pin(async move {
                self.operation().await;
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
