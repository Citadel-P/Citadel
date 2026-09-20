//! Process-owned admission and cancellation for container mutations.
use citadel_platforms::{RuntimeCapabilityError, containers::ContainerTaskSpawner};
use citadel_runtime::DynamicTasks;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

pub struct TrackedContainerTasks(DynamicTasks);

impl TrackedContainerTasks {
    pub fn new(tasks: DynamicTasks) -> Self {
        Self(tasks)
    }
}

impl ContainerTaskSpawner for TrackedContainerTasks {
    fn spawn(&self, operation: BoxFuture<'static, Result<(), RuntimeCapabilityError>>) -> bool {
        self.0.spawn("container.mutation", operation)
    }
    fn shutdown_token(&self) -> CancellationToken {
        self.0.cancellation()
    }
}

#[cfg(test)]
mod tests {
    use crate::tasks::platforms::*;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    #[tokio::test]
    async fn process_shutdown_drains_containers_and_closes_admission() {
        let shutdown = CancellationToken::new();
        let owner = DynamicTasks::new(shutdown.clone());
        let spawner = TrackedContainerTasks::new(owner.clone());
        let token = spawner.shutdown_token();
        let completed = Arc::new(AtomicBool::new(false));
        let observed = completed.clone();
        assert!(spawner.spawn(Box::pin(async move {
            token.cancelled().await;
            observed.store(true, Ordering::SeqCst);
            Ok(())
        })));
        assert_eq!(owner.active(), 1);
        shutdown.cancel();
        owner.drain(Duration::from_secs(1)).await.unwrap();
        assert!(completed.load(Ordering::SeqCst));
        assert!(!spawner.spawn(Box::pin(async { Ok(()) })));
    }
}
