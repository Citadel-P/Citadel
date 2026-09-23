use super::*;

impl PostgresBackupPersistence {
    pub(super) fn create_repository_impl<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupRepositoryConfiguration,
    ) -> BoxFuture<'a, Result<BackupRepository, BackupError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let kind = input
                .spec
                .get("$type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            sqlx::query("INSERT INTO backuprepositories(id,name,normalizedname,description,type,spec,passwordsecretid,status,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,'Unknown',$8)").bind(id).bind(&input.name).bind(input.name.to_uppercase()).bind(&input.description).bind(kind).bind(&input.spec).bind(input.password_secret_id).bind(actor.value()).execute(&self.pool).await.map_err(storage)?;
            self.get_repository(id).await
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn list_repositories_impl(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupRepository>, BackupError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT repository.* FROM backuprepositories repository
WHERE repository.archivedat IS NULL
  AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=repository.id
        AND access.permissionlevel = ANY($3)
  ))
ORDER BY repository.name,repository.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BackupRepository as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_repository)
                .collect()
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn get_repository_impl(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<BackupRepository, BackupError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM backuprepositories WHERE id=$1 AND archivedat IS NULL")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(BackupError::NotFound)
                .and_then(map_repository)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn update_repository_impl<'a>(
        &'a self,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<BackupRepository, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let current = sqlx::query(
                "SELECT * FROM backuprepositories WHERE id=$1 AND archivedat IS NULL FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(BackupError::NotFound)
            .and_then(map_repository)?;
            let input = repository_patch::apply(&current, patch)?;
            if input.spec != current.spec {
                let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND expiresat>CURRENT_TIMESTAMP)")
                    .bind(id).fetch_one(&mut *tx).await.map_err(storage)?;
                if active {
                    return Err(BackupError::Conflict(
                        "Backup Repository has an active operation.".into(),
                    ));
                }
            }
            let row = sqlx::query("UPDATE backuprepositories SET description=$2,spec=$3,type=$4,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 RETURNING *")
                .bind(id).bind(&input.description).bind(&input.spec).bind(input.spec["$type"].as_str().expect("validated repository type"))
                .fetch_one(&mut *tx).await.map_err(storage)?;
            let repository = map_repository(row)?;
            tx.commit().await.map_err(storage)?;
            Ok(repository)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn archive_repository_impl(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<(), BackupError>> {
        Box::pin(async move {
            let n=sqlx::query("UPDATE backuprepositories SET archivedat=CURRENT_TIMESTAMP,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 AND archivedat IS NULL AND controlstate='Idle' AND NOT EXISTS(SELECT 1 FROM backuppolicies WHERE backuprepositoryid=$1 AND archivedat IS NULL)").bind(id).execute(&self.pool).await.map_err(storage)?.rows_affected();
            if n == 0 {
                Err(BackupError::Conflict(
                    "Backup Repository is active, missing, or still referenced.".into(),
                ))
            } else {
                Ok(())
            }
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn record_repository_operation_impl<'a>(
        &'a self,
        id: Uuid,
        operation: &'a str,
        location: &'a str,
        platform_id: Option<Uuid>,
        succeeded: bool,
        message: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(BackupRepository, BackupRepositoryValidation), BackupError>> {
        Box::pin(async move {
            let status = if succeeded { "Ready" } else { "Unavailable" };
            let now = Utc::now();
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let changed = sqlx::query("UPDATE backuprepositories SET status=$2,lastcheckedat=CASE WHEN $3 IN('Validate','Initialize','Check') THEN $5 ELSE lastcheckedat END,lastprunedat=CASE WHEN $3='Prune' AND $4 THEN $5 ELSE lastprunedat END,updatedat=$5,rowversion=rowversion+1,controlstate='Idle',currentrunid=NULL,controlstartedat=NULL WHERE id=$1 AND archivedat IS NULL")
                .bind(id).bind(status).bind(operation).bind(succeeded).bind(now).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed == 0 {
                return Err(BackupError::NotFound);
            }
            let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM backuprepositoryvalidations WHERE backuprepositoryid=$1 AND location=$2 AND (($3::uuid IS NULL AND platformid IS NULL) OR platformid=$3) FOR UPDATE")
                .bind(id).bind(location).bind(platform_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            let validation_id = existing.unwrap_or_else(Uuid::now_v7);
            let error_code = (!succeeded).then_some("backup.repository_unavailable");
            if existing.is_some() {
                sqlx::query("UPDATE backuprepositoryvalidations SET status=$2,lastvalidatedat=$3,lasterrorcode=$4,lasterrormessage=$5 WHERE id=$1")
                    .bind(validation_id).bind(status).bind(now).bind(error_code).bind(message).execute(&mut *tx).await.map_err(storage)?;
            } else {
                sqlx::query("INSERT INTO backuprepositoryvalidations(id,backuprepositoryid,location,platformid,status,lastvalidatedat,lasterrorcode,lasterrormessage) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
                    .bind(validation_id).bind(id).bind(location).bind(platform_id).bind(status).bind(now).bind(error_code).bind(message).execute(&mut *tx).await.map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            if !succeeded {
                tracing::warn!(%id, %operation, error=message.unwrap_or("unknown"), "Backup Repository operation failed");
            }
            Ok((
                self.get_repository(id).await?,
                BackupRepositoryValidation {
                    id: validation_id,
                    backup_repository_id: id,
                    location: location.to_owned(),
                    platform_id,
                    status: status.to_owned(),
                    last_validated_at: now,
                    last_error_code: error_code.map(str::to_owned),
                    last_error_message: message.map(str::to_owned),
                },
            ))
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn acquire_repository_operation_impl(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
        operation: &str,
        expires_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, BackupError>> {
        let operation = operation.to_owned();
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            // All location changes and operation claims take the same row lock.
            sqlx::query(
                "SELECT id FROM backuprepositories WHERE id=$1 AND archivedat IS NULL FOR UPDATE",
            )
            .bind(repository_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(BackupError::NotFound)?;
            let affected = sqlx::query("INSERT INTO backuprepositoryleases(backuprepositoryid,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,$3,CURRENT_TIMESTAMP,$4) ON CONFLICT(backuprepositoryid) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype=EXCLUDED.operationtype,createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backuprepositoryleases.expiresat<=CURRENT_TIMESTAMP")
                .bind(repository_id)
                .bind(operation_id)
                .bind(operation)
                .bind(expires_at)
                .execute(&mut *tx)
                .await
                .map_err(storage)?
                .rows_affected();
            tx.commit().await.map_err(storage)?;
            Ok(affected == 1)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn release_repository_operation_impl(
        &self,
        repository_id: Uuid,
        operation_id: Uuid,
    ) -> BoxFuture<'_, Result<(), BackupError>> {
        Box::pin(async move {
            sqlx::query(
                "DELETE FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2",
            )
            .bind(repository_id)
            .bind(operation_id)
            .execute(&self.pool)
            .await
            .map_err(storage)?;
            Ok(())
        })
    }
}
