use super::*;

impl PostgresUpdateRepository {
    pub(super) async fn save_review_impl<'a>(
        &'a self,
        actor: UpdateActor,
        plan: &'a UpdatePlan,
    ) -> Result<(), UpdateError> {
        plan.validate(Utc::now())?;
        let _fence = self.authorization.read_fence().await;
        let mut tx = self.pool.begin().await.map_err(storage)?;
        authorize(&mut tx, actor).await?;
        if instance(&mut tx).await? != plan.instance_id {
            return Err(UpdateError::Conflict(
                "Installation identity changed.".into(),
            ));
        }
        sqlx::query("DELETE FROM instanceupdateplans WHERE expiresat < CURRENT_TIMESTAMP")
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        sqlx::query(
            "INSERT INTO instanceupdateplans(id,actorid,expiresat,plan) VALUES($1,$2,$3,$4)",
        )
        .bind(plan.id)
        .bind(actor.actor_id.value())
        .bind(plan.expires_at)
        .bind(sqlx::types::Json(plan))
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
        tx.commit().await.map_err(storage)
    }

    pub(super) async fn review_impl(
        &self,
        actor: UpdateActor,
        id: Uuid,
    ) -> Result<UpdatePlan, UpdateError> {
        let _fence = self.authorization.read_fence().await;
        let mut tx = self.pool.begin().await.map_err(storage)?;
        authorize(&mut tx, actor).await?;
        let plan = sqlx::query_scalar::<_, sqlx::types::Json<UpdatePlan>>(
            "SELECT plan FROM instanceupdateplans WHERE id=$1 AND actorid=$2",
        )
        .bind(id)
        .bind(actor.actor_id.value())
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?
        .ok_or(UpdateError::NotFound)?
        .0;
        plan.validate(Utc::now())?;
        tx.commit().await.map_err(storage)?;
        Ok(plan)
    }
}
