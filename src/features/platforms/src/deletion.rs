use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeletePlatformsInput {
    pub ids: Vec<Uuid>,
}

impl DeletePlatformsInput {
    pub fn validate(&mut self) -> Result<(), PlatformDeletionError> {
        if self.ids.is_empty() || self.ids.iter().any(Uuid::is_nil) {
            return Err(PlatformDeletionError::Validation);
        }
        self.ids.sort_unstable();
        self.ids.dedup();
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformDeletionError {
    #[error("Select at least one valid Platform.")]
    Validation,
    #[error("Platform does not exist.")]
    NotFound,
    #[error(
        "Platform is still referenced by workloads, builds or restores. Remove or migrate them first."
    )]
    InUse,
    #[error("{0}")]
    Storage(String),
}

pub trait PlatformDeletionRepository: Send + Sync {
    /// Atomically remove registrations and record audit snapshots; never mutate Docker.
    fn delete<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), PlatformDeletionError>>;
}

pub struct PlatformDeletionService {
    store: std::sync::Arc<dyn PlatformDeletionRepository>,
    sessions: std::sync::Arc<dyn crate::edge_management::EdgeSessionControl>,
}
impl PlatformDeletionService {
    pub fn new(
        store: std::sync::Arc<dyn PlatformDeletionRepository>,
        sessions: std::sync::Arc<dyn crate::edge_management::EdgeSessionControl>,
    ) -> Self {
        Self { store, sessions }
    }
    pub async fn delete(
        &self,
        actor: ActorId,
        ids: &[Uuid],
        removed: impl Fn(Uuid),
    ) -> Result<(), PlatformDeletionError> {
        self.store.delete(actor, ids).await?;
        // No fallible I/O or cancellation point after the committed deletion.
        for id in ids {
            self.sessions.disconnect_platform(*id);
            removed(*id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_deduplicates_selection() {
        for ids in [vec![], vec![Uuid::nil()]] {
            assert!(DeletePlatformsInput { ids }.validate().is_err());
        }
        let id = Uuid::now_v7();
        let mut input = DeletePlatformsInput { ids: vec![id, id] };
        input.validate().unwrap();
        assert_eq!(input.ids, [id]);
    }
}
