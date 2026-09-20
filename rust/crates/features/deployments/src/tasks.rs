//! Consumer-owned admission to process-tracked durable Deployment work.
use crate::DeploymentError;
use futures_util::future::BoxFuture;

pub trait DeploymentTaskSpawner: Send + Sync {
    /// Reject before polling when process admission has closed. Accepted futures
    /// remain owned independently of the requesting connection and report errors.
    fn spawn(
        &self,
        name: &'static str,
        operation: BoxFuture<'static, Result<(), DeploymentError>>,
    ) -> bool;
}
