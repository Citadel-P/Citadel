use super::*;
use citadel_primitives::AuthorizedResource;
impl SwarmServiceService {
    pub async fn list(
        &self,
        actor_id: ActorId,
        administrator: bool,
        filter: &SwarmServiceFilter,
    ) -> Result<Vec<AuthorizedResource<crate::SwarmService>>, SwarmServiceError> {
        self.store
            .list_authorized(actor_id, administrator, filter)
            .await
    }

    pub async fn get(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<AuthorizedResource<crate::SwarmService>, SwarmServiceError> {
        self.store.get_authorized(actor_id, administrator, id).await
    }
    pub async fn duplicate_draft(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<crate::SwarmServiceDuplicateDraft, SwarmServiceError> {
        self.store.duplicate_draft(actor, administrator, id).await
    }
}
