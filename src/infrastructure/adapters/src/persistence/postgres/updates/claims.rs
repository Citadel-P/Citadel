use super::*;

impl PostgresUpdateRepository {
    pub(super) async fn claim_impl<'a>(
        &'a self,
        actor: UpdateActor,
        plan: &'a UpdatePlan,
        key: Uuid,
        now: DateTime<Utc>,
    ) -> Result<UpdateOperation, UpdateError> {
        let _fence = self.authorization.read_fence().await;
        let mut tx = self.pool.begin().await.map_err(storage)?;
        authorize(&mut tx, actor).await?;
        let instance = instance(&mut tx).await?;
        if instance != plan.instance_id {
            return Err(UpdateError::Conflict(
                "Installation identity changed.".into(),
            ));
        }
        if let Some(existing) = sqlx::query_scalar::<_, sqlx::types::Json<UpdateOperation>>(
            "SELECT operation FROM instanceupdates WHERE instanceid=$1 AND idempotencykey=$2",
        )
        .bind(instance)
        .bind(key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?
        {
            if existing.0.actor_id != actor.actor_id || existing.0.plan != *plan {
                return Err(UpdateError::Conflict(
                    "The request key belongs to another approval.".into(),
                ));
            }
            return Ok(existing.0);
        }
        let stored = sqlx::query_scalar::<_, sqlx::types::Json<UpdatePlan>>(
            "SELECT plan FROM instanceupdateplans WHERE id=$1 AND actorid=$2 FOR UPDATE",
        )
        .bind(plan.id)
        .bind(actor.actor_id.value())
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?
        .ok_or(UpdateError::NotFound)?
        .0;
        if stored != *plan {
            return Err(UpdateError::Conflict("The reviewed update changed.".into()));
        }
        if sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM instanceupdates WHERE instanceid=$1 AND ownsinstallation)",
        )
        .bind(instance)
        .fetch_one(&mut *tx)
        .await
        .map_err(storage)?
        {
            return Err(UpdateError::Conflict(
                "An update or unresolved recovery already owns this installation.".into(),
            ));
        }
        let operation = UpdateOperation::approve(plan.clone(), actor.actor_id, key, now)?;
        sqlx::query("INSERT INTO instanceupdates(id,instanceid,actorid,idempotencykey,revision,operation) VALUES($1,$2,$3,$4,0,$5)")
                .bind(operation.id).bind(instance).bind(actor.actor_id.value()).bind(key).bind(sqlx::types::Json(&operation)).execute(&mut *tx).await.map_err(storage)?;
        operation_activity(&mut tx, &operation).await?;
        tx.commit().await.map_err(storage)?;
        Ok(operation)
    }
}
