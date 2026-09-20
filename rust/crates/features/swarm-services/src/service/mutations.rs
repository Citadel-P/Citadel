use super::*;
impl SwarmServiceService {
    pub async fn create(
        &self,
        actor_id: ActorId,
        administrator: bool,
        mut input: CreateSwarmService,
    ) -> Result<SwarmServiceDetails, SwarmServiceError> {
        if input.duplicate_source.as_ref().is_some_and(|source| {
            source.resource_type != "SwarmService" || source.resource_id.is_nil()
        }) {
            return Err(validation("Duplicate source must be a Swarm Service."));
        }
        normalize_name(&mut input.name)?;
        normalize_description(&mut input.description)?;
        if input.platform_id.is_nil() {
            return Err(validation("A Platform must be selected."));
        }
        if input.tag_ids.len() > 100 {
            return Err(validation("A Service cannot contain more than 100 Tags."));
        }
        input.tag_ids = unique_ids(&input.tag_ids);
        input.spec = input.spec.for_create();
        if input.duplicate_source.is_some() {
            input.spec.webhook = None;
        }
        input.spec.validate()?;
        let created = self.store.create(actor_id, administrator, &input).await?;
        self.notifier.changed(created.id, "created");
        Ok(created)
    }

    pub async fn update(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: UpdateSwarmService,
    ) -> Result<SwarmServiceDetails, SwarmServiceError> {
        input.spec.validate()?;
        let current = self
            .store
            .get_authorized(actor_id, administrator, id)
            .await?;
        if current.spec.scheduling_mode != input.spec.scheduling_mode
            && current.applied_image_digest.is_some()
        {
            return Err(validation(
                "Service scheduling mode cannot change after the first successful Apply.",
            ));
        }
        let updated = self
            .store
            .update(actor_id, administrator, id, &input)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn update_description(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        description: Option<&str>,
    ) -> Result<SwarmServiceDetails, SwarmServiceError> {
        if id.is_nil() || description.is_some_and(|v| v.chars().count() > 600) {
            return Err(SwarmServiceError::Validation(
                "A valid Service id and a description of at most 600 characters are required."
                    .into(),
            ));
        }
        let updated = self
            .store
            .update_description(actor_id, administrator, id, description)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn rename(
        &self,
        actor_id: ActorId,
        administrator: bool,
        mut input: RenameSwarmService,
    ) -> Result<SwarmServiceDetails, SwarmServiceError> {
        normalize_name(&mut input.name)?;
        let updated = self.store.rename(actor_id, administrator, &input).await?;
        self.notifier.changed(input.id, "renamed");
        Ok(updated)
    }
    pub fn scale(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        replicas: i32,
    ) -> mpsc::Receiver<SwarmServiceProgressItem> {
        self.run_operation(
            actor_id,
            administrator,
            id,
            ServiceOperationKind::Scale,
            Some(replicas),
        )
    }
    pub fn force_update(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> mpsc::Receiver<SwarmServiceProgressItem> {
        self.run_operation(
            actor_id,
            administrator,
            id,
            ServiceOperationKind::ForceUpdate,
            None,
        )
    }

    pub(super) async fn resolve_environment(
        &self,
        claim: &ServiceOperationClaim,
    ) -> Result<(Vec<String>, Vec<String>, String), SwarmServiceError> {
        let referenced = referenced_binding_names(&claim.spec.environment)?;
        let resolved = self.bindings.resolve(claim.id, &referenced).await?;
        let selected = resolved
            .entries
            .iter()
            .filter(|entry| {
                referenced
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(&entry.name))
            })
            .collect::<Vec<_>>();
        for name in &referenced {
            if !selected
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case(name))
            {
                return Err(validation(&format!(
                    "Service references undefined Citadel variable or secret '{name}'."
                )));
            }
        }
        let mut environment = Vec::with_capacity(claim.spec.environment.len());
        for configured in &claim.spec.environment {
            if !configured.contains('=') && valid_binding_name(configured) {
                let binding = selected
                    .iter()
                    .find(|entry| entry.name.eq_ignore_ascii_case(configured))
                    .expect("referenced binding was validated");
                environment.push(format!("{configured}={}", binding.value.as_str()));
                continue;
            }
            let mut value = configured.clone();
            for binding in &selected {
                value = value.replace(&format!("${{{}}}", binding.name), binding.value.as_str());
            }
            environment.push(value);
        }
        let redaction_values = selected
            .iter()
            .filter(|entry| entry.secret)
            .map(|entry| entry.value.to_string())
            .collect::<Vec<_>>();
        let variables = selected.iter().filter(|entry| !entry.secret).count();
        let secrets = selected.iter().filter(|entry| entry.secret).count();
        let message = if variables == 0 && secrets == 0 {
            "No Citadel variables or secrets were referenced by this Service.".to_owned()
        } else {
            format!("Injected {variables} Citadel variable(s) and {secrets} secret(s).")
        };
        Ok((environment, redaction_values, message))
    }
}
