use super::*;
#[derive(Clone)]
pub struct PostgresBindingRepository {
    pub(crate) pool: PgPool,
}
impl PostgresBindingRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
impl BindingRepository for PostgresBindingRepository {
    fn get_bindings<'a>(
        &'a self,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindings, BindingError>> {
        Box::pin(async move {
            ensure_binding_scope_exists(&self.pool, scope, resource_id).await?;
            load_bindings(&self.pool, scope, resource_id).await
        })
    }

    fn create_binding<'a>(
        &'a self,
        binding: &'a NewResourceBinding,
    ) -> BoxFuture<'a, Result<ResourceBindings, BindingError>> {
        Box::pin(async move {
            let scope = binding
                .scope
                .ok_or_else(|| BindingError::Validation("Binding scope is required.".into()))?;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            ensure_binding_scope_exists_tx(&mut transaction, scope, binding.resource_id).await?;
            validate_binding_references(&mut transaction, binding.kind, binding.secret_id).await?;
            sqlx::query(r#"INSERT INTO resourcebindings
                (id,name,kind,scope,resourceid,value,secretid,secretdeliverymode,targetpath,createdat,updatedat)
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$10)"#)
                .bind(Uuid::now_v7()).bind(binding.name.trim()).bind(binding_kind(binding.kind))
                .bind(scope.as_database_str()).bind(binding.resource_id).bind(binding.value.as_deref())
                .bind(binding.secret_id).bind(binding.secret_delivery_mode.map(delivery_mode))
                .bind(binding.target_path.as_deref()).bind(Utc::now())
                .execute(&mut *transaction).await.map_err(database_error)?;
            transaction.commit().await.map_err(storage)?;
            load_bindings(&self.pool, scope, binding.resource_id).await
        })
    }

    fn update_binding<'a>(
        &'a self,
        binding: &'a ResourceBindingInput,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindings, BindingError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            ensure_binding_scope_exists_tx(&mut transaction, scope, resource_id).await?;
            validate_binding_references(&mut transaction, binding.kind, binding.secret_id).await?;
            let affected = sqlx::query(
                r#"UPDATE resourcebindings SET name=$4,kind=$5,value=$6,
                secretid=$7,secretdeliverymode=$8,targetpath=$9,updatedat=$10
                WHERE id=$1 AND scope=$2 AND resourceid IS NOT DISTINCT FROM $3"#,
            )
            .bind(binding.id)
            .bind(scope.as_database_str())
            .bind(resource_id)
            .bind(binding.name.trim())
            .bind(binding_kind(binding.kind))
            .bind(binding.value.as_deref())
            .bind(binding.secret_id)
            .bind(binding.secret_delivery_mode.map(delivery_mode))
            .bind(binding.target_path.as_deref())
            .bind(Utc::now())
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?
            .rows_affected();
            exactly_one(affected)?;
            transaction.commit().await.map_err(storage)?;
            load_bindings(&self.pool, scope, resource_id).await
        })
    }

    fn delete_binding<'a>(
        &'a self,
        id: Uuid,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindings, BindingError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            ensure_binding_scope_exists_tx(&mut transaction, scope, resource_id).await?;
            let secret_id = sqlx::query_scalar::<_, Option<Uuid>>(r#"DELETE FROM resourcebindings
                WHERE id=$1 AND scope=$2 AND resourceid IS NOT DISTINCT FROM $3 RETURNING secretid"#)
                .bind(id).bind(scope.as_database_str()).bind(resource_id)
                .fetch_optional(&mut *transaction).await.map_err(storage)?
                .ok_or(BindingError::NotFound)?;
            if let Some(secret_id) = secret_id {
                sqlx::query(r#"DELETE FROM secretdefinitions secret WHERE secret.id=$1
                    AND NOT EXISTS (SELECT 1 FROM resourcebindings binding WHERE binding.secretid=secret.id)"#)
                    .bind(secret_id).execute(&mut *transaction).await.map_err(storage)?;
            }
            transaction.commit().await.map_err(storage)?;
            load_bindings(&self.pool, scope, resource_id).await
        })
    }

    fn list_secret_definitions<'a>(
        &'a self,
        scope: Option<ResourceBindingScope>,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<Vec<SecretDefinition>, BindingError>> {
        Box::pin(async move {
            if scope.is_some() != resource_id.is_some() {
                return Err(BindingError::Validation(
                    "scope and resourceId must be provided together.".into(),
                ));
            }
            let rows = if let (Some(scope), Some(resource_id)) = (scope, resource_id) {
                sqlx::query(
                    r#"SELECT DISTINCT secret.* FROM secretdefinitions secret
                    JOIN resourcebindings binding ON binding.secretid=secret.id
                    WHERE (binding.scope='Global' AND binding.resourceid IS NULL)
                       OR (binding.scope=$1 AND binding.resourceid=$2)
                    ORDER BY secret.name, secret.id"#,
                )
                .bind(scope.as_database_str())
                .bind(resource_id)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
            } else {
                sqlx::query(
                    r#"SELECT DISTINCT secret.* FROM secretdefinitions secret
                    JOIN resourcebindings binding ON binding.secretid=secret.id
                    WHERE binding.scope='Global' AND binding.resourceid IS NULL
                    ORDER BY secret.name, secret.id"#,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
            };
            rows.into_iter().map(map_secret_definition).collect()
        })
    }

    fn create_internal_secret<'a>(
        &'a self,
        name: &'a str,
        protected_value: &'a str,
    ) -> BoxFuture<'a, Result<SecretDefinition, BindingError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let now = Utc::now();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            sqlx::query("INSERT INTO secretdefinitions (id,name,providertype,createdat,updatedat) VALUES ($1,$2,'InternalEncrypted',$3,$3)")
                .bind(id).bind(name).bind(now).execute(&mut *transaction).await.map_err(database_error)?;
            sqlx::query("INSERT INTO internalsecretvalues (secretid,encryptedvalue,createdat,updatedat) VALUES ($1,$2,$3,$3)")
                .bind(id).bind(protected_value).bind(now).execute(&mut *transaction).await.map_err(storage)?;
            transaction.commit().await.map_err(storage)?;
            get_secret_definition(&self.pool, id).await
        })
    }

    fn create_external_secret<'a>(
        &'a self,
        input: &'a ExternalSecretInput,
    ) -> BoxFuture<'a, Result<SecretDefinition, BindingError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let now = Utc::now();
            let affected=sqlx::query(r#"INSERT INTO secretdefinitions
                (id,name,providertype,providerid,externalpath,externalkey,externalversion,createdat,updatedat)
                SELECT $1,$2,'VaultCompatibleKvV2',$3,$4,$5,$6,$7,$7
                WHERE EXISTS (SELECT 1 FROM secretproviders WHERE id=$3)"#)
                .bind(id).bind(input.name.trim()).bind(input.provider_id).bind(input.external_path.trim())
                .bind(input.external_key.trim()).bind(input.external_version).bind(now)
                .execute(&self.pool).await.map_err(database_error)?.rows_affected();
            if affected == 0 {
                return Err(BindingError::Validation(
                    "Secret provider does not exist.".into(),
                ));
            }
            get_secret_definition(&self.pool, id).await
        })
    }

    fn update_external_secret<'a>(
        &'a self,
        id: Uuid,
        input: &'a ExternalSecretPatch,
    ) -> BoxFuture<'a, Result<SecretDefinition, BindingError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let current = sqlx::query("SELECT * FROM secretdefinitions WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(storage)?
                .ok_or(BindingError::NotFound)
                .and_then(map_secret_definition)?;
            if current.provider_type != SecretProviderType::VaultCompatibleKvV2 {
                return Err(BindingError::Validation(
                    "Only external Vault-compatible Secrets can be updated with this operation."
                        .to_owned(),
                ));
            }
            let merged = input.merge(&current)?;
            let provider_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM secretproviders WHERE id=$1 AND providertype='VaultCompatibleKvV2')",
            )
            .bind(merged.provider_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            if !provider_exists {
                return Err(BindingError::NotFound);
            }
            let affected = sqlx::query(
                r#"UPDATE secretdefinitions SET
                name=$2, providerid=$3, externalpath=$4, externalkey=$5,
                externalversion=$6, updatedat=$7
                WHERE id=$1 AND providertype='VaultCompatibleKvV2'"#,
            )
            .bind(id)
            .bind(merged.name.trim())
            .bind(merged.provider_id)
            .bind(merged.external_path.trim_matches('/'))
            .bind(merged.external_key.trim())
            .bind(merged.external_version)
            .bind(Utc::now())
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?
            .rows_affected();
            exactly_one(affected)?;
            transaction.commit().await.map_err(storage)?;
            get_secret_definition(&self.pool, id).await
        })
    }

    fn delete_secret_definition<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), BindingError>> {
        Box::pin(async move {
            exactly_one(
                sqlx::query("DELETE FROM secretdefinitions WHERE id=$1")
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(database_error)?
                    .rows_affected(),
            )
        })
    }

    fn list_secret_providers<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Vec<SecretProvider>, BindingError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM secretproviders ORDER BY name,id")
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_secret_provider)
                .collect()
        })
    }

    fn get_secret_provider<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<SecretProvider, BindingError>> {
        Box::pin(async move { get_secret_provider(&self.pool, id).await })
    }

    fn create_secret_provider<'a>(
        &'a self,
        input: &'a StoredSecretProviderInput,
    ) -> BoxFuture<'a, Result<SecretProvider, BindingError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let now = Utc::now();
            let config = json!({
                "Address":input.address,"MountPath":input.mount_path,"ProtectedToken":input.protected_token});
            sqlx::query("INSERT INTO secretproviders (id,name,providertype,configuration,createdat,updatedat) VALUES ($1,$2,'VaultCompatibleKvV2',$3,$4,$4)")
                .bind(id).bind(&input.name).bind(config.to_string()).bind(now).execute(&self.pool).await.map_err(database_error)?;
            get_secret_provider(&self.pool, id).await
        })
    }

    fn update_secret_provider<'a>(
        &'a self,
        id: Uuid,
        input: &'a StoredSecretProviderPatch,
    ) -> BoxFuture<'a, Result<SecretProvider, BindingError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let current = get_secret_provider_row_tx(&mut transaction, id).await?;
            let mut config = current.1;
            let object = config.as_object_mut().ok_or_else(|| {
                BindingError::Storage("Secret provider configuration is invalid.".into())
            })?;
            if let Some(value) = input.address.as_ref() {
                object.insert("Address".into(), Value::String(value.clone()));
            }
            if let Some(value) = input.mount_path.as_ref() {
                object.insert("MountPath".into(), Value::String(value.clone()));
            }
            if let Some(value) = input.protected_token.as_ref() {
                object.insert("ProtectedToken".into(), Value::String(value.clone()));
            }
            let affected=sqlx::query("UPDATE secretproviders SET name=COALESCE($2,name),configuration=$3,updatedat=$4 WHERE id=$1")
                .bind(id).bind(input.name.as_deref()).bind(config.to_string()).bind(Utc::now()).execute(&mut *transaction).await.map_err(database_error)?.rows_affected();
            exactly_one(affected)?;
            transaction.commit().await.map_err(storage)?;
            get_secret_provider(&self.pool, id).await
        })
    }

    fn delete_secret_provider<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), BindingError>> {
        Box::pin(async move {
            exactly_one(
                sqlx::query("DELETE FROM secretproviders WHERE id=$1")
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(database_error)?
                    .rows_affected(),
            )
        })
    }
}
