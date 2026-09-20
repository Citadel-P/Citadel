#[derive(Clone)]
pub struct PostgresGitRepositoryExecutionPersistence {
    pub(super) pool: PgPool,
}

impl PostgresGitRepositoryExecutionPersistence {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
use super::*;

impl GitRepositoryExecutionPersistence for PostgresGitRepositoryExecutionPersistence {
    fn get_source<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<GitRepositorySource, GitRepositoryExecutionError>> {
        self.get_source_impl(id)
    }

    fn enqueue_sync<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        self.enqueue_sync_impl(actor_id, id, branch)
    }

    fn enqueue_apply<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        self.enqueue_apply_impl(actor_id, id, branch)
    }

    fn enqueue_webhook<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: &'a str,
        expected: &'a GitRepositoryWebhook,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        self.enqueue_webhook_impl(actor_id, id, branch, expected)
    }

    fn enqueue_due<'a>(
        &'a self,
        limit: usize,
    ) -> BoxFuture<'a, Result<usize, GitRepositoryExecutionError>> {
        self.enqueue_due_impl(limit)
    }

    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<GitSyncClaim>, GitRepositoryExecutionError>> {
        self.claim_next_impl(stale_before)
    }

    fn complete<'a>(
        &'a self,
        claim: &'a GitSyncClaim,
        result: &'a SyncResult,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        self.complete_impl(claim, result)
    }

    fn fail<'a>(
        &'a self,
        claim: &'a GitSyncClaim,
        message: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        self.fail_impl(claim, message)
    }

    fn get_ref<'a>(
        &'a self,
        id: Uuid,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Option<GitRepositoryRef>, GitRepositoryExecutionError>> {
        self.get_ref_impl(id, branch)
    }

    fn list_refs<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<GitRepositoryRef>, GitRepositoryExecutionError>> {
        self.list_refs_impl(id)
    }

    fn resolve_reference<'a>(
        &'a self,
        id: Uuid,
        revision: Option<&'a str>,
    ) -> BoxFuture<'a, Result<String, GitRepositoryExecutionError>> {
        self.resolve_reference_impl(id, revision)
    }

    fn get_webhook<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<GitRepositoryWebhook>, GitRepositoryExecutionError>> {
        self.get_webhook_impl(id)
    }
}
