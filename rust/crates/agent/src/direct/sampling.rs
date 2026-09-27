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
    state: Mutex<State>,
}
struct State {
    sampler: LocalDockerSampler,
    cached: Option<(Instant, u64, u64, LocalDockerSample)>,
}
impl SharedSampler {
    pub fn new(docker: DockerClient) -> Self {
        Self {
            docker: docker.clone(),
            epoch: AtomicU64::new(0),
            state: Mutex::new(State {
                sampler: LocalDockerSampler::new(docker, 8),
                cached: None,
            }),
        }
    }
    pub fn invalidate(&self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
    }
    pub async fn sample(
        &self,
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
            && at.elapsed() < Duration::from_secs(6)
            && *old_epoch == epoch
            && *old_generation == generation
        {
            return Ok(sample.clone());
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
