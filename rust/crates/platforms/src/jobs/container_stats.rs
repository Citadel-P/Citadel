use futures_util::{StreamExt, future::BoxFuture, stream};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    ContainerStatsStore, PlatformRuntimePort, RuntimeCapabilityError, RuntimeContainerStat,
};

pub trait ContainerStatsSampler: PlatformRuntimePort {
    fn sample_container_stats<'a>(
        &'a self,
        container_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeContainerStat, RuntimeCapabilityError>>;
}

pub struct ContainerStatsBatch {
    pub stats: Vec<RuntimeContainerStat>,
    pub failed_samples: usize,
}

pub async fn collect_running_container_stats<S>(
    source: &S,
    cancellation: &CancellationToken,
) -> Result<ContainerStatsBatch, RuntimeCapabilityError>
where
    S: ContainerStatsSampler + Clone + 'static,
{
    let containers = source.list_containers(cancellation).await?;
    let results = stream::iter(
        containers
            .into_iter()
            .filter(|container| container.state == "running")
            .take(1_024)
            .map(|container| {
                let source = source.clone();
                let cancellation = cancellation.clone();
                async move {
                    source
                        .sample_container_stats(&container.id, &cancellation)
                        .await
                }
            }),
    )
    .buffer_unordered(8)
    .collect::<Vec<_>>()
    .await;
    let failed_samples = results.iter().filter(|result| result.is_err()).count();
    let stats = results.into_iter().filter_map(Result::ok).collect();
    Ok(ContainerStatsBatch {
        stats,
        failed_samples,
    })
}

pub async fn persist_container_stats(
    store: &dyn ContainerStatsStore,
    platform_id: Uuid,
    stats: &[RuntimeContainerStat],
) -> Result<usize, RuntimeCapabilityError> {
    if stats.is_empty() {
        return Ok(0);
    }
    store.persist(platform_id, stats).await
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::{FutureExt, future::BoxFuture};

    use super::*;
    use crate::{RuntimeContainerSummary, RuntimePlatformInfo, RuntimeStatsStream};

    #[derive(Clone)]
    struct Fixture;

    impl PlatformRuntimePort for Fixture {
        fn get_info<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
            unreachable!()
        }

        fn list_containers<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
            Box::pin(async {
                Ok(vec![
                    RuntimeContainerSummary {
                        id: "running".into(),
                        state: "running".into(),
                        name: String::new(),
                        image: String::new(),
                        image_id: String::new(),
                        created: 0,
                        status: String::new(),
                        labels: Default::default(),
                        ports: serde_json::Value::Null,
                        stack: None,
                        is_system: false,
                        system_role: None,
                        has_citadel_ownership_labels: false,
                        is_swarm_task: false,
                    },
                    RuntimeContainerSummary {
                        id: "stopped".into(),
                        state: "exited".into(),
                        name: String::new(),
                        image: String::new(),
                        image_id: String::new(),
                        created: 0,
                        status: String::new(),
                        labels: Default::default(),
                        ports: serde_json::Value::Null,
                        stack: None,
                        is_system: false,
                        system_role: None,
                        has_citadel_ownership_labels: false,
                        is_swarm_task: false,
                    },
                ])
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

    impl ContainerStatsSampler for Fixture {
        fn sample_container_stats<'a>(
            &'a self,
            container_id: &'a str,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimeContainerStat, RuntimeCapabilityError>> {
            async move {
                Ok(RuntimeContainerStat {
                    docker_container_id: container_id.into(),
                    memory_active: 0.0,
                    memory_cache: 0.0,
                    cpu_usage: 0.0,
                    memory_limit: 0.0,
                    rx_bytes: 0.0,
                    tx_bytes: 0.0,
                    created: 0,
                })
            }
            .boxed()
        }
    }

    #[tokio::test]
    async fn samples_only_running_containers() {
        let batch = collect_running_container_stats(&Fixture, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(batch.failed_samples, 0);
        assert_eq!(batch.stats.len(), 1);
        assert_eq!(batch.stats[0].docker_container_id, "running");
    }
}
