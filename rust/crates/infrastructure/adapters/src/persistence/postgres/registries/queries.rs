use super::*;
pub(super) const DEFAULT_REGISTRY_ID: Uuid = Uuid::from_u128(0x100);

pub(super) async fn get_registry(
    pool: &PgPool,
    id: Uuid,
) -> Result<RegistryDetails, RegistryError> {
    let q =
        format!("SELECT resource.*, {TAG_SUMMARIES} FROM registries resource WHERE resource.id=$2");
    sqlx::query(AssertSqlSafe(q.as_str()))
        .bind(TaggableResourceType::Registry.as_database_str())
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(RegistryError::NotFound)
        .and_then(map_registry)
}

pub(super) async fn get_registry_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    lock: bool,
) -> Result<RegistryDetails, RegistryError> {
    let lock_clause = if lock { " FOR UPDATE OF resource" } else { "" };
    let query = format!(
        "SELECT resource.*, {TAG_SUMMARIES} FROM registries resource WHERE resource.id=$2{lock_clause}"
    );
    sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(TaggableResourceType::Registry.as_database_str())
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(RegistryError::NotFound)
        .and_then(map_registry)
}

pub(super) async fn load_registries_tx(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<Vec<RegistryDetails>, RegistryError> {
    let q = format!(
        "SELECT resource.*, {TAG_SUMMARIES} FROM registries resource WHERE resource.id=ANY($2::uuid[]) ORDER BY resource.id"
    );
    sqlx::query(AssertSqlSafe(q.as_str()))
        .bind(TaggableResourceType::Registry.as_database_str())
        .bind(ids)
        .fetch_all(&mut **tx)
        .await
        .map_err(storage)?
        .into_iter()
        .map(map_registry)
        .collect()
}

pub(crate) fn unique_ids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub(crate) fn exactly_one(value: u64) -> Result<(), RegistryError> {
    match value {
        1 => Ok(()),
        0 => Err(RegistryError::NotFound),
        _ => Err(RegistryError::Storage(
            "Mutation affected an unexpected number of rows.".into(),
        )),
    }
}

pub(crate) fn database_error(error: sqlx::Error) -> RegistryError {
    if let sqlx::Error::Database(db) = &error {
        if db.code().as_deref() == Some("23505") {
            return RegistryError::Conflict(
                "A resource with the same unique value already exists.".into(),
            );
        }
        if db.code().as_deref() == Some("23503") {
            return RegistryError::Conflict("The resource is still in use.".into());
        }
    }
    storage(error)
}

pub(crate) fn storage(error: impl std::fmt::Display) -> RegistryError {
    RegistryError::Storage(error.to_string())
}
