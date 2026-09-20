use super::*;
impl DeploymentService {
    pub async fn list(
        &self,
        actor_id: ActorId,
        administrator: bool,
        filter: &DeploymentFilter,
    ) -> Result<Vec<DeploymentDetails>, DeploymentError> {
        self.store
            .list_authorized(actor_id, administrator, filter)
            .await
    }

    pub async fn get(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<DeploymentDetails, DeploymentError> {
        self.store.get_authorized(actor_id, administrator, id).await
    }

    pub async fn get_config(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<DeploymentConfig, DeploymentError> {
        self.get(actor_id, administrator, id)
            .await
            .map(|value| DeploymentConfig::from(&value))
    }

    pub async fn duplicate_draft(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<DeploymentDuplicateDraft, DeploymentError> {
        self.store
            .duplicate_draft(actor_id, administrator, id)
            .await
    }
}
