use super::*;

impl ResourceBindingScope {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::Stack => "Stack",
            Self::Deployment => "Deployment",
            Self::SwarmService => "SwarmService",
        }
    }
}

pub fn validate_binding(input: BindingValidation<'_>) -> Result<(), BindingError> {
    validate_name(input.name, "Resource binding")?;
    if (input.scope == ResourceBindingScope::Global) != input.resource_id.is_none() {
        return Err(BindingError::Validation(
            "Global bindings cannot target a resource and resource bindings require one."
                .to_owned(),
        ));
    }
    match input.kind {
        ResourceBindingKind::Variable => {
            if input.value.is_none()
                || input.secret_id.is_some()
                || input.delivery.is_some()
                || input.target_path.is_some()
            {
                return Err(BindingError::Validation(
                    "Variable bindings require a value and cannot reference secret delivery metadata."
                        .to_owned(),
                ));
            }
        }
        ResourceBindingKind::Secret => {
            if input.value.is_some() || input.secret_id.is_none() || input.delivery.is_none() {
                return Err(BindingError::Validation(
                    "Secret bindings require a Secret and delivery mode and cannot store plaintext."
                        .to_owned(),
                ));
            }
            match input.delivery {
                Some(SecretDeliveryMode::EnvironmentVariable) if input.target_path.is_some() => {
                    return Err(BindingError::Validation(
                        "Environment variable Secrets cannot define a target path.".to_owned(),
                    ));
                }
                Some(SecretDeliveryMode::MountedFile) => {
                    validate_target_path(input.target_path)?;
                }
                Some(SecretDeliveryMode::NativePlatformSecret) => {
                    return Err(BindingError::Validation(
                        "Native platform Secret delivery is not supported yet.".to_owned(),
                    ));
                }
                _ => {}
            }
            if input.scope == ResourceBindingScope::Global
                && input.delivery != Some(SecretDeliveryMode::EnvironmentVariable)
            {
                return Err(BindingError::Validation(
                    "Only environment variable Secret delivery is supported for global bindings."
                        .to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_target_path(target_path: Option<&str>) -> Result<(), BindingError> {
    let Some(path) = target_path.filter(|path| !path.trim().is_empty()) else {
        return Err(BindingError::Validation(
            "Mounted file Secrets require a target path.".to_owned(),
        ));
    };
    let path = path.trim();
    if !path.starts_with('/')
        || path.contains('\\')
        || path == "/"
        || path.ends_with('/')
        || path.split('/').any(|segment| matches!(segment, "." | ".."))
        || matches!(path, "/etc/passwd" | "/etc/shadow")
        || ["/proc/", "/sys/", "/dev/"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
    {
        return Err(BindingError::Validation(
            "Mounted file Secret target path is not a safe absolute container file path."
                .to_owned(),
        ));
    }
    Ok(())
}

impl ExternalSecretInput {
    pub fn validate(&self) -> Result<(), BindingError> {
        validate_external_secret(
            &self.name,
            self.provider_id,
            &self.external_path,
            &self.external_key,
            self.external_version,
        )
    }
}

impl ExternalSecretPatch {
    pub fn merge(&self, current: &SecretDefinition) -> Result<ExternalSecretInput, BindingError> {
        let merged = ExternalSecretInput {
            name: self.name.clone().unwrap_or_else(|| current.name.clone()),
            provider_id: self.provider_id.or(current.provider_id).ok_or_else(|| {
                BindingError::Validation("External Secrets require a Secret provider.".to_owned())
            })?,
            external_path: self
                .external_path
                .clone()
                .or_else(|| current.external_path.clone())
                .unwrap_or_default(),
            external_key: self
                .external_key
                .clone()
                .or_else(|| current.external_key.clone())
                .unwrap_or_default(),
            external_version: self
                .external_version
                .merge_optional(current.external_version.as_ref()),
        };
        merged.validate()?;
        Ok(merged)
    }
}

pub(crate) fn validate_external_secret(
    name: &str,
    provider_id: Uuid,
    path: &str,
    key: &str,
    version: Option<i32>,
) -> Result<(), BindingError> {
    validate_name(name, "Secret")?;
    let mut chars = name.chars();
    let valid_name = chars
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric());
    if !valid_name {
        return Err(BindingError::Validation(
            "Secret name must be a valid environment variable name.".to_owned(),
        ));
    }
    if provider_id.is_nil()
        || path.trim().is_empty()
        || path.len() > 512
        || key.trim().is_empty()
        || key.len() > 256
        || version.is_some_and(|value| value <= 0)
    {
        return Err(BindingError::Validation(
            "External Secrets require a provider, path, and key within the supported limits; their version must be greater than zero.".to_owned(),
        ));
    }
    Ok(())
}

pub fn validate_provider(name: &str, address: &str, mount_path: &str) -> Result<(), BindingError> {
    validate_name(name, "Secret provider")?;
    let valid_address = address.starts_with("http://") || address.starts_with("https://");
    if !valid_address || mount_path.trim_matches('/').is_empty() {
        return Err(BindingError::Validation(
            "Vault address must be an absolute HTTP or HTTPS URL and mount path is required."
                .to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_bindings_never_accept_plaintext_or_unsafe_mounts() {
        let secret_id = Uuid::now_v7();
        let input = |value, delivery, target_path| BindingValidation {
            name: "TOKEN",
            kind: ResourceBindingKind::Secret,
            scope: ResourceBindingScope::Deployment,
            resource_id: Some(Uuid::now_v7()),
            value,
            secret_id: Some(secret_id),
            delivery,
            target_path,
        };
        assert!(
            validate_binding(input(
                Some("plaintext"),
                Some(SecretDeliveryMode::EnvironmentVariable),
                None,
            ))
            .is_err()
        );
        assert!(
            validate_binding(input(
                None,
                Some(SecretDeliveryMode::MountedFile),
                Some("/proc/token"),
            ))
            .is_err()
        );
        assert!(
            validate_binding(input(
                None,
                Some(SecretDeliveryMode::MountedFile),
                Some("/run/secrets/token"),
            ))
            .is_ok()
        );
    }

    #[test]
    fn global_bindings_are_environment_only_and_have_no_resource_id() {
        let valid = BindingValidation {
            name: "TOKEN",
            kind: ResourceBindingKind::Secret,
            scope: ResourceBindingScope::Global,
            resource_id: None,
            value: None,
            secret_id: Some(Uuid::now_v7()),
            delivery: Some(SecretDeliveryMode::EnvironmentVariable),
            target_path: None,
        };
        assert!(validate_binding(valid).is_ok());
        assert!(
            validate_binding(BindingValidation {
                delivery: Some(SecretDeliveryMode::MountedFile),
                target_path: Some("/run/secrets/token"),
                ..valid
            })
            .is_err()
        );
    }

    #[test]
    fn external_secret_patch_distinguishes_omitted_and_cleared_versions() {
        let current = SecretDefinition {
            id: Uuid::now_v7(),
            name: "TOKEN".into(),
            provider_type: SecretProviderType::VaultCompatibleKvV2,
            provider_id: Some(Uuid::now_v7()),
            external_path: Some("services/citadel".into()),
            external_key: Some("token".into()),
            external_version: Some(3),
            created_at: Utc::now(),
        };
        let omitted: ExternalSecretPatch = serde_json::from_str("{}").unwrap();
        assert_eq!(omitted.merge(&current).unwrap().external_version, Some(3));

        let cleared: ExternalSecretPatch =
            serde_json::from_str(r#"{"externalVersion":null}"#).unwrap();
        assert_eq!(cleared.merge(&current).unwrap().external_version, None);
    }
}
