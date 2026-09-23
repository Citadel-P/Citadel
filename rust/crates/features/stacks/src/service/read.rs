use super::*;
impl StackService {
    pub async fn list(
        &self,
        actor: ActorId,
        administrator: bool,
        filter: &StackFilter,
    ) -> Result<Vec<StackDetails>, StackError> {
        self.store
            .list_authorized(actor, administrator, filter)
            .await
    }

    pub async fn get(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<StackDetails, StackError> {
        self.store.get_authorized(actor, administrator, id).await
    }

    pub async fn get_config(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<StackConfig, StackError> {
        self.get(actor, administrator, id).await.map(Into::into)
    }

    pub async fn releases(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<Vec<StackReleaseDetails>, StackError> {
        self.store.releases(actor, administrator, id).await
    }

    pub async fn duplicate_draft(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> Result<crate::StackDuplicateDraft, StackError> {
        let stack = self.store.get_authorized(actor, administrator, id).await?;
        let mut spec = stack.spec.clone().ok_or(StackError::NotFound)?.for_create();
        if let StackSpec::Git {
            webhook: Some(webhook),
            ..
        } = &mut spec
        {
            webhook.secret = None;
        }
        spec.common_mut().project_name = None;
        Ok(crate::StackDuplicateDraft {
            draft: crate::StackDraft {
                name: format!("{} Copy", stack.name),
                platform_id: stack.platform_id.ok_or(StackError::NotFound)?,
                description: stack.description.clone(),
                stack_source: stack.stack_source,
                spec,
                drift_policy: stack.drift_policy.clone(),
                tag_ids: stack.tags.iter().map(|tag| tag.id).collect(),
                source_id: stack.id,
                source_name: stack.name.clone(),
            },
            warnings: Vec::new(),
        })
    }
}
