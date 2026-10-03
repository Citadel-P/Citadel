//! Adapt the process-owned tracker to the SwarmService feature's admission port.
use citadel_runtime::DynamicTasks;
use citadel_swarm_services::{SwarmServiceError, SwarmServiceTaskSpawner};
use futures_util::future::BoxFuture;

pub struct TrackedSwarmServiceTasks(DynamicTasks);

impl TrackedSwarmServiceTasks {
    pub fn new(tasks: DynamicTasks) -> Self {
        Self(tasks)
    }
}

impl SwarmServiceTaskSpawner for TrackedSwarmServiceTasks {
    fn spawn(
        &self,
        name: &'static str,
        operation: BoxFuture<'static, Result<(), SwarmServiceError>>,
    ) -> bool {
        self.0.spawn(name, operation)
    }
}

#[cfg(test)]
mod tests {
    use crate::tasks::swarm_services::*;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn swarm_services_tasks_share_the_process_owner_and_shutdown_gate() {
        let shutdown = CancellationToken::new();
        let owner = DynamicTasks::new(shutdown.clone());
        let spawner = TrackedSwarmServiceTasks::new(owner.clone());
        let cleaned = Arc::new(AtomicBool::new(false));
        let observed = cleaned.clone();
        let cancelled = shutdown.clone();
        assert!(spawner.spawn(
            "swarm_services.fixture",
            Box::pin(async move {
                cancelled.cancelled().await;
                observed.store(true, Ordering::SeqCst);
                Ok(())
            })
        ));
        assert_eq!(owner.active(), 1);
        shutdown.cancel();
        owner.drain(Duration::from_secs(1)).await.unwrap();
        assert!(cleaned.load(Ordering::SeqCst));
        assert_eq!(owner.active(), 0);
        assert!(!spawner.spawn("swarm_services.late", Box::pin(async { Ok(()) })));
    }
}
