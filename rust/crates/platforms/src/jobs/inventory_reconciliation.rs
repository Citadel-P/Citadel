use chrono::Utc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    PlatformInventoryPort, RuntimeCapabilityError, RuntimeInventorySnapshot, RuntimeSwarmInventory,
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
    let (info, containers, images, networks, volumes) = tokio::try_join!(
        runtime.get_info(cancellation),
        runtime.list_containers(cancellation),
        runtime.list_images(cancellation),
        runtime.list_networks(cancellation),
        runtime.list_volumes(cancellation),
    )?;
    let swarm = if target.platform_type.eq_ignore_ascii_case("DockerSwarm") {
        let (nodes, services, tasks, configs, secrets) = tokio::try_join!(
            runtime.list_swarm_nodes(cancellation),
            runtime.list_swarm_services(cancellation),
            runtime.list_swarm_tasks(cancellation),
            runtime.list_swarm_configs(cancellation),
            runtime.list_swarm_secrets(cancellation),
        )?;
        Some(RuntimeSwarmInventory {
            nodes,
            services,
            tasks,
            configs,
            secrets,
        })
    } else {
        None
    };
    Ok(RuntimeInventorySnapshot {
        platform_id: target.platform_id,
        info,
        containers,
        images,
        networks,
        volumes,
        swarm,
        observed_at: Utc::now(),
    })
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
        assert_eq!(runtime.swarm.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn swarm_inventory_collects_each_bounded_resource_set_once() {
        let runtime = CountingRuntime::default();
        let snapshot =
            collect_inventory(&runtime, &target("DockerSwarm"), &CancellationToken::new())
                .await
                .unwrap();

        assert!(snapshot.swarm.is_some());
        assert_eq!(runtime.common.load(Ordering::Relaxed), 5);
        assert_eq!(runtime.swarm.load(Ordering::Relaxed), 5);
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
    }

    impl PlatformRuntimePort for CountingRuntime {
        fn get_info<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
            self.common.fetch_add(1, Ordering::Relaxed);
            Box::pin(async {
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
                })
            })
        }

        fn list_containers<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
            self.common.fetch_add(1, Ordering::Relaxed);
            Box::pin(async { Ok(Vec::new()) })
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
                Box::pin(async { Ok(Vec::new()) })
            }
        };
    }

    impl PlatformInventoryPort for CountingRuntime {
        counted_list!(list_images, RuntimeImageSummary, common);
        counted_list!(list_networks, RuntimeNetworkSummary, common);
        counted_list!(list_volumes, RuntimeVolumeSummary, common);
        counted_list!(list_swarm_nodes, RuntimeSwarmNode, swarm);
        counted_list!(list_swarm_services, RuntimeSwarmService, swarm);
        counted_list!(list_swarm_tasks, RuntimeSwarmTask, swarm);
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
