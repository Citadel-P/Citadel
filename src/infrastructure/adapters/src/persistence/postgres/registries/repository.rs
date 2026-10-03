use super::*;
#[derive(Clone)]
pub struct PostgresRegistryRepository {
    pub(crate) pool: PgPool,
}
impl PostgresRegistryRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
impl RegistryRepository for PostgresRegistryRepository {
    fn find_id_by_name<'a>(
        &'a self,
        name: &'a str,
    ) -> BoxFuture<'a, Result<Option<Uuid>, RegistryError>> {
        Box::pin(async move {
            sqlx::query_scalar("SELECT id FROM registries WHERE lower(name)=lower($1)")
                .bind(name)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)
        })
    }

    fn list_registries<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<RegistryDetails>, RegistryError>> {
        Box::pin(async move {
            let query = authorized_catalog_query("registries", "registry", ResourceType::Registry);
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(ResourceType::Registry as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .bind(TaggableResourceType::Registry.as_database_str())
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_registry)
                .collect()
        })
    }

    fn get_registry<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<RegistryDetails, RegistryError>> {
        Box::pin(async move { get_registry(&self.pool, id).await })
    }

    fn create_registry<'a>(
        &'a self,
        actor_id: ActorId,
        registry: &'a NewRegistry,
    ) -> BoxFuture<'a, Result<RegistryDetails, RegistryError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let now = Utc::now();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            validate_tag_ids(&mut transaction, &registry.tag_ids).await?;
            sqlx::query(
                r#"
INSERT INTO registries (id, name, description, registryhost, status, createdat, createdbyactorid, configuration)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
"#,
            )
            .bind(id)
            .bind(&registry.name)
            .bind(registry.description.as_deref())
            .bind(&registry.registry_host)
            .bind(registry.status.as_str())
            .bind(now)
            .bind(actor_id.value())
            .bind(&registry.configuration)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;
            insert_resource_tags(
                &mut transaction,
                actor_id,
                TaggableResourceType::Registry,
                id,
                &registry.tag_ids,
            )
            .await?;
            let created = get_registry_tx(&mut transaction, id, false).await?;
            insert_registry_activity(
                &mut transaction,
                actor_id,
                id,
                &registry.name,
                ActivityEventInfo::registry_created(registry_snapshot(&created)?),
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            get_registry(&self.pool, id).await
        })
    }

    fn update_registry<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        patch: &'a RegistryPatch,
        kind: RegistryMutationKind,
    ) -> BoxFuture<'a, Result<RegistryDetails, RegistryError>> {
        Box::pin(async move {
            if id == DEFAULT_REGISTRY_ID {
                return Err(RegistryError::Conflict(
                    "The default Registry cannot be modified.".to_owned(),
                ));
            }
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let old = get_registry_tx(&mut transaction, id, true).await?;
            let mut updated = patch.apply_to(&old);
            match kind {
                RegistryMutationKind::Update => updated.validate()?,
                RegistryMutationKind::Metadata => {
                    validate_description(updated.description.as_deref())?;
                }
                RegistryMutationKind::Rename => {
                    validate_name_identifier(&updated.name, "Registry")?;
                    updated.name = updated.name.trim().to_owned();
                }
            }
            if patch.tag_ids.is_some() {
                validate_tag_ids(&mut transaction, &updated.tag_ids).await?;
            }
            let affected = sqlx::query(
                "UPDATE registries SET name=$2, description=$3, registryhost=$4, status=$5, configuration=$6 WHERE id=$1",
            )
            .bind(id)
            .bind(&updated.name)
            .bind(updated.description.as_deref())
            .bind(&updated.registry_host)
            .bind(updated.status.as_str())
            .bind(&updated.configuration)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?
            .rows_affected();
            exactly_one(affected)?;
            if patch.tag_ids.is_some() {
                replace_resource_tags_tx(
                    &mut transaction,
                    actor_id,
                    TaggableResourceType::Registry,
                    id,
                    &updated.tag_ids,
                )
                .await?;
            }
            if kind != RegistryMutationKind::Metadata {
                let updated_view = get_registry_tx(&mut transaction, id, false).await?;
                let info = if kind == RegistryMutationKind::Rename {
                    ActivityEventInfo::registry_renamed(old.name.clone(), updated.name.clone())
                } else {
                    ActivityEventInfo::registry_updated(
                        registry_snapshot(&old)?,
                        registry_snapshot(&updated_view)?,
                    )
                };
                insert_registry_activity(&mut transaction, actor_id, id, &updated.name, info)
                    .await?;
            }
            transaction.commit().await.map_err(storage)?;
            get_registry(&self.pool, id).await
        })
    }

    fn delete_registries<'a>(
        &'a self,
        actor_id: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), RegistryError>> {
        Box::pin(async move {
            let ids = unique_ids(ids);
            if ids.is_empty() {
                return Err(RegistryError::Validation(
                    "Ids must not be empty.".to_owned(),
                ));
            }
            if ids.contains(&DEFAULT_REGISTRY_ID) {
                return Err(RegistryError::Conflict(
                    "The default Registry cannot be deleted.".to_owned(),
                ));
            }
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let rows = load_registries_tx(&mut transaction, &ids).await?;
            if rows.len() != ids.len() {
                return Err(RegistryError::NotFound);
            }
            sqlx::query("DELETE FROM registries WHERE id = ANY($1::uuid[])")
                .bind(&ids)
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
            for registry in rows {
                insert_registry_activity(
                    &mut transaction,
                    actor_id,
                    registry.id,
                    &registry.name,
                    ActivityEventInfo::registry_deleted(registry_snapshot(&registry)?),
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
