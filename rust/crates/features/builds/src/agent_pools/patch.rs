use crate::BuildAgentPool;
use crate::BuildAgentPoolConfiguration;
use crate::BuildError;
use serde_json::Value;
impl BuildAgentPool {
    pub fn snapshot(&self) -> citadel_activities::BuildAgentPoolActivitySnapshot {
        let field = |camel: &str, pascal: &str| {
            self.provider_spec
                .get(camel)
                .or_else(|| self.provider_spec.get(pascal))
        };
        let text = |camel, pascal| {
            field(camel, pascal)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let (region, instance_type) = if self.provider == "SelfManagedVm" {
            (
                if text("connectionMode", "ConnectionMode") == "EdgeAgent" {
                    format!("edge-build-pool://{}", self.id)
                } else {
                    text("endpoint", "Endpoint")
                },
                format!(
                    "static-vm x{}",
                    field("maxWorkers", "MaxWorkers")
                        .and_then(Value::as_i64)
                        .unwrap_or(1)
                ),
            )
        } else {
            (
                text("region", "Region"),
                text("instanceType", "InstanceType"),
            )
        };
        citadel_activities::BuildAgentPoolActivitySnapshot {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            enabled: self.enabled,
            provider: self.provider.clone(),
            provider_spec: self.provider_spec.clone(),
            architecture: field("architecture", "Architecture")
                .and_then(Value::as_str)
                .unwrap_or("Amd64")
                .into(),
            region,
            instance_type,
            max_active_builders: self.max_active_builders,
            queue_timeout_seconds: self.queue_timeout_seconds,
            provisioning_timeout_seconds: self.provisioning_timeout_seconds,
            registration_timeout_seconds: self.registration_timeout_seconds,
            heartbeat_timeout_seconds: self.heartbeat_timeout_seconds,
            cleanup_timeout_seconds: self.cleanup_timeout_seconds,
            maximum_instance_lifetime_seconds: self.maximum_instance_lifetime_seconds,
            failure_retention_minutes: self.failure_retention_minutes,
            last_validation_status: self.last_validation_status.clone(),
            last_validation_message: self.last_validation_message.clone(),
            last_validated_at: self.last_validated_at,
        }
    }

    pub fn apply_patch(
        &self,
        patch: Value,
        metadata_only: bool,
    ) -> Result<BuildAgentPoolConfiguration, BuildError> {
        const FIELDS: &[&str] = &[
            "description",
            "enabled",
            "providerSpec",
            "maxActiveBuilders",
            "queueTimeoutSeconds",
            "provisioningTimeoutSeconds",
            "registrationTimeoutSeconds",
            "heartbeatTimeoutSeconds",
            "cleanupTimeoutSeconds",
            "maximumInstanceLifetimeSeconds",
            "failureRetentionMinutes",
        ];
        let Value::Object(patch) = patch else {
            return Err(BuildError::Validation(
                "Build Agent Pool update must be a JSON object.".into(),
            ));
        };
        let mut value = serde_json::to_value(BuildAgentPoolConfiguration::from(self))
            .map_err(|error| BuildError::Storage(error.to_string()))?;
        for (key, update) in patch {
            if !FIELDS.contains(&key.as_str()) || (metadata_only && key != "description") {
                return Err(BuildError::Validation(format!(
                    "Unsupported Build Agent Pool update field '{key}'."
                )));
            }
            // Nullable update fields retain their current value; an explicit
            // null description clears it. Provider specs are replaced as a unit.
            if !update.is_null() || key == "description" {
                value[&key] = update;
            }
        }
        let mut input: BuildAgentPoolConfiguration = serde_json::from_value(value)
            .map_err(|_| BuildError::Validation("Invalid Build Agent Pool update.".into()))?;
        input.validate()?;
        Ok(input)
    }
}
