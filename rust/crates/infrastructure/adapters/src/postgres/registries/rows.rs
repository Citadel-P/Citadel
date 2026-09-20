use super::*;
pub(super) fn map_registry(row: PgRow) -> Result<RegistryDetails, RegistryError> {
    let configuration: Value = row.try_get("configuration").map_err(storage)?;
    Ok(RegistryDetails {
        id: row.try_get("id").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        status: parse_registry_status(&row.try_get::<String, _>("status").map_err(storage)?)?,
        description: row.try_get("description").map_err(storage)?,
        registry_host: row.try_get("registryhost").map_err(storage)?,
        registry_type: registry_type(&configuration)?,
        configuration,
        created_at: row.try_get("createdat").map_err(storage)?,
        tags: serde_json::from_value(row.try_get("tags").map_err(storage)?).map_err(storage)?,
    })
}

pub(super) fn parse_registry_status(v: &str) -> Result<RegistryStatus, RegistryError> {
    match v {
        "Active" => Ok(RegistryStatus::Active),
        "Disabled" => Ok(RegistryStatus::Disabled),
        "Deprecated" => Ok(RegistryStatus::Deprecated),
        _ => Err(RegistryError::Storage(format!(
            "Unknown Registry status '{v}'."
        ))),
    }
}
