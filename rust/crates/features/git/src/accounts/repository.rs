use super::*;

pub trait GitAccountRepository: Send + Sync {
    fn list<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<StoredGitAccount>, GitAccountError>>;
    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>>;
    fn create<'a>(
        &'a self,
        account: &'a StoredGitAccount,
    ) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>>;
    fn update<'a>(
        &'a self,
        account: &'a StoredGitAccount,
    ) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>>;
    fn delete<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), GitAccountError>>;
}
