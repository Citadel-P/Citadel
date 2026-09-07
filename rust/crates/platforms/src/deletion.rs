use citadel_domain::ActorId;
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

pub trait PlatformDeletionStore: Send + Sync {
    /// Atomically remove registrations and record audit snapshots; never mutate Docker.
    fn delete<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), PlatformDeletionError>>;
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
