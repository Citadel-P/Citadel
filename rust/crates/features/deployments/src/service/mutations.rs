use super::*;
use citadel_primitives::AuthorizedResource;
impl DeploymentService {
    pub async fn create(
        &self,
        actor_id: ActorId,
        administrator: bool,
        mut input: CreateDeployment,
    ) -> Result<AuthorizedResource<crate::Deployment>, DeploymentError> {
        normalize_name(&mut input.name)?;
        normalize_description(&mut input.description)?;
        if input.platform_id.is_nil() {
            return Err(DeploymentError::Validation(
                "A Platform must be selected.".to_owned(),
            ));
        }
        if input.tag_ids.len() > 100 {
            return Err(DeploymentError::Validation(
                "A Deployment cannot contain more than 100 Tags.".to_owned(),
            ));
        }
        input.tag_ids = unique_ids(&input.tag_ids);
        if let Some(source) = &input.duplicate_source
            && (source.resource_type != "Deployment" || source.resource_id.is_nil())
        {
            return Err(DeploymentError::Validation(
                "Duplicate source must identify a Deployment.".to_owned(),
            ));
        }
        input.spec = input.spec.for_create();
        input.spec.validate()?;
        self.ensure_expansion_entitlements(None, &input.spec)
            .await?;
        let created = self.store.create(actor_id, administrator, &input).await?;
        self.notifier.changed(created.id, "created");
        Ok(created)
    }

    pub async fn update_config(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        platform_id: Option<Uuid>,
        spec: Option<Value>,
    ) -> Result<AuthorizedResource<crate::Deployment>, DeploymentError> {
        let current = self
            .store
            .get_authorized(actor_id, administrator, id)
            .await?;
        if platform_id.is_some_and(|platform_id| platform_id != current.platform_id) {
            return Err(DeploymentError::Validation(
                "A Deployment cannot be moved to another Platform.".to_owned(),
            ));
        }
        let spec = match spec {
            Some(patch) => merge_spec(&current.spec, patch)?,
            None => current.spec.clone(),
        };
        let spec = spec.for_update(&current.spec);
        spec.validate()?;
        self.ensure_expansion_entitlements(Some(&current.spec), &spec)
            .await?;
        let updated = self
            .store
            .update_config(actor_id, administrator, id, current.row_version, &spec)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn update_metadata(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        mut input: UpdateDeploymentMetadata,
    ) -> Result<AuthorizedResource<crate::Deployment>, DeploymentError> {
        if let PatchField::Value(description) = &mut input.description {
            let mut value = Some(std::mem::take(description));
            normalize_description(&mut value)?;
            input.description = value.map_or(PatchField::Null, PatchField::Value);
        }
        let updated = self
            .store
            .update_metadata(actor_id, administrator, id, &input)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(updated)
    }

    pub async fn rename(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        mut name: String,
    ) -> Result<AuthorizedResource<crate::Deployment>, DeploymentError> {
        normalize_name(&mut name)?;
        let renamed = self
            .store
            .rename(actor_id, administrator, id, &name)
            .await?;
        self.notifier.changed(id, "updated");
        Ok(renamed)
    }

    pub(super) async fn ensure_expansion_entitlements(
        &self,
        current: Option<&DeploymentSpec>,
        proposed: &DeploymentSpec,
    ) -> Result<(), DeploymentError> {
        if expands_operational_guardrails(current, proposed) {
            self.require_entitlement(
                LicenseCapability::OperationalGuardrails,
                "operational-guardrails",
            )
            .await?;
        }
        if expands_automated_operations(current, proposed) {
            self.require_entitlement(
                LicenseCapability::AutomatedOperations,
                "automated-operations",
            )
            .await?;
        }
        Ok(())
    }

    async fn require_entitlement(
        &self,
        capability: LicenseCapability,
        name: &'static str,
    ) -> Result<(), DeploymentError> {
        if self.entitlements.enabled(capability).await? {
            Ok(())
        } else {
            Err(DeploymentError::LicenseRequired(name))
        }
    }
}

pub(super) fn expands_automated_operations(
    current: Option<&DeploymentSpec>,
    proposed: &DeploymentSpec,
) -> bool {
    let proposed_build = build_redeploy_target(proposed);
    let current_build = current.and_then(build_redeploy_target);
    proposed_build.is_some() && proposed_build != current_build
}

pub(super) fn expands_operational_guardrails(
    current: Option<&DeploymentSpec>,
    proposed: &DeploymentSpec,
) -> bool {
    proposed.update_behavior == crate::UpdateBehavior::AutoDeploy
        && current.is_none_or(|value| value.update_behavior != crate::UpdateBehavior::AutoDeploy)
}

pub(super) fn build_redeploy_target(spec: &DeploymentSpec) -> Option<Uuid> {
    match &spec.image {
        crate::DeploymentImageInfo::Build {
            build_project_id,
            redeploy_on_build: true,
            ..
        } => Some(*build_project_id),
        _ => None,
    }
}

pub(super) fn merge_spec(
    current: &DeploymentSpec,
    patch: Value,
) -> Result<DeploymentSpec, DeploymentError> {
    if !patch.is_object() {
        return Err(DeploymentError::Validation(
            "Deployment spec patch must be a JSON object.".to_owned(),
        ));
    }
    let mut merged = serde_json::to_value(current).map_err(|error| {
        DeploymentError::Storage(format!("failed to serialize Deployment spec: {error}"))
    })?;
    merge_json(&mut merged, &patch);
    serde_json::from_value(merged).map_err(|error| {
        DeploymentError::Validation(format!("Deployment spec patch is invalid: {error}"))
    })
}

use citadel_primitives::merge_json;

pub(super) fn normalize_name(name: &mut String) -> Result<(), DeploymentError> {
    *name = name.trim().to_owned();
    if name.is_empty() || name.len() > 64 {
        return Err(DeploymentError::Validation(
            "Deployment name must contain between 1 and 64 characters.".to_owned(),
        ));
    }
    if !name
        .bytes()
        .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_' | b'.'))
    {
        return Err(DeploymentError::Validation(
            "Deployment name contains unsupported characters.".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn normalize_description(
    description: &mut Option<String>,
) -> Result<(), DeploymentError> {
    *description = citadel_primitives::normalization::optional_text(description.take());
    if description.as_ref().is_some_and(|value| value.len() > 600) {
        return Err(DeploymentError::Validation(
            "Deployment description cannot exceed 600 characters.".to_owned(),
        ));
    }
    Ok(())
}
