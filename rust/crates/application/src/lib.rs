#![forbid(unsafe_code)]

mod supervisor;

use citadel_domain::{ActorId, PlatformSummary};
use futures_util::future::BoxFuture;

pub use supervisor::{SupervisedTaskError, TaskSupervisor};

#[derive(Debug, thiserror::Error)]
pub enum AuthorizedReadError {
    #[error("authorized read failed: {0}")]
    Storage(String),
}

pub trait AuthorizedPlatformReader: Send + Sync {
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
    ) -> BoxFuture<'a, Result<Vec<PlatformSummary>, AuthorizedReadError>>;
}
