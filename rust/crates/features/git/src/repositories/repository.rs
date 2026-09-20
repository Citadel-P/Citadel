use super::*;

pub trait GitRepositoryExecutionPersistence: Send + Sync {
    fn get_source<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<GitRepositorySource, GitRepositoryExecutionError>>;
    fn enqueue_sync<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>>;
    fn enqueue_apply<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        self.enqueue_sync(actor_id, id, Some(branch))
    }
    fn enqueue_webhook<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: &'a str,
        expected: &'a GitRepositoryWebhook,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>>;
    fn enqueue_due<'a>(
        &'a self,
        limit: usize,
    ) -> BoxFuture<'a, Result<usize, GitRepositoryExecutionError>>;
    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<GitSyncClaim>, GitRepositoryExecutionError>>;
    fn complete<'a>(
        &'a self,
        claim: &'a GitSyncClaim,
        result: &'a SyncResult,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>>;
    fn fail<'a>(
        &'a self,
        claim: &'a GitSyncClaim,
        message: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>>;
    fn list_refs<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<GitRepositoryRef>, GitRepositoryExecutionError>>;
    fn get_ref<'a>(
        &'a self,
        id: Uuid,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Option<GitRepositoryRef>, GitRepositoryExecutionError>>;
    fn resolve_reference<'a>(
        &'a self,
        id: Uuid,
        revision: Option<&'a str>,
    ) -> BoxFuture<'a, Result<String, GitRepositoryExecutionError>>;
    fn get_webhook<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<GitRepositoryWebhook>, GitRepositoryExecutionError>>;
}

pub trait GitRepositoryPersistence: Send + Sync {
    fn list_git_repositories<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<GitRepository>, GitRepositoryError>>;
    fn get_git_repository<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<GitRepository, GitRepositoryError>>;
    fn create_git_repository<'a>(
        &'a self,
        actor_id: ActorId,
        repository: &'a CreateGitRepository,
    ) -> BoxFuture<'a, Result<GitRepository, GitRepositoryError>>;
    fn update_git_repository<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        patch: &'a GitRepositoryPatch,
        kind: GitRepositoryMutationKind,
    ) -> BoxFuture<'a, Result<GitRepository, GitRepositoryError>>;
    fn delete_git_repositories<'a>(
        &'a self,
        actor_id: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), GitRepositoryError>>;
}
