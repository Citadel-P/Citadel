//! Admission to process-owned work that survives the initiating request.
use crate::BuildError;
use futures_util::future::BoxFuture;

pub trait BuildTaskSpawner: Send + Sync {
    /// Reject without polling when shutdown closes admission. Accepted work
    /// remains owned until completion and reports failures to the process owner.
    fn spawn(&self, name: &'static str, task: BoxFuture<'static, Result<(), BuildError>>) -> bool;
}
