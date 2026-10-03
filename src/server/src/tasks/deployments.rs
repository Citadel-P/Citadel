//! Adapt the process-owned tracker to the Deployment feature's admission port.
use citadel_deployments::{DeploymentError, DeploymentTaskSpawner};
use citadel_runtime::DynamicTasks;
use futures_util::future::BoxFuture;

pub struct TrackedDeploymentTasks(DynamicTasks);

impl TrackedDeploymentTasks {
    pub fn new(tasks: DynamicTasks) -> Self {
        Self(tasks)
    }
}

impl DeploymentTaskSpawner for TrackedDeploymentTasks {
    fn spawn(
        &self,
        name: &'static str,
        operation: BoxFuture<'static, Result<(), DeploymentError>>,
    ) -> bool {
        self.0.spawn(name, operation)
    }
}

#[cfg(test)]
mod tests {
    use crate::tasks::deployments::*;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn deployment_tasks_share_the_process_owner_and_shutdown_gate() {
        let shutdown = CancellationToken::new();
        let owner = DynamicTasks::new(shutdown.clone());
        let spawner = TrackedDeploymentTasks::new(owner.clone());
        let cleaned = Arc::new(AtomicBool::new(false));
        let observed = cleaned.clone();
        let cancelled = shutdown.clone();
        assert!(spawner.spawn(
            "deployment.fixture",
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
        assert!(!spawner.spawn("deployment.late", Box::pin(async { Ok(()) })));
    }
}
