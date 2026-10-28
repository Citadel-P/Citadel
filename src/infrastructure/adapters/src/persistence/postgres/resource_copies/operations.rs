use super::*;
impl PostgresCopyRepository {
    pub(super) async fn save(&self, actor: ActorId, plan: &CopyPlan) -> Result<(), CopyError> {
        if !plan.preview.blockers.is_empty() || plan.preview.volumes.is_empty() {
            return Err(conflict("Copy review is incomplete."));
        }
        let access = self.access(actor).await?;
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        self.verify_source(&mut tx, &access, plan).await?;
        self.verify_snapshots(&mut tx, &access, plan).await?;
        self.authorize_current(&mut tx, &access, plan).await?;
        sqlx::query("INSERT INTO resourcecopies(id,actorid,kind,sourceid,name,platformid,plan) VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(plan.preview.id).bind(actor.value()).bind(plan.source.kind.as_str()).bind(plan.source.id).bind(&plan.request.name).bind(plan.target_platform()).bind(serde_json::to_value(plan).map_err(storage_error)?).execute(&mut *tx).await.map_err(storage_error)?;
        tx.commit().await.map_err(storage_error)
    }
    pub(super) async fn start_copy(
        &self,
        actor: ActorId,
        id: Uuid,
    ) -> Result<CopyProgress, CopyError> {
        let access = self.access(actor).await?;
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        let (mut plan, status) = owned_plan(&mut tx, actor, id).await?;
        if status != CopyStatus::Review {
            tx.rollback().await.map_err(storage_error)?;
            drop(access);
            return self.get_copy(actor, id).await;
        }
        let expired: bool = sqlx::query_scalar("SELECT createdat < CURRENT_TIMESTAMP-INTERVAL '30 minutes' FROM resourcecopies WHERE id=$1").bind(id).fetch_one(&mut *tx).await.map_err(storage_error)?;
        if expired {
            return Err(conflict("This review expired. Review the copy again."));
        }
        self.verify_source(&mut tx, &access, &plan).await?;
        self.verify_snapshots(&mut tx, &access, &plan).await?;
        self.authorize_current(&mut tx, &access, &plan).await?;
        // Lock repositories in UUID order, the same order for every copy. A
        // running prune/backup must settle before selected snapshots are pinned.
        let repositories = selected_snapshots(&plan)?
            .iter()
            .map(|s| s.repository_id)
            .collect::<std::collections::BTreeSet<_>>();
        for repository in repositories {
            sqlx::query(
                "SELECT id FROM backuprepositories WHERE id=$1 AND archivedat IS NULL FOR UPDATE",
            )
            .bind(repository)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage_error)?
            .ok_or(CopyError::NotFound)?;
            let busy: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM backuprepositoryleases WHERE backuprepositoryid=$1)",
            )
            .bind(repository)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage_error)?;
            if busy {
                return Err(conflict(
                    "A selected backup repository is busy. Wait for its operation to finish, then retry.",
                ));
            }
        }
        // A prune may have completed while we waited for its repository lock.
        // Recheck under that lock before committing the durable snapshot pin.
        self.verify_snapshots(&mut tx, &access, &plan).await?;
        if plan.request.data_mode == CopyDataMode::CurrentData {
            plan.current_volumes = plan
                .preview
                .volumes
                .iter()
                .map(|v| CopyRestore {
                    id: Uuid::now_v7(),
                    source: v.source.clone(),
                    target: v.target.clone(),
                    status: "Queued".into(),
                    message: None,
                })
                .collect();
            super::current::write_plan(&mut tx, id, &plan).await?;
        }
        for volume in plan
            .preview
            .volumes
            .iter()
            .filter(|_| plan.request.data_mode == CopyDataMode::ExistingBackup)
        {
            let selection = plan
                .request
                .selections
                .iter()
                .find(|s| s.volume == volume.source)
                .ok_or_else(|| conflict("Copy review is incomplete."))?;
            let snapshot = volume
                .snapshots
                .iter()
                .find(|s| {
                    s.backup_run_item_id == selection.backup_run_item_id
                        && s.backup_run_id == selection.backup_run_id
                })
                .ok_or_else(|| conflict("Copy snapshot is unavailable."))?;
            let restore = Uuid::now_v7();
            sqlx::query("INSERT INTO backuprestoreruns(id,backuprunid,backuprepositoryid,sourcebackuprunitemid,targetplatformid,targetdockernodeid,targetvolumename,overwriteexisting,targetvolumecreatedbycitadel,status,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,false,false,'Queued',$8)")
                .bind(restore).bind(snapshot.backup_run_id).bind(snapshot.repository_id).bind(snapshot.backup_run_item_id).bind(plan.target_platform()).bind(plan.target_node(volume)).bind(&volume.target).bind(actor.value()).execute(&mut *tx).await.map_err(storage_error)?;
            sqlx::query("INSERT INTO resourcecopyrestores(copyid,restoreid,sourcevolume,targetvolume) VALUES($1,$2,$3,$4)").bind(id).bind(restore).bind(&volume.source).bind(&volume.target).execute(&mut *tx).await.map_err(storage_error)?;
        }
        sqlx::query(
            "UPDATE resourcecopies SET status='Restoring',updatedat=CURRENT_TIMESTAMP WHERE id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if e.as_database_error()
                .is_some_and(|e| e.is_unique_violation())
            {
                conflict("Another copy is reserving this name. Choose another name.")
            } else {
                storage_error(e)
            }
        })?;
        sqlx::query("SELECT pg_notify('citadel_restore_work','')")
            .execute(&mut *tx)
            .await
            .map_err(storage_error)?;
        changes::publish(&mut tx, id).await.map_err(storage_error)?;
        tx.commit().await.map_err(storage_error)?;
        drop(access);
        self.get_copy(actor, id).await
    }
    pub(super) async fn get_copy(
        &self,
        actor: ActorId,
        id: Uuid,
    ) -> Result<CopyProgress, CopyError> {
        let _access = self.access(actor).await?;
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        // The operation belongs to its initiating actor. Source ACLs are checked
        // again for execution; progress remains available for cancellation.
        progress(&mut tx, actor, id).await
    }
    pub(super) async fn cleanup_copy(
        &self,
        actor: ActorId,
        id: Uuid,
    ) -> Result<CopyProgress, CopyError> {
        let access = self.access(actor).await?;
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        let (plan, status) = owned_plan(&mut tx, actor, id).await?;
        if !matches!(
            status,
            CopyStatus::Failed | CopyStatus::Cancelled | CopyStatus::Cleaning | CopyStatus::Cleaned
        ) {
            return Err(conflict(
                "Only stopped copies without a created workload can be cleaned up.",
            ));
        }
        self.require(
            &mut tx,
            &access,
            ResourceType::Platform,
            plan.target_platform(),
            PermissionLevel::Write,
            None,
        )
        .await?;
        if status != CopyStatus::Cleaned {
            sqlx::query("UPDATE resourcecopies SET status='Cleaning',message='Checking and removing unused destination volumes...',updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND destinationid IS NULL").bind(id).execute(&mut *tx).await.map_err(storage_error)?;
        }
        if status != CopyStatus::Cleaned {
            changes::publish(&mut tx, id).await.map_err(storage_error)?;
        }
        let value = progress(&mut tx, actor, id).await?;
        tx.commit().await.map_err(storage_error)?;
        Ok(value)
    }
    pub(super) async fn cancel_copy(
        &self,
        actor: ActorId,
        id: Uuid,
    ) -> Result<CopyProgress, CopyError> {
        let _access = self.access(actor).await?;
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        let (_, status) = owned_plan(&mut tx, actor, id).await?;
        if !status.terminal() && status != CopyStatus::Cleaning {
            sqlx::query("UPDATE resourcecopies SET status=CASE WHEN status='Review' THEN 'Cancelled' ELSE 'Cancelling' END,message='Cancellation requested.',updatedat=CURRENT_TIMESTAMP WHERE id=$1").bind(id).execute(&mut *tx).await.map_err(storage_error)?;
            cancel_queued(&mut tx, id).await?;
            changes::publish(&mut tx, id).await.map_err(storage_error)?;
        }
        let result = progress(&mut tx, actor, id).await?;
        tx.commit().await.map_err(storage_error)?;
        Ok(result)
    }
    async fn verify_snapshots(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        access: &Access,
        plan: &CopyPlan,
    ) -> Result<(), CopyError> {
        for snapshot in selected_snapshots(plan)? {
            self.require(
                tx,
                access,
                ResourceType::BackupPolicy,
                snapshot.policy_id,
                PermissionLevel::Execute,
                Some(SpecificPermission::Restore),
            )
            .await?;
            if plan.moves_storage() {
                let portable: bool = sqlx::query_scalar("SELECT type='S3Compatible' FROM backuprepositories WHERE id=$1 AND archivedat IS NULL")
                    .bind(snapshot.repository_id).fetch_optional(&mut **tx).await.map_err(storage_error)?.unwrap_or(false);
                if !portable {
                    return Err(conflict(
                        "Copies to another platform or node require backups in an S3-compatible repository reachable from the destination.",
                    ));
                }
            }
            let available:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM backuprunitems i JOIN backupruns r ON r.id=i.backuprunid JOIN backuprepositories br ON br.id=r.backuprepositoryid WHERE i.id=$1 AND r.id=$2 AND i.platformid=$3 AND i.volumename=$4 AND i.dockernodeid IS NOT DISTINCT FROM $5 AND i.hostpath IS NULL AND i.status='Succeeded' AND i.resticsnapshotid IS NOT NULL AND r.snapshotavailability='Available' AND r.status IN ('Succeeded','SucceededWithWarnings') AND br.archivedat IS NULL)")
                .bind(snapshot.backup_run_item_id).bind(snapshot.backup_run_id).bind(plan.source.platform_id).bind(&snapshot.volume).bind(&snapshot.docker_node_id).fetch_one(&mut **tx).await.map_err(storage_error)?;
            if !available {
                return Err(conflict(
                    "A selected snapshot is no longer available. Review the copy again.",
                ));
            }
        }
        Ok(())
    }
}
fn selected_snapshots(plan: &CopyPlan) -> Result<Vec<&CopySnapshot>, CopyError> {
    if plan.request.data_mode == CopyDataMode::CurrentData {
        return Ok(Vec::new());
    }
    plan.preview
        .volumes
        .iter()
        .map(|v| {
            let selection = plan
                .request
                .selections
                .iter()
                .find(|s| s.volume == v.source)
                .ok_or_else(|| conflict("Copy review is incomplete."))?;
            v.snapshots
                .iter()
                .find(|s| {
                    s.backup_run_id == selection.backup_run_id
                        && s.backup_run_item_id == selection.backup_run_item_id
                })
                .ok_or_else(|| conflict("Copy snapshot is unavailable."))
        })
        .collect()
}
pub(super) async fn owned_plan(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    id: Uuid,
) -> Result<(CopyPlan, CopyStatus), CopyError> {
    let row =
        sqlx::query("SELECT plan,status FROM resourcecopies WHERE id=$1 AND actorid=$2 FOR UPDATE")
            .bind(id)
            .bind(actor.value())
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage_error)?
            .ok_or(CopyError::NotFound)?;
    Ok((
        serde_json::from_value(row.try_get("plan").map_err(storage_error)?)
            .map_err(storage_error)?,
        serde_json::from_value(Value::String(row.try_get("status").map_err(storage_error)?))
            .map_err(storage_error)?,
    ))
}
pub(super) async fn cancel_queued(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(), CopyError> {
    let raw: Value = sqlx::query_scalar("SELECT plan FROM resourcecopies WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage_error)?;
    let mut plan: CopyPlan = serde_json::from_value(raw).map_err(storage_error)?;
    for job in &mut plan.current_volumes {
        if job.status == "Queued" {
            job.status = "Cancelled".into();
        }
    }
    super::current::write_plan(tx, id, &plan).await?;
    sqlx::query("UPDATE backuprestoreruns r SET status='Cancelled',completedat=CURRENT_TIMESTAMP,errorcode='CopyCancelled',errormessage='Resource copy cancelled.' FROM resourcecopyrestores c WHERE c.restoreid=r.id AND c.copyid=$1 AND r.status='Queued'").bind(id).execute(&mut **tx).await.map_err(storage_error)?;
    Ok(())
}
pub(super) async fn progress(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    id: Uuid,
) -> Result<CopyProgress, CopyError> {
    let row=sqlx::query("SELECT kind,sourceid,name,platformid,status,destinationid,message,plan FROM resourcecopies WHERE id=$1 AND actorid=$2").bind(id).bind(actor.value()).fetch_optional(&mut **tx).await.map_err(storage_error)?.ok_or(CopyError::NotFound)?;
    let mut restores=sqlx::query("SELECT r.id,c.sourcevolume,c.targetvolume,r.status,r.errormessage FROM resourcecopyrestores c JOIN backuprestoreruns r ON r.id=c.restoreid WHERE c.copyid=$1 ORDER BY c.sourcevolume").bind(id).fetch_all(&mut **tx).await.map_err(storage_error)?.into_iter().map(|r|Ok(CopyRestore { id:r.try_get("id").map_err(storage_error)?,source:r.try_get("sourcevolume").map_err(storage_error)?,target:r.try_get("targetvolume").map_err(storage_error)?,status:r.try_get("status").map_err(storage_error)?,message:r.try_get("errormessage").map_err(storage_error)? })).collect::<Result<Vec<_>,CopyError>>()?;
    let plan: CopyPlan = serde_json::from_value(row.try_get("plan").map_err(storage_error)?)
        .map_err(storage_error)?;
    restores.extend(plan.current_volumes);
    Ok(CopyProgress {
        data_mode: plan.request.data_mode,
        id,
        kind: serde_json::from_value(Value::String(row.try_get("kind").map_err(storage_error)?))
            .map_err(storage_error)?,
        source_id: row.try_get("sourceid").map_err(storage_error)?,
        name: row.try_get("name").map_err(storage_error)?,
        platform_id: row.try_get("platformid").map_err(storage_error)?,
        status: serde_json::from_value(Value::String(
            row.try_get("status").map_err(storage_error)?,
        ))
        .map_err(storage_error)?,
        destination_id: row.try_get("destinationid").map_err(storage_error)?,
        message: row.try_get("message").map_err(storage_error)?,
        restores,
    })
}
