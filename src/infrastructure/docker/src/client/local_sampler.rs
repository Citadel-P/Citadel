//! One discovery list per stats cycle; descriptor/storage/disk refresh at most once a minute.
use super::{
    DockerClient,
    runtime::{ContainerCounts, normalize_docker_error},
};
use citadel_platforms::{
    RuntimeCapabilityError, RuntimePlatformStats,
    jobs::{ContainerStatsBatch, sample_running_container_stats},
};
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

pub struct LocalDockerSampler {
    docker: DockerClient,
    concurrency: usize,
    refreshed: Option<Instant>,
    metadata: Option<(Instant, RuntimePlatformStats)>,
    generation: u64,
    metadata_changed: std::sync::Arc<tokio::sync::Notify>,
    refresh: tokio::task::JoinSet<(
        Instant,
        Result<RuntimePlatformStats, RuntimeCapabilityError>,
    )>,
}

#[derive(Clone)]
pub struct LocalDockerSample {
    counts: ContainerCounts,
    pub captured_at: i64,
    pub containers: ContainerStatsBatch,
    pub platform: Option<RuntimePlatformStats>,
}

impl LocalDockerSampler {
    pub fn new(docker: DockerClient, concurrency: usize) -> Self {
        Self {
            docker,
            concurrency: concurrency.clamp(1, 32),
            refreshed: None,
            metadata: None,
            generation: 0,
            metadata_changed: Default::default(),
            refresh: tokio::task::JoinSet::new(),
        }
    }

    pub fn metadata_notification(&self) -> std::sync::Arc<tokio::sync::Notify> {
        self.metadata_changed.clone()
    }

    pub async fn sample(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<LocalDockerSample, RuntimeCapabilityError> {
        let captured_at = chrono::Utc::now().timestamp();
        let generation = self.docker.daemon_generation();
        if self.generation != generation {
            self.refresh.abort_all();
            while self.refresh.join_next().await.is_some() {}
            self.refreshed = None;
            self.metadata = None;
            self.generation = generation;
        }
        let containers = tokio::select! {
            () = cancellation.cancelled() => return Err(super::runtime::cancelled_error()),
            result = self.docker.list_container_models(Some(true), None, None, None) => result.map_err(normalize_docker_error)?,
        };
        // Sampling consumes only identity and running state. Avoid serializing ports,
        // sorting/copying labels, and mapping ownership for every statistics tick.
        use citadel_docker_api::models::container_summary::State;
        let mut counts = ContainerCounts {
            total: containers.len() as i64,
            ..Default::default()
        };
        let mut targets = Vec::with_capacity(containers.len());
        for container in containers {
            match container.state {
                Some(State::Running) => {
                    counts.running += 1;
                    {
                        targets.push(citadel_platforms::RuntimeContainerSummary {
                            id: container.id.unwrap_or_default(),
                            state: "running".into(),
                            ..Default::default()
                        });
                    }
                }
                Some(State::Paused) => counts.paused += 1,
                Some(State::Exited) => counts.stopped += 1,
                _ => {}
            }
        }
        let batch =
            sample_running_container_stats(&self.docker, targets, self.concurrency, cancellation)
                .await?;
        // One owned slow refresh. JoinSet aborts it when the sampler is dropped.
        // Publish fast telemetry without waiting for metadata I/O.
        self.collect_metadata();
        if self.refresh.is_empty()
            && self
                .refreshed
                .is_none_or(|at| at.elapsed() >= Duration::from_secs(60))
        {
            let docker = self.docker.clone();
            let cancel = cancellation.clone();
            self.refreshed = Some(Instant::now());
            let changed = self.metadata_changed.clone();
            self.refresh.spawn(async move {
                let _timer =
                    citadel_runtime::runtime_metrics::RuntimeWork::DockerMetadataRefresh.start();
                let result = tokio::select! {
                    () = cancel.cancelled() => Err(super::runtime::cancelled_error()),
                    result = async {
                        let info = docker.info().await.map_err(normalize_docker_error)?;
                        docker.platform_stats_with_counts(info, ContainerCounts::default()).await
                    } => result,
                };
                changed.notify_one();
                (Instant::now(), result)
            });
        }
        // Three missed refreshes expire optional disk/storage telemetry. Never report stale
        // disk as a new measurement indefinitely after an outage.
        tracing::debug!(
            running_containers = counts.running,
            sampled = batch.stats.len(),
            failed_samples = batch.failed_samples,
            "Local Docker sampling cycle"
        );
        let mut sample = LocalDockerSample {
            counts,
            captured_at,
            containers: batch,
            platform: None,
        };
        self.enrich_metadata(&mut sample);
        Ok(sample)
    }

    /// Attach metadata that completed independently, without collecting another CPU cycle.
    pub fn enrich_metadata(&mut self, sample: &mut LocalDockerSample) {
        self.collect_metadata();
        if let Some((at, _)) = &self.metadata {
            citadel_runtime::runtime_metrics::RuntimeWork::DockerMetadataRefresh.age(at.elapsed());
        }
        sample.platform = self
            .metadata
            .as_ref()
            .filter(|(at, _)| at.elapsed() < Duration::from_secs(180))
            .map(|(_, metadata)| {
                let mut value = metadata.clone();
                value.container_count = sample.counts.total;
                value.containers_running = sample.counts.running;
                value.containers_paused = sample.counts.paused;
                value.containers_stopped = sample.counts.stopped;
                value
            });
    }
    fn collect_metadata(&mut self) {
        while let Some(result) = self.refresh.try_join_next() {
            match result {
                Ok((at, Ok(metadata))) => self.metadata = Some((at, metadata)),
                Ok((_, Err(error))) => {
                    if error.kind == citadel_platforms::RuntimeErrorKind::Cancelled {
                        self.refreshed = None;
                    }
                    tracing::warn!(%error, "Slow Docker metadata refresh failed");
                }
                Err(error) => {
                    tracing::warn!(%error, "Metadata task failed");
                }
            }
        }
    }
}
