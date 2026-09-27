//! Shared, bounded daemon snapshot. Concurrent RPCs refresh once and retain one cycle.
use citadel_adapters::connectors::docker::{DockerClient, LocalDockerSample, LocalDockerSampler};
use citadel_platforms::RuntimeCapabilityError;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::{sync::Mutex, time::Instant};
use tokio_util::sync::CancellationToken;

pub(super) struct SharedSampler {
    docker: DockerClient,
    epoch: AtomicU64,
    metadata_changed: std::sync::Arc<tokio::sync::Notify>,
    state: Mutex<State>,
}
struct State {
    sampler: LocalDockerSampler,
    cached: Option<(Instant, u64, u64, LocalDockerSample)>,
}
impl SharedSampler {
    pub fn new(docker: DockerClient) -> Self {
        let sampler = LocalDockerSampler::new(docker.clone(), 8);
        Self {
            metadata_changed: sampler.metadata_notification(),
            docker: docker.clone(),
            epoch: AtomicU64::new(0),
            state: Mutex::new(State {
                sampler,
                cached: None,
            }),
        }
    }
    pub async fn cached_stats(
        &self,
    ) -> std::collections::HashMap<String, citadel_platforms::RuntimeContainerStat> {
        let state = self.state.lock().await;
        let Some((at, epoch, generation, sample)) = state.cached.as_ref() else {
            return Default::default();
        };
        if at.elapsed() >= Duration::from_secs(6)
            || *epoch != self.epoch.load(Ordering::Acquire)
            || *generation != self.docker.daemon_generation()
        {
            return Default::default();
        }
        sample
            .containers
            .stats
            .iter()
            .map(|s| (s.docker_container_id.clone(), s.clone()))
            .collect()
    }
    pub fn metadata_notification(&self) -> std::sync::Arc<tokio::sync::Notify> {
        self.metadata_changed.clone()
    }
    pub fn invalidate(&self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
    }
    pub async fn sample(
        &self,
        max_age: Duration,
        cancel: &CancellationToken,
    ) -> Result<LocalDockerSample, RuntimeCapabilityError> {
        let cancelled = || {
            RuntimeCapabilityError::new(
                citadel_platforms::RuntimeErrorKind::Cancelled,
                "Sampling cancelled",
                false,
            )
        };
        let mut state = tokio::select! { biased; ()=cancel.cancelled()=>return Err(cancelled()), state=self.state.lock()=>state };
        let epoch = self.epoch.load(Ordering::Acquire);
        let generation = self.docker.daemon_generation();
        if let Some((at, old_epoch, old_generation, sample)) = &state.cached
            && at.elapsed() < max_age.min(Duration::from_secs(6))
            && *old_epoch == epoch
            && *old_generation == generation
        {
            let mut sample = sample.clone();
            state.sampler.enrich_metadata(&mut sample);
            return Ok(sample);
        }
        let sample = state.sampler.sample(cancel).await?;
        // Never turn failed/partial observations into cached successful zeroes.
        if sample.containers.failed_samples == 0
            && self.epoch.load(Ordering::Acquire) == epoch
            && self.docker.daemon_generation() == generation
        {
            state.cached = Some((
                Instant::now(),
                epoch,
                self.docker.daemon_generation(),
                sample.clone(),
            ));
        } else {
            state.cached = None;
        }
        Ok(sample)
    }
}
