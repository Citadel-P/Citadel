use super::*;

pub struct GitRepositoryService {
    persistence: Arc<dyn GitRepositoryPersistence>,
}
impl GitRepositoryService {
    pub fn new(persistence: Arc<dyn GitRepositoryPersistence>) -> Self {
        Self { persistence }
    }
    pub fn persistence(&self) -> &Arc<dyn GitRepositoryPersistence> {
        &self.persistence
    }
    pub async fn create(
        &self,
        actor: ActorId,
        mut command: CreateGitRepository,
    ) -> Result<GitRepository, GitRepositoryError> {
        command.validate()?;
        self.persistence
            .create_git_repository(actor, &command)
            .await
    }
}
