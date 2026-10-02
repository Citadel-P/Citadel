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
    citadel_registries::mask_registry_credentials(&mut masked);
    Ok(masked)
}
