use futures_util::{StreamExt, stream};
use tokio_util::sync::CancellationToken;

use crate::RuntimeCapabilityError;
use crate::RuntimeContainerStat;

use crate::ContainerStatsPort;

#[derive(Clone)]
pub struct ContainerStatsBatch {
    pub stats: Vec<RuntimeContainerStat>,
    pub failed_samples: usize,
}

/// Sample an already-discovered set. The caller owns discovery and the I/O budget.
pub async fn sample_running_container_stats<S>(
    source: &S,
    containers: Vec<crate::RuntimeContainerSummary>,
    concurrency: usize,
    cancellation: &CancellationToken,
) -> Result<ContainerStatsBatch, RuntimeCapabilityError>
where
    S: ContainerStatsPort + Clone + 'static,
{
    let samples = stream::iter(
        containers
            .into_iter()
            .filter(|container| container.state == "running")
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
    .buffer_unordered(concurrency.clamp(1, 32))
    .collect::<Vec<_>>();
    let results = tokio::select! {
        biased;
        () = cancellation.cancelled() => return Err(RuntimeCapabilityError::new(crate::RuntimeErrorKind::Cancelled, "Sampling cancelled", false)),
        results = samples => results,
    };
    let failed_samples = results.iter().filter(|result| result.is_err()).count();
    let stats = results.into_iter().filter_map(Result::ok).collect();
    Ok(ContainerStatsBatch {
        stats,
        failed_samples,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::{FutureExt, future::BoxFuture};

    use super::*;
    use crate::{ContainerInventoryPort, RuntimeContainerSummary};

    #[derive(Default)]
    struct Counters {
        active: std::sync::atomic::AtomicUsize,
        peak: std::sync::atomic::AtomicUsize,
        lists: std::sync::atomic::AtomicUsize,
    }
    #[derive(Clone, Default)]
    struct Fixture(std::sync::Arc<Counters>);

    impl crate::ContainerInventoryPort for Fixture {
        fn list_containers<'a>(
            &'a self,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
            self.0
                .lists
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
    }

    impl ContainerStatsPort for Fixture {
        fn sample_container_stats<'a>(
            &'a self,
            container_id: &'a str,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RuntimeContainerStat, RuntimeCapabilityError>> {
            async move {
                let active = self
                    .0
                    .active
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    + 1;
                self.0
                    .peak
                    .fetch_max(active, std::sync::atomic::Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(2)).await;
                self.0
                    .active
                    .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
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
        let source = Fixture::default();
        let cancellation = CancellationToken::new();
        let containers = source.list_containers(&cancellation).await.unwrap();
        let batch = sample_running_container_stats(&source, containers, 8, &cancellation)
            .await
            .unwrap();
        assert_eq!(batch.failed_samples, 0);
        assert_eq!(batch.stats.len(), 1);
        assert_eq!(batch.stats[0].docker_container_id, "running");
    }
    #[tokio::test]
    async fn supplied_discovery_respects_the_explicit_sampling_budget() {
        let source = Fixture::default();
        let cancel = CancellationToken::new();
        let container = source.list_containers(&cancel).await.unwrap().remove(0);
        let containers = (0..48)
            .map(|id| {
                let mut value = container.clone();
                value.id = id.to_string();
                value
            })
            .collect();
        let batch = sample_running_container_stats(&source, containers, 3, &cancel)
            .await
            .unwrap();
        assert_eq!(batch.stats.len(), 48);
        assert_eq!(source.0.peak.load(std::sync::atomic::Ordering::SeqCst), 3);
        assert_eq!(
            source.0.lists.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the sampler must reuse supplied discovery"
        );
        cancel.cancel();
        assert!(
            sample_running_container_stats(&source, vec![container], 3, &cancel)
                .await
                .is_err()
        );
        assert_eq!(source.0.active.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn samples_every_container_beyond_the_old_limit_with_bounded_concurrency() {
        let source = Fixture::default();
        let containers = (0..1_025)
            .map(|i| RuntimeContainerSummary {
                id: i.to_string(),
                state: "running".into(),
                ..Default::default()
            })
            .collect();
        let batch =
            sample_running_container_stats(&source, containers, 3, &CancellationToken::new())
                .await
                .unwrap();
        assert_eq!(batch.stats.len(), 1_025);
        assert_eq!(batch.failed_samples, 0);
        assert!(source.0.peak.load(std::sync::atomic::Ordering::SeqCst) <= 3);
    }
}
