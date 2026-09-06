use crate::{BuildError, BuildProjectInput, BuildProjectView};
use serde_json::Value;

impl BuildProjectView {
    pub fn apply_patch(
        &self,
        patch: Value,
        metadata_only: bool,
    ) -> Result<BuildProjectInput, BuildError> {
        const FIELDS: &[&str] = &[
            "description",
            "enabled",
            "gitRepositoryId",
            "branch",
            "contextPath",
            "dockerfilePath",
            "target",
            "buildArgs",
            "buildSecrets",
            "builderKind",
            "platformId",
            "buildAgentPoolId",
            "registryId",
            "imageRepository",
            "tagTemplates",
            "webhook",
            "timeoutSeconds",
            "retentionRunCount",
        ];
        let Value::Object(patch) = patch else {
            return Err(BuildError::Validation(
                "Build update must be an object.".into(),
            ));
        };
        if patch
            .keys()
            .any(|key| !FIELDS.contains(&key.as_str()) || (metadata_only && key != "description"))
        {
            return Err(BuildError::Validation(
                "Build update contains unsupported fields.".into(),
            ));
        }
        let mut value =
            serde_json::to_value(self).map_err(|error| BuildError::Storage(error.to_string()))?;
        let object = value.as_object_mut().expect("Project serializes as object");
        for (key, value) in patch {
            object.insert(key, value);
        }
        serde_json::from_value(value)
            .map_err(|error| BuildError::Validation(format!("Invalid Build update: {error}")))
    }

    pub fn snapshot(&self) -> citadel_domain::BuildProjectActivitySnapshot {
        let mut webhook = self.webhook.clone();
        if let Some(Value::Object(fields)) = &mut webhook {
            for (name, value) in fields {
                if name.eq_ignore_ascii_case("secret") && !value.is_null() {
                    *value = Value::String("********".into());
                }
            }
        }
        citadel_domain::BuildProjectActivitySnapshot {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            enabled: self.enabled,
            git_repository_id: self.git_repository_id,
            branch: self.branch.clone(),
            context_path: self.context_path.clone(),
            dockerfile_path: self.dockerfile_path.clone(),
            target: self.target.clone(),
            builder_kind: self.builder_kind.clone(),
            platform_id: self.platform_id,
            build_agent_pool_id: self.build_agent_pool_id,
            registry_id: self.registry_id,
            image_repository: self.image_repository.clone(),
            tag_templates: self.tag_templates.clone(),
            webhook,
            timeout_seconds: self.timeout_seconds,
            retention_run_count: self.retention_run_count,
            build_secrets: self
                .build_secrets
                .iter()
                .map(|s| citadel_domain::BuildSecretActivitySnapshot {
                    id: s.id.clone(),
                    secret_id: s.secret_id,
                })
                .collect(),
        }
    }
}
