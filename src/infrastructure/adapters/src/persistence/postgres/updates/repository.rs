use super::*;

pub(super) fn storage(error: impl std::fmt::Display) -> UpdateError {
    UpdateError::Storage(error.to_string())
}

#[derive(Clone)]
pub struct PostgresUpdateRepository {
    pub(super) pool: PgPool,
    pub(super) authorization: Arc<AuthorizationCache>,
}

impl PostgresUpdateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            authorization: AuthorizationCache::attach(&pool),
            pool,
        }
    }
}

impl UpdateRepository for PostgresUpdateRepository {
    fn interrupt<'a>(
        &'a self,
        operation: &'a UpdateOperation,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(self.interrupt_impl(operation))
    }

    fn abort_source<'a>(
        &'a self,
        operation: &'a UpdateOperation,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(self.abort_source_impl(operation))
    }

    fn save_review<'a>(
        &'a self,
        actor: UpdateActor,
        plan: &'a UpdatePlan,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(self.save_review_impl(actor, plan))
    }

    fn review(
        &self,
        actor: UpdateActor,
        id: Uuid,
    ) -> BoxFuture<'_, Result<UpdatePlan, UpdateError>> {
        Box::pin(self.review_impl(actor, id))
    }

    fn previous(
        &self,
        actor: UpdateActor,
        key: Uuid,
    ) -> BoxFuture<'_, Result<Option<UpdateOperation>, UpdateError>> {
        Box::pin(self.previous_impl(actor, key))
    }

    fn claim<'a>(
        &'a self,
        actor: UpdateActor,
        plan: &'a UpdatePlan,
        key: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(UpdateOperation, bool), UpdateError>> {
        Box::pin(self.claim_impl(actor, plan, key, now))
    }

    fn active(&self) -> BoxFuture<'_, Result<Option<UpdateOperation>, UpdateError>> {
        Box::pin(self.active_impl())
    }

    fn operation(&self, id: Uuid) -> BoxFuture<'_, Result<UpdateOperation, UpdateError>> {
        Box::pin(self.operation_impl(id))
    }

    fn import<'a>(
        &'a self,
        journal: &'a UpdateJournal,
    ) -> BoxFuture<'a, Result<Option<CompletionReceipt>, UpdateError>> {
        Box::pin(self.import_impl(journal))
    }

    fn release<'a>(
        &'a self,
        journal: &'a UpdateJournal,
        running: &'a CoreIdentity,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(self.release_impl(journal, running))
    }
}
