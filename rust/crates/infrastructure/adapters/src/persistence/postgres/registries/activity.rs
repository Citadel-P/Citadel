use super::*;
pub(crate) async fn insert_registry_activity(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    id: Uuid,
    name: &str,
    info: ActivityEventInfo,
) -> Result<(), RegistryError> {
    let event = ActivityEvent::new_registry_event(id, name.to_owned(), actor, info, Utc::now())
        .map_err(|error| RegistryError::Storage(error.to_string()))?;
    insert_typed_activity(tx, &event)
        .await
        .map_err(|error| RegistryError::Storage(error.to_string()))
}

pub(super) fn registry_snapshot(
    value: &RegistryDetails,
) -> Result<RegistryActivitySnapshot, RegistryError> {
    Ok(RegistryActivitySnapshot {
        id: value.id,
        name: value.name.clone(),
        description: value.description.clone().unwrap_or_default(),
        registry_host: value.registry_host.clone(),
        status: value.status.as_str().to_owned(),
        configuration: masked_registry_configuration(&value.configuration)?,
    })
}

pub(super) fn masked_registry_configuration(value: &Value) -> Result<Value, RegistryError> {
    let mut masked = value.clone();
    let kind = registry_type(&masked)?;
    let keys: &[&str] = match kind.as_str() {
        "Gitlab" | "GitHub" | "DockerHub" => &["PAT"],
        "Custom" | "Azure" => &["Password"],
        "AWS" => &["AccessKey", "SecretAccessKey"],
        _ => &[],
    };
    if let Some(object) = masked.as_object_mut() {
        for key in keys {
            mask_property(object, key);
        }
    }
    Ok(masked)
}

pub(super) fn mask_property(object: &mut serde_json::Map<String, Value>, pascal_name: &str) {
    let camel_name = format!(
        "{}{}",
        pascal_name[..1].to_ascii_lowercase(),
        &pascal_name[1..]
    );
    for key in [pascal_name, camel_name.as_str()] {
        if let Some(value) = object.get_mut(key) {
            *value = if value.as_str().is_some_and(|value| !value.is_empty()) {
                Value::String("****************".to_owned())
            } else {
                Value::Null
            };
        }
    }
}
