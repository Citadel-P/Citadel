use super::*;
use citadel_primitives::AuthorizedResource;

impl DeploymentService {
    pub async fn adoption_draft(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        cancel: &CancellationToken,
    ) -> Result<crate::adoption::ContainerAdoptionDraft, DeploymentError> {
        self.adoption
            .as_ref()
            .ok_or_else(|| DeploymentError::Runtime("Container adoption is unavailable.".into()))?
            .draft(actor, administrator, id, cancel)
            .await
    }

    pub async fn adopt_container(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        mut input: crate::adoption::AdoptContainer,
        cancel: &CancellationToken,
    ) -> Result<AuthorizedResource<crate::Deployment>, DeploymentError> {
        normalize_name(&mut input.name)?;
        normalize_description(&mut input.description)?;
        if input.tag_ids.len() > 100 || input.preview_fingerprint.len() != 64 {
            return Err(DeploymentError::Validation(
                "Review the adoption draft and selected Tags before adopting.".into(),
            ));
        }
        input.tag_ids = unique_ids(&input.tag_ids);
        input.spec = input.spec.for_create();
        input.spec.validate()?;
        self.ensure_expansion_entitlements(None, &input.spec)
            .await?;
        let created = self
            .adoption
            .as_ref()
            .ok_or_else(|| DeploymentError::Runtime("Container adoption is unavailable.".into()))?
            .adopt(actor, administrator, id, input, cancel)
            .await?;
        self.notifier.adopted(&created);
        Ok(created)
    }
}
