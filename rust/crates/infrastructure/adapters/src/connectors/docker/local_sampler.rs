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
}

#[derive(Clone)]
pub struct LocalDockerSample {
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
        }
    }

    pub async fn sample(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<LocalDockerSample, RuntimeCapabilityError> {
        let captured_at = chrono::Utc::now().timestamp();
        let generation = self.docker.daemon_generation();
        if self.generation != generation {
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
        let mut targets = Vec::with_capacity(containers.len().min(1_024));
        for container in containers {
            match container.state {
                Some(State::Running) => {
                    counts.running += 1;
                    if targets.len() < 1_024 {
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
        if self
            .refreshed
            .is_none_or(|at| at.elapsed() >= Duration::from_secs(60))
        {
            let refresh = async {
                let info = self.docker.info().await.map_err(normalize_docker_error)?;
                self.docker
                    .platform_stats_with_counts(info, ContainerCounts::default())
                    .await
            };
            let metadata = tokio::select! {
                () = cancellation.cancelled() => return Err(super::runtime::cancelled_error()),
                result = refresh => result,
            };
            match metadata {
                Ok(metadata) => self.metadata = Some((Instant::now(), metadata)),
                Err(error) => {
                    tracing::warn!(%error, "Slow Docker metadata refresh failed; container samples remain available")
                }
            }
            self.refreshed = Some(Instant::now());
        }
        // Three missed refreshes expire optional disk/storage telemetry. Never report stale
        // disk as a new measurement indefinitely after an outage.
        tracing::debug!(
            running_containers = counts.running,
            sampled = batch.stats.len(),
            failed_samples = batch.failed_samples,
            "Local Docker sampling cycle"
        );
        let platform = self
            .metadata
            .as_ref()
            .filter(|(at, _)| at.elapsed() < Duration::from_secs(180))
            .map(|(_, metadata)| {
                let mut sample = metadata.clone();
                sample.container_count = counts.total;
                sample.containers_running = counts.running;
                sample.containers_paused = counts.paused;
                sample.containers_stopped = counts.stopped;
                sample
            });
        Ok(LocalDockerSample {
            captured_at,
            containers: batch,
            platform,
        })
    }
}
