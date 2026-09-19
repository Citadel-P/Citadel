use super::*;
impl SwarmServiceService {
    pub async fn delete(
        &self,
        actor_id: ActorId,
        administrator: bool,
        ids: &[Uuid],
    ) -> Result<(), SwarmServiceError> {
        let ids = unique_ids(ids);
        if ids.is_empty() {
            return Err(validation("Ids must not be empty."));
        }
        let claims = self.store.delete(actor_id, administrator, &ids).await?;
        let cancellation = self.shutdown.child_token();
        for claim in &claims {
            self.store.mark_delete_attempted(claim).await?;
            if let Some(docker_id) = &claim.docker_service_id
                && let Err(error) = self
                    .runtime
                    .delete(claim.platform_id, docker_id, &cancellation)
                    .await
            {
                if !matches!(
                    error,
                    SwarmServiceError::Runtime(_) | SwarmServiceError::Cancelled
                ) {
                    self.store.release_delete(&ids).await?;
                }
                return Err(error);
            }
            self.store
                .complete_delete(actor_id, std::slice::from_ref(claim))
                .await?;
            self.notifier.changed(claim.id, "deleted");
        }
        Ok(())
    }
}
