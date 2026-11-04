use crate::{
    CheckState, CompletionReceipt, ReleaseNotice, UpdateError, UpdateJournal, UpdateOperation,
    UpdatePlan,
};
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use uuid::Uuid;

/// Human identity is rechecked against current roles within each write transaction.
#[derive(Debug, Clone, Copy)]
pub struct UpdateActor {
    pub subject_id: Uuid,
    pub actor_id: ActorId,
}

pub trait ReleaseCheckRepository: Send + Sync {
    fn checks(&self) -> BoxFuture<'_, Result<CheckState, UpdateError>>;
    fn record_check<'a>(
        &'a self,
        result: Result<Option<&'a ReleaseNotice>, &'a str>,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, UpdateError>>;
}

/// Read-only observation under the exact persisted maintenance claim. Implementations
/// reject released, foreign, corrupt or advanced ownership before reporting counts.
pub trait UpdateDrainRepository: Send + Sync {
    fn blockers<'a>(
        &'a self,
        operation: &'a UpdateOperation,
    ) -> BoxFuture<'a, Result<Vec<crate::UpdateDrainBlocker>, UpdateError>>;
}

pub trait UpdateRepository: ReleaseCheckRepository {
    /// Record interrupted preparation without releasing the claim or inventing helper evidence.
    fn interrupt<'a>(
        &'a self,
        operation: &'a UpdateOperation,
    ) -> BoxFuture<'a, Result<(), UpdateError>>;

    /// After runtime source reconciliation: verify source catalog and exact claim,
    /// persist failure/Activity and release ownership in one transaction.
    fn abort_source<'a>(
        &'a self,
        operation: &'a UpdateOperation,
    ) -> BoxFuture<'a, Result<(), UpdateError>>;

    fn save_review<'a>(
        &'a self,
        actor: UpdateActor,
        plan: &'a UpdatePlan,
    ) -> BoxFuture<'a, Result<(), UpdateError>>;
    fn review(
        &self,
        actor: UpdateActor,
        id: Uuid,
    ) -> BoxFuture<'_, Result<UpdatePlan, UpdateError>>;
    fn previous(
        &self,
        actor: UpdateActor,
        key: Uuid,
    ) -> BoxFuture<'_, Result<Option<UpdateOperation>, UpdateError>>;
    fn claim<'a>(
        &'a self,
        actor: UpdateActor,
        plan: &'a UpdatePlan,
        key: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(UpdateOperation, bool), UpdateError>>;
    fn active(&self) -> BoxFuture<'_, Result<Option<UpdateOperation>, UpdateError>>;
    fn operation(&self, id: Uuid) -> BoxFuture<'_, Result<UpdateOperation, UpdateError>>;
    /// Called only after the adapter validates the private journal and running
    /// execution evidence. The transaction owns state, receipts and Activities.
    fn import<'a>(
        &'a self,
        journal: &'a UpdateJournal,
    ) -> BoxFuture<'a, Result<Option<CompletionReceipt>, UpdateError>>;
    /// The caller verifies runtime readiness and helper cleanup. Success needs
    /// a committed target receipt; Failed/Postponed needs the committed outcome
    /// and unchanged source. Ambiguous replacement/recovery retains ownership.
    fn release<'a>(
        &'a self,
        journal: &'a UpdateJournal,
        running: &'a crate::CoreIdentity,
    ) -> BoxFuture<'a, Result<(), UpdateError>>;
}
