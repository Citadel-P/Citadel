//! One-time snapshot jobs belong to the durable copy plan, not backup policies.
use super::{operations::*, *};
impl PostgresCopyRepository {
    pub(super) async fn recover_current_jobs(&self) -> Result<(), CopyError> {
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        let rows = sqlx::query(r#"SELECT id,plan FROM resourcecopies WHERE status IN ('Restoring','Cancelling') AND plan->'request'->>'dataMode' = 'CurrentData' AND jsonb_path_exists(plan, '$.current_volumes[*] ? (@.status == "Running")') FOR UPDATE"#)
            .fetch_all(&mut *tx).await.map_err(storage_error)?;
        for row in rows {
            let id = row.try_get("id").map_err(storage_error)?;
            let mut plan: CopyPlan =
                serde_json::from_value(row.try_get("plan").map_err(storage_error)?)
                    .map_err(storage_error)?;
            let mut changed = false;
            for job in &mut plan.current_volumes {
                if job.status == "Running" {
                    job.status = "Failed".into();
                    job.message = Some("Copy interrupted. Its helper and partial destination may remain; use Clean up before starting a new copy. The snapshot was not replayed.".into());
                    changed = true;
                }
            }
            if changed {
                write_plan(&mut tx, id, &plan).await?;
            }
        }
        tx.commit().await.map_err(storage_error)
    }
    pub(super) async fn claim_current_job(
        &self,
        actor: ActorId,
        id: Uuid,
    ) -> Result<Option<(CopyPlan, CopyRestore)>, CopyError> {
        let access = self.access(actor).await?;
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        let (mut plan, status) = owned_plan(&mut tx, actor, id).await?;
        if status != CopyStatus::Restoring
            || plan.request.data_mode != CopyDataMode::CurrentData
            || plan
                .current_volumes
                .iter()
                .any(|v| !matches!(v.status.as_str(), "Queued" | "Succeeded"))
        {
            return Ok(None);
        }
        self.verify_source(&mut tx, &access, &plan).await?;
        self.authorize_current(&mut tx, &access, &plan).await?;
        let Some(job) = plan
            .current_volumes
            .iter_mut()
            .find(|v| v.status == "Queued")
        else {
            return Ok(None);
        };
        job.status = "Running".into();
        let job = job.clone();
        write_plan(&mut tx, id, &plan).await?;
        tx.commit().await.map_err(storage_error)?;
        Ok(Some((plan, job)))
    }
    pub(super) async fn finish_current_job(
        &self,
        actor: ActorId,
        id: Uuid,
        volume: Uuid,
        result: Result<(), String>,
    ) -> Result<(), CopyError> {
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        let (mut plan, status) = owned_plan(&mut tx, actor, id).await?;
        let job = plan
            .current_volumes
            .iter_mut()
            .find(|v| v.id == volume)
            .ok_or(CopyError::NotFound)?;
        if job.status == "Running" {
            match result {
                Ok(()) if status == CopyStatus::Restoring => {
                    job.status = "Succeeded".into();
                    job.message = None;
                }
                Ok(()) => {
                    job.status = "Cancelled".into();
                    job.message = Some("Copy cancelled; destination retained for cleanup.".into());
                }
                Err(error) => {
                    job.status = "Failed".into();
                    job.message = Some(error);
                }
            }
            write_plan(&mut tx, id, &plan).await?;
        }
        tx.commit().await.map_err(storage_error)
    }
    pub(super) async fn authorize_current(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        access: &Access,
        plan: &CopyPlan,
    ) -> Result<(), CopyError> {
        if plan.request.data_mode == CopyDataMode::CurrentData {
            // Same source-data permission as downloading volume contents.
            self.require(
                tx,
                access,
                ResourceType::Platform,
                plan.source.platform_id,
                PermissionLevel::Read,
                Some(SpecificPermission::Download),
            )
            .await?;
            if !plan.request.consistency_acknowledged {
                return Err(conflict(
                    "Confirm source consistency before copying current data.",
                ));
            }
        }
        Ok(())
    }
}
pub(super) async fn write_plan(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    plan: &CopyPlan,
) -> Result<(), CopyError> {
    let changed = sqlx::query("UPDATE resourcecopies SET plan=$2,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND plan IS DISTINCT FROM $2")
        .bind(id)
        .bind(serde_json::to_value(plan).map_err(storage_error)?)
        .execute(&mut **tx)
        .await
        .map_err(storage_error)?.rows_affected();
    if changed > 0 {
        changes::publish(tx, id).await.map_err(storage_error)?;
    }
    Ok(())
}
