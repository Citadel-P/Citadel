use super::*;
pub(super) async fn load_bindings(
    pool: &PgPool,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
) -> Result<ResourceBindings, BindingError> {
    let entries=sqlx::query("SELECT * FROM resourcebindings WHERE scope=$1 AND resourceid IS NOT DISTINCT FROM $2 ORDER BY name,id")
        .bind(scope.as_database_str()).bind(resource_id).fetch_all(pool).await.map_err(storage)?.into_iter().map(|r|map_binding(r,false)).collect::<Result<Vec<_>,_>>()?;
    let mut effective = if scope == ResourceBindingScope::Global {
        Vec::new()
    } else {
        sqlx::query("SELECT * FROM resourcebindings WHERE scope='Global' AND resourceid IS NULL ORDER BY name,id")
        .fetch_all(pool).await.map_err(storage)?.into_iter().map(|r|map_binding(r,true)).collect::<Result<Vec<_>,_>>()?
    };
    for entry in &entries {
        effective.retain(|global| global.name != entry.name);
    }
    effective.extend(entries.iter().cloned());
    effective.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    Ok(ResourceBindings {
        entries,
        effective_entries: effective,
    })
}

pub(super) async fn ensure_binding_scope_exists(
    pool: &PgPool,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
) -> Result<(), BindingError> {
    let mut transaction = pool.begin().await.map_err(storage)?;
    ensure_binding_scope_exists_tx(&mut transaction, scope, resource_id).await?;
    transaction.rollback().await.map_err(storage)
}

pub(super) async fn ensure_binding_scope_exists_tx(
    transaction: &mut Transaction<'_, Postgres>,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
) -> Result<(), BindingError> {
    let Some(resource_id) = resource_id else {
        return if scope == ResourceBindingScope::Global {
            Ok(())
        } else {
            Err(BindingError::Validation(
                "Resource-scoped bindings require a resource ID.".to_owned(),
            ))
        };
    };
    let table = match scope {
        ResourceBindingScope::Global => {
            return Err(BindingError::Validation(
                "Global bindings cannot target a resource.".to_owned(),
            ));
        }
        ResourceBindingScope::Stack => "stacks",
        ResourceBindingScope::Deployment => "deployments",
        ResourceBindingScope::SwarmService => "swarmservices",
    };
    let query = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=$1)");
    let exists = sqlx::query_scalar::<_, bool>(AssertSqlSafe(query.as_str()))
        .bind(resource_id)
        .fetch_one(&mut **transaction)
        .await
        .map_err(storage)?;
    if exists {
        Ok(())
    } else {
        Err(BindingError::NotFound)
    }
}

pub(super) async fn get_secret_definition(
    pool: &PgPool,
    id: Uuid,
) -> Result<SecretDefinition, BindingError> {
    sqlx::query("SELECT * FROM secretdefinitions WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(BindingError::NotFound)
        .and_then(map_secret_definition)
}

pub(super) async fn get_secret_provider(
    pool: &PgPool,
    id: Uuid,
) -> Result<SecretProvider, BindingError> {
    sqlx::query("SELECT * FROM secretproviders WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(BindingError::NotFound)
        .and_then(map_secret_provider)
}

pub(super) async fn get_secret_provider_row_tx(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(String, Value), BindingError> {
    let row = sqlx::query("SELECT name,configuration FROM secretproviders WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?
        .ok_or(BindingError::NotFound)?;
    let name = row.try_get("name").map_err(storage)?;
    let config = serde_json::from_str(&row.try_get::<String, _>("configuration").map_err(storage)?)
        .map_err(storage)?;
    Ok((name, config))
}

pub(super) async fn validate_binding_references(
    tx: &mut Transaction<'_, Postgres>,
    kind: ResourceBindingKind,
    secret: Option<Uuid>,
) -> Result<(), BindingError> {
    if kind == ResourceBindingKind::Secret {
        let Some(id) = secret else {
            return Err(BindingError::Validation(
                "Secret binding requires a Secret.".into(),
            ));
        };
        if !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM secretdefinitions WHERE id=$1)",
        )
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?
        {
            return Err(BindingError::Validation("Secret does not exist.".into()));
        }
    }
    Ok(())
}

pub(crate) fn exactly_one(value: u64) -> Result<(), BindingError> {
    match value {
        1 => Ok(()),
        0 => Err(BindingError::NotFound),
        _ => Err(BindingError::Storage(
            "Mutation affected an unexpected number of rows.".into(),
        )),
    }
}

pub(crate) fn database_error(error: sqlx::Error) -> BindingError {
    if let sqlx::Error::Database(db) = &error {
        if db.code().as_deref() == Some("23505") {
            return BindingError::Conflict(
                "A resource with the same unique value already exists.".into(),
            );
        }
        if db.code().as_deref() == Some("23503") {
            return BindingError::Conflict("The resource is still in use.".into());
        }
    }
    storage(error)
}

pub(crate) fn storage(error: impl std::fmt::Display) -> BindingError {
    BindingError::Storage(error.to_string())
}
