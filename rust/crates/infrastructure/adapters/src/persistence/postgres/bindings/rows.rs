use super::*;
pub(super) fn map_binding(row: PgRow, inherited: bool) -> Result<ResourceBinding, BindingError> {
    let kind = parse_binding_kind(&row.try_get::<String, _>("kind").map_err(storage)?)?;
    Ok(ResourceBinding {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        kind,
        scope: parse_scope(&row.try_get::<String, _>("scope").map_err(storage)?)?,
        resource_id: row.try_get("resourceid").map_err(storage)?,
        value: if kind == ResourceBindingKind::Variable {
            row.try_get("value").map_err(storage)?
        } else {
            None
        },
        secret_id: row.try_get("secretid").map_err(storage)?,
        secret_delivery_mode: row
            .try_get::<Option<String>, _>("secretdeliverymode")
            .map_err(storage)?
            .map(|v| parse_delivery(&v))
            .transpose()?,
        target_path: row.try_get("targetpath").map_err(storage)?,
        is_inherited: inherited,
    })
}

pub(super) fn map_secret_definition(row: PgRow) -> Result<SecretDefinition, BindingError> {
    Ok(SecretDefinition {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        provider_type: parse_provider_type(
            &row.try_get::<String, _>("providertype").map_err(storage)?,
        )?,
        provider_id: row.try_get("providerid").map_err(storage)?,
        external_path: row.try_get("externalpath").map_err(storage)?,
        external_key: row.try_get("externalkey").map_err(storage)?,
        external_version: row.try_get("externalversion").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}

pub(super) fn map_secret_provider(row: PgRow) -> Result<SecretProvider, BindingError> {
    let config: Value =
        serde_json::from_str(&row.try_get::<String, _>("configuration").map_err(storage)?)
            .map_err(storage)?;
    Ok(SecretProvider {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        provider_type: SecretProviderType::VaultCompatibleKvV2,
        address: config
            .get("Address")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        mount_path: config
            .get("MountPath")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}

pub(super) fn binding_kind(v: ResourceBindingKind) -> &'static str {
    match v {
        ResourceBindingKind::Variable => "Variable",
        ResourceBindingKind::Secret => "Secret",
    }
}

pub(super) fn parse_binding_kind(v: &str) -> Result<ResourceBindingKind, BindingError> {
    match v {
        "Variable" => Ok(ResourceBindingKind::Variable),
        "Secret" => Ok(ResourceBindingKind::Secret),
        _ => Err(BindingError::Storage(format!(
            "Unknown binding kind '{v}'."
        ))),
    }
}

pub(super) fn parse_scope(v: &str) -> Result<ResourceBindingScope, BindingError> {
    match v {
        "Global" => Ok(ResourceBindingScope::Global),
        "Stack" => Ok(ResourceBindingScope::Stack),
        "Deployment" => Ok(ResourceBindingScope::Deployment),
        "SwarmService" => Ok(ResourceBindingScope::SwarmService),
        _ => Err(BindingError::Storage(format!(
            "Unknown binding scope '{v}'."
        ))),
    }
}

pub(super) fn delivery_mode(v: SecretDeliveryMode) -> &'static str {
    match v {
        SecretDeliveryMode::EnvironmentVariable => "EnvironmentVariable",
        SecretDeliveryMode::MountedFile => "MountedFile",
        SecretDeliveryMode::NativePlatformSecret => "NativePlatformSecret",
    }
}

pub(super) fn parse_delivery(v: &str) -> Result<SecretDeliveryMode, BindingError> {
    match v {
        "EnvironmentVariable" => Ok(SecretDeliveryMode::EnvironmentVariable),
        "MountedFile" => Ok(SecretDeliveryMode::MountedFile),
        "NativePlatformSecret" => Ok(SecretDeliveryMode::NativePlatformSecret),
        _ => Err(BindingError::Storage(format!(
            "Unknown Secret delivery mode '{v}'."
        ))),
    }
}

pub(super) fn parse_provider_type(v: &str) -> Result<SecretProviderType, BindingError> {
    match v {
        "InternalEncrypted" => Ok(SecretProviderType::InternalEncrypted),
        "VaultCompatibleKvV2" => Ok(SecretProviderType::VaultCompatibleKvV2),
        _ => Err(BindingError::Storage(format!(
            "Unknown Secret provider type '{v}'."
        ))),
    }
}
