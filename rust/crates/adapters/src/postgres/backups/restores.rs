use super::*;

impl PostgresBackupPersistence {
    pub(super) fn enqueue_restore_impl(
        &self,
        request: BackupRestoreRequest,
    ) -> BoxFuture<'_, Result<BackupRestoreRun, BackupError>> {
        let name = request.target_volume_name.trim().to_owned();
        Box::pin(async move {
            if name.is_empty() {
                return Err(BackupError::Validation(
                    "Target Volume name is required.".into(),
                ));
            }
            let source = self.get_run(request.backup_run_id).await?;
            if source.status != "Succeeded" || source.snapshot_availability != "Available" {
                return Err(BackupError::Conflict(
                    "Backup snapshot is not available for restore.".into(),
                ));
            }
            let selected_item = match (source.items.as_slice(), request.source_backup_run_item_id) {
                ([], None) => None,
                ([item], None) => Some(item),
                (_, Some(id)) => Some(source.items.iter().find(|item| item.id == id).ok_or_else(
                    || {
                        BackupError::Validation(
                            "Source Backup Run Item does not belong to this Backup Run.".into(),
                        )
                    },
                )?),
                (_, None) => {
                    return Err(BackupError::Validation(
                        "A source Backup Run Item is required for a multi-Volume backup.".into(),
                    ));
                }
            };
            if selected_item
                .is_some_and(|item| item.status != "Succeeded" || item.restic_snapshot_id.is_none())
            {
                return Err(BackupError::Conflict(
                    "The selected Backup Run Item has no available snapshot.".into(),
                ));
            }
            let id = Uuid::now_v7();
            let mut tx = self.pool.begin().await.map_err(storage)?;
            sqlx::query("INSERT INTO backuprestoreruns(id,backuprunid,backuprepositoryid,sourcebackuprunitemid,targetplatformid,targetdockernodeid,targetvolumename,overwriteexisting,targetvolumecreatedbycitadel,status,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,false,'Queued',$9)").bind(id).bind(request.backup_run_id).bind(source.backup_repository_id).bind(selected_item.map(|item| item.id)).bind(request.target_platform_id).bind(request.target_docker_node_id).bind(name).bind(request.overwrite_existing).bind(request.actor.value()).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("SELECT pg_notify('citadel_restore_work','')")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            self.get_restore(id).await
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn claim_restore_impl(
        &self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<RestoreClaim>, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            recover_stale(&mut tx, stale_before).await?;
            let row=sqlx::query("SELECT r.id,r.backuprunid,r.backuprepositoryid,r.sourcebackuprunitemid,r.targetplatformid,r.targetdockernodeid,r.targetvolumename FROM backuprestoreruns r JOIN backuprepositories br ON br.id=r.backuprepositoryid WHERE r.status='Queued' AND br.archivedat IS NULL AND NOT EXISTS(SELECT 1 FROM backuprepositoryleases l WHERE l.backuprepositoryid=r.backuprepositoryid AND l.expiresat>CURRENT_TIMESTAMP) ORDER BY r.queuedat,r.id FOR UPDATE OF r,br SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await.map_err(storage)?;
            let Some(row) = row else {
                tx.commit().await.map_err(storage)?;
                return Ok(None);
            };
            let id: Uuid = row.try_get("id").map_err(storage)?;
            let source_id: Uuid = row.try_get("backuprunid").map_err(storage)?;
            let source_item_id: Option<Uuid> =
                row.try_get("sourcebackuprunitemid").map_err(storage)?;
            let repo_id: Uuid = row.try_get("backuprepositoryid").map_err(storage)?;
            let target_platform: Uuid = row.try_get("targetplatformid").map_err(storage)?;
            let target_node: Option<String> = row.try_get("targetdockernodeid").map_err(storage)?;
            let target_volume: String = row.try_get("targetvolumename").map_err(storage)?;
            let source_key =
                docker_volume_key(target_platform, target_node.as_deref(), &target_volume);
            let leased = sqlx::query("INSERT INTO backuprepositoryleases(backuprepositoryid,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,'Restore',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+INTERVAL '4 hours') ON CONFLICT(backuprepositoryid) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype='Restore',createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backuprepositoryleases.expiresat<=CURRENT_TIMESTAMP").bind(repo_id).bind(id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if leased == 0 {
                tx.rollback().await.map_err(storage)?;
                return Ok(None);
            }
            let source_leased = sqlx::query("INSERT INTO backupsourceleases(sourcekey,ownerrunid,operationtype,createdat,expiresat) VALUES($1,$2,'Restore',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP+INTERVAL '4 hours 5 minutes') ON CONFLICT(sourcekey) DO UPDATE SET ownerrunid=EXCLUDED.ownerrunid,operationtype='Restore',createdat=EXCLUDED.createdat,expiresat=EXCLUDED.expiresat WHERE backupsourceleases.expiresat<=CURRENT_TIMESTAMP")
                .bind(source_key).bind(id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if source_leased == 0 {
                tx.rollback().await.map_err(storage)?;
                return Ok(None);
            }
            let claimed = sqlx::query("UPDATE backuprestoreruns SET status='Running',startedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Queued'").bind(id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if claimed != 1 {
                return Err(BackupError::Storage(
                    "Backup Restore Run could not be claimed atomically.".into(),
                ));
            }
            sqlx::query("UPDATE backuprepositories SET controlstate='Processing',currentrunid=$2,controlstartedat=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1").bind(repo_id).bind(id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            let source = self.get_run(source_id).await?;
            let source_item = source_item_id
                .map(|id| {
                    source
                        .items
                        .iter()
                        .find(|item| item.id == id)
                        .cloned()
                        .ok_or_else(|| {
                            BackupError::Storage(
                                "Restore source Backup Run Item is missing.".into(),
                            )
                        })
                })
                .transpose()?;
            Ok(Some(RestoreClaim {
                repository: self.get_repository(repo_id).await?,
                run: self.get_restore(id).await?,
                source,
                source_item,
            }))
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn finish_restore_impl<'a>(
        &'a self,
        claim: &'a RestoreClaim,
        result: &'a RestoreExecutionResult,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let finished = sqlx::query("UPDATE backuprestoreruns SET status=$2,exitcode=$3,errorcode=$4,errormessage=$5,completedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Running'").bind(claim.run.id).bind(result.status).bind(result.exit_code).bind(&result.error_code).bind(&result.error_message).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if finished == 0 {
                tx.rollback().await.map_err(storage)?;
                return Ok(());
            }
            write_logs(
                &mut tx,
                "backuprestorerunlogs",
                "backuprestorerunid",
                claim.run.id,
                &result.logs,
            )
            .await?;
            release(&mut tx, claim.repository.id, claim.run.id, None, false).await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn list_restores_impl(
        &self,
        actor: ActorId,
        administrator: bool,
        backup_run_id: Option<Uuid>,
        policy_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BackupRestoreRun>, BackupError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT restore.* FROM backuprestoreruns restore
JOIN backupruns backup ON backup.id=restore.backuprunid
WHERE ($4::uuid IS NULL OR restore.backuprunid=$4)
  AND ($5::uuid IS NULL OR backup.backuppolicyid=$5)
  AND ($6 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=backup.backuppolicyid
        AND access.permissionlevel = ANY($3)
  ))
ORDER BY restore.queuedat DESC,restore.id DESC LIMIT $7"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BackupPolicy as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(backup_run_id)
                .bind(policy_id)
                .bind(administrator)
                .bind(i64::try_from(limit.clamp(1, 1000)).unwrap_or(1000))
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_restore)
                .collect()
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn get_restore_impl(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<BackupRestoreRun, BackupError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM backuprestoreruns WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(BackupError::NotFound)
                .and_then(map_restore)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn restore_logs_impl(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<BackupLog>, BackupError>> {
        logs(&self.pool, "backuprestorerunlogs", "backuprestorerunid", id)
    }
}

impl PostgresBackupPersistence {
    pub(super) fn cancel_restore_impl(&self, id: Uuid) -> BoxFuture<'_, Result<bool, BackupError>> {
        Box::pin(async move {
            let row=sqlx::query("UPDATE backuprestoreruns SET status='Cancelled',completedat=CURRENT_TIMESTAMP,errorcode='Cancelled',errormessage='Restore cancelled.' WHERE id=$1 AND status='Queued' RETURNING backuprepositoryid").bind(id).fetch_optional(&self.pool).await.map_err(storage)?;
            Ok(row.is_some())
        })
    }
}
