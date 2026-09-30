use super::*;
use citadel_primitives::AuthorizedResource;
impl SwarmServiceService {
    pub async fn adoption_draft(
        &self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<crate::adoption::SwarmServiceAdoptionDraft, SwarmServiceError> {
        self.adoption
            .as_ref()
            .ok_or_else(|| SwarmServiceError::Runtime("Service adoption is unavailable.".into()))?
            .draft(actor, administrator, platform, id, cancel)
            .await
    }
    pub async fn adopt(
        &self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        id: &str,
        mut input: crate::adoption::AdoptSwarmService,
        cancel: &CancellationToken,
    ) -> Result<AuthorizedResource<crate::SwarmService>, SwarmServiceError> {
        normalize_name(&mut input.name)?;
        normalize_description(&mut input.description)?;
        if input.preview_fingerprint.len() != 64 || input.tag_ids.len() > 100 {
            return Err(validation("Invalid adoption fingerprint or Tags."));
        }
        input.tag_ids = unique_ids(&input.tag_ids);
        input.spec = input.spec.for_create();
        input.spec.webhook = None;
        input.spec.update_behavior = crate::UpdateBehavior::Disabled;
        input.spec.validate()?;
        let created = self
            .adoption
            .as_ref()
            .ok_or_else(|| SwarmServiceError::Runtime("Service adoption is unavailable.".into()))?
            .adopt(actor, administrator, platform, id, &input, cancel)
            .await?;
        self.notifier.changed(created.id, "created");
        Ok(created)
    }
}
