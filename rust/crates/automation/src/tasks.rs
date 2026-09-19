//! Admission to direct executions owned and drained by the server process.
use crate::AutomationError;
use futures_util::future::BoxFuture;
pub trait AutomationTaskSpawner: Send + Sync {
    /// Reject without polling once shutdown closes admission.
    fn spawn(
        &self,
        name: &'static str,
        task: BoxFuture<'static, Result<(), AutomationError>>,
    ) -> bool;
}
