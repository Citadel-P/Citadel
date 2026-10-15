use crate::*;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use uuid::Uuid;

pub trait CopyRepository: Send + Sync {
    fn targets(
        &self,
        actor: ActorId,
        kind: CopyKind,
        source: Uuid,
    ) -> BoxFuture<'_, Result<CopyTargets, CopyError>>;
    fn prepare_destination<'a>(
        &'a self,
        actor: ActorId,
        plan: &'a mut CopyPlan,
    ) -> BoxFuture<'a, Result<(), CopyError>>;
    fn current_data_allowed(
        &self,
        actor: ActorId,
        platform: Uuid,
    ) -> BoxFuture<'_, Result<bool, CopyError>>;
    fn recover_current(&self) -> BoxFuture<'_, Result<(), CopyError>>;
    fn claim_current(
        &self,
        actor: ActorId,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<(CopyPlan, CopyRestore)>, CopyError>>;
    fn finish_current(
        &self,
        actor: ActorId,
        id: Uuid,
        volume: Uuid,
        result: Result<(), String>,
    ) -> BoxFuture<'_, Result<(), CopyError>>;
    fn source(
        &self,
        actor: ActorId,
        kind: CopyKind,
        id: Uuid,
    ) -> BoxFuture<'_, Result<CopySource, CopyError>>;
    fn snapshots<'a>(
        &'a self,
        actor: ActorId,
        source: &'a CopySource,
    ) -> BoxFuture<'a, Result<Vec<CopySnapshot>, CopyError>>;
    fn save_review<'a>(
        &'a self,
        actor: ActorId,
        plan: &'a CopyPlan,
    ) -> BoxFuture<'a, Result<(), CopyError>>;
    fn start(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>>;
    fn get(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>>;
    fn cleanup(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>>;
    fn finish_cleanup(
        &self,
        actor: ActorId,
        id: Uuid,
        result: Result<(), String>,
    ) -> BoxFuture<'_, Result<(), CopyError>>;
    fn cancel(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>>;
    /// A bounded scan of durable parents. Multiple callers may inspect the same
    /// parent; advance must fence each transition in a transaction.
    fn pending(&self) -> BoxFuture<'_, Result<Vec<(ActorId, Uuid)>, CopyError>>;
    fn plan(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyPlan, CopyError>>;
    fn advance(
        &self,
        actor: ActorId,
        id: Uuid,
        storage_result: Option<Result<(), String>>,
    ) -> BoxFuture<'_, Result<CopyProgress, CopyError>>;
}

pub trait CopyStorage: Send + Sync {
    fn copy_current<'a>(
        &'a self,
        plan: &'a CopyPlan,
        volume: &'a CopyRestore,
        cancel: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<(), CopyError>>;
    fn cleanup<'a>(
        &'a self,
        plan: &'a CopyPlan,
        restores: &'a [CopyRestore],
        cancel: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<(), CopyError>>;
    fn check<'a>(
        &'a self,
        plan: &'a CopyPlan,
        restores: Option<&'a [CopyRestore]>,
        cancel: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<(), CopyError>>;
}
#[derive(Debug, Clone)]
pub struct RestoredCopyVolume {
    pub platform_id: Uuid,
    pub restore_id: Uuid,
    pub node_id: Option<String>,
    pub name: String,
}

impl RestoredCopyVolume {
    pub fn verify_location(
        &self,
        platform_id: Uuid,
        node_id: Option<&str>,
    ) -> Result<(), CopyError> {
        if self.platform_id != platform_id || self.node_id.as_deref() != node_id {
            return Err(CopyError::Conflict("Copied storage belongs to another Platform or Swarm node. Restore its original placement before applying; storage is not moved by editing configuration.".into()));
        }
        Ok(())
    }
}

/// Committed progress invalidations; None asks subscribers to resynchronize.
pub trait CopyChangeSource: Send {
    fn next(&mut self) -> BoxFuture<'_, Result<Option<Uuid>, CopyError>>;
}
