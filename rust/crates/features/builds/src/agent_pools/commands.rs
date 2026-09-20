use crate::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildAgentPoolConfiguration {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub provider_spec: Value,
    pub max_active_builders: Option<i32>,
    pub queue_timeout_seconds: Option<i32>,
    pub provisioning_timeout_seconds: Option<i32>,
    pub registration_timeout_seconds: Option<i32>,
    pub heartbeat_timeout_seconds: Option<i32>,
    pub cleanup_timeout_seconds: Option<i32>,
    pub maximum_instance_lifetime_seconds: Option<i32>,
    pub failure_retention_minutes: Option<i32>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl BuildAgentPoolConfiguration {
    pub fn validate(&mut self) -> Result<(), BuildError> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() || self.name.chars().count() > 128 {
            return Err(BuildError::Validation(
                "Build Agent Pool name must contain between 1 and 128 characters.".to_owned(),
            ));
        }
        self.description = self
            .description
            .take()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if self
            .description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 600)
        {
            return Err(BuildError::Validation(
                "Build Agent Pool description cannot exceed 600 characters.".into(),
            ));
        }
        let provider = self
            .provider_spec
            .get("$type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(provider, "AwsEc2" | "SelfManagedVm") {
            return Err(BuildError::Validation(
                "Build Agent Pool provider is unsupported.".to_owned(),
            ));
        }
        if provider == "SelfManagedVm" {
            pool_target(&self.provider_spec)?;
        }
        validate_range(
            self.max_active_builders.unwrap_or(1),
            1,
            100,
            "maximum active builders",
        )?;
        if self.tag_ids.len() > 100 {
            return Err(BuildError::Validation(
                "A Build Agent Pool cannot have more than 100 Tags.".to_owned(),
            ));
        }
        self.tag_ids.sort_unstable();
        self.tag_ids.dedup();
        validate_range(
            self.queue_timeout_seconds.unwrap_or(3600),
            60,
            86_400,
            "queue timeout",
        )?;
        validate_range(
            self.provisioning_timeout_seconds.unwrap_or(600),
            30,
            3600,
            "provisioning timeout",
        )?;
        validate_range(
            self.registration_timeout_seconds.unwrap_or(300),
            30,
            3600,
            "registration timeout",
        )?;
        validate_range(
            self.heartbeat_timeout_seconds.unwrap_or(90),
            10,
            3600,
            "heartbeat timeout",
        )?;
        validate_range(
            self.cleanup_timeout_seconds.unwrap_or(600),
            30,
            3600,
            "cleanup timeout",
        )?;
        validate_range(
            self.maximum_instance_lifetime_seconds.unwrap_or(7200),
            300,
            86_400,
            "maximum instance lifetime",
        )?;
        validate_range(
            self.failure_retention_minutes.unwrap_or(0),
            0,
            10_080,
            "failure retention",
        )?;
        Ok(())
    }
}

impl From<&BuildAgentPool> for BuildAgentPoolConfiguration {
    fn from(value: &BuildAgentPool) -> Self {
        Self {
            name: value.name.clone(),
            description: value.description.clone(),
            enabled: value.enabled,
            provider_spec: value.provider_spec.clone(),
            max_active_builders: Some(value.max_active_builders),
            queue_timeout_seconds: Some(value.queue_timeout_seconds),
            provisioning_timeout_seconds: Some(value.provisioning_timeout_seconds),
            registration_timeout_seconds: Some(value.registration_timeout_seconds),
            heartbeat_timeout_seconds: Some(value.heartbeat_timeout_seconds),
            cleanup_timeout_seconds: Some(value.cleanup_timeout_seconds),
            maximum_instance_lifetime_seconds: Some(value.maximum_instance_lifetime_seconds),
            failure_retention_minutes: Some(value.failure_retention_minutes),
            tag_ids: Vec::new(),
        }
    }
}
