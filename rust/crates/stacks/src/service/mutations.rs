use super::*;
impl StackService {
    /// Evaluates the bounded set of active Stacks whose drift policy is enabled.
    /// A failure on one Stack is returned to the caller for diagnostics without
    /// preventing the remaining candidates from being checked.
    /// The daemon-event fast path shares the same reconciliation and persisted
    /// drift state as the sweep, but never reverses an intentional stop/pause.
    pub async fn monitor_container_event(&self, id: Uuid) -> Result<(), StackError> {
        if !self.operational_guardrails_enabled().await? {
            return Ok(());
        }
        let actor = ActorId::new(Uuid::from_u128(1));
        let stack = self.store.get_authorized(actor, true, id).await?;
        if !event_drift_eligible(&stack) {
            return Ok(());
        }
        let result = self.reconcile_drift(actor, true, id).await?;
        let report = result.after_report.unwrap_or(result.before_report);
        if let Some((status, info)) = drift_status_update(&stack, &report)
            && self.store.record_drift(&stack, status, info).await?
        {
            self.notifier.changed(id, "driftChanged");
        }
        Ok(())
    }

    pub async fn create(
        &self,
        actor: ActorId,
        administrator: bool,
        mut input: CreateStack,
    ) -> Result<StackDetails, StackError> {
        normalize_name(&mut input.name)?;
        normalize_description(&mut input.description)?;
        if input.platform_id.is_nil() {
            return Err(validation("A Platform must be selected."));
        }
        if input.stack_source != input.spec.source() {
            return Err(validation(
                "Stack source and specification type must match.",
            ));
        }
        input.spec = input.spec.for_create();
        input.spec.validate()?;
        input.drift_policy = Some(input.drift_policy.unwrap_or_default().normalized());
        input.tag_ids = normalize_tags(&input.tag_ids);
        if input.tag_ids.len() > 100 {
            return Err(validation("A Stack cannot contain more than 100 Tags."));
        }
        let created = self.store.create(actor, administrator, &input).await?;
        self.notifier.changed(created.id, "created");
        Ok(created)
    }

    pub async fn update(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        mut input: UpdateStack,
    ) -> Result<StackDetails, StackError> {
        if let Some(name) = input.name.as_mut() {
            normalize_name(name)?;
        }
        if let Some(description) = input.description.as_mut() {
            normalize_description(description)?;
        }
        let current = self.store.get_authorized(actor, administrator, id).await?;
        let expected = input.row_version.unwrap_or(current.row_version);
        let mut api = serde_json::to_value(current.spec.as_ref().ok_or_else(|| {
            StackError::Storage("Stack has no current specification.".to_owned())
        })?)
        .map_err(|error| StackError::Storage(error.to_string()))?;
        if let Some(patch) = &input.spec {
            merge_json(&mut api, patch);
        }
        let spec: StackSpec = serde_json::from_value(api)
            .map_err(|error| validation(&format!("Invalid Stack specification: {error}")))?;
        if input
            .stack_source
            .is_some_and(|source| source != spec.source())
            || current.stack_source != spec.source()
        {
            return Err(validation("A Stack source type cannot be changed."));
        }
        spec.validate()?;
        let updated = self
            .store
            .update(actor, administrator, id, expected, &input, &spec)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn update_metadata(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        mut description: Option<String>,
    ) -> Result<StackDetails, StackError> {
        normalize_description(&mut description)?;
        let updated = self
            .store
            .update_metadata(actor, administrator, id, description.as_deref())
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn rename(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        mut name: String,
    ) -> Result<StackDetails, StackError> {
        normalize_name(&mut name)?;
        let updated = self.store.rename(actor, administrator, id, &name).await?;
        self.notifier.changed(id, "renamed");
        Ok(updated)
    }
}
