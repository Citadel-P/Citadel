use super::*;

impl PostgresAlertRepository {
    pub(super) fn list_channels_impl(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AlertChannel>, AlertError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT channel.* FROM alertchannels channel
WHERE $4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=$2 AND access.resourceid=channel.id
      AND access.permissionlevel = ANY($3)
)
ORDER BY channel.name,channel.id"#
            );
            let rows = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::AlertChannel as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            rows.into_iter().map(map_channel).collect()
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn get_channel_impl(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<AlertChannel, AlertError>> {
        Box::pin(async move {
            let row = sqlx::query("SELECT * FROM alertchannels WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(AlertError::NotFound)?;
            map_channel(row)
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn create_channel_impl<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AlertChannelConfiguration,
    ) -> BoxFuture<'a, Result<AlertChannel, AlertError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let mut tx = self.pool.begin().await.map_err(storage)?;
            sqlx::query("INSERT INTO alertchannels(id,name,alertdestination,url,isactive,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6)")
                .bind(id).bind(&input.name).bind(&input.alert_destination).bind(&input.url).bind(input.is_active).bind(actor.value())
                .execute(&mut *tx).await.map_err(storage)?;
            self.commit_configuration(tx).await?;
            self.get_channel(id).await
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn update_channel_impl<'a>(
        &'a self,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<AlertChannel, AlertError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query("SELECT * FROM alertchannels WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(AlertError::NotFound)?;
            let input =
                citadel_alerts::configuration_patch::apply_channel(&map_channel(row)?, patch)?;
            let changed = sqlx::query("UPDATE alertchannels SET name=$2,alertdestination=$3,url=$4,isactive=$5 WHERE id=$1")
                .bind(id).bind(&input.name).bind(&input.alert_destination).bind(&input.url).bind(input.is_active)
                .execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed == 0 {
                return Err(AlertError::NotFound);
            }
            self.commit_configuration(tx).await?;
            self.get_channel(id).await
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn delete_channels_impl<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async move {
            let ids = normalized_ids(ids, "Alert Channel")?;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            require_complete_set(&mut tx, "alertchannels", &ids).await?;
            sqlx::query("DELETE FROM alertchannels WHERE id = ANY($1)")
                .bind(&ids)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            self.commit_configuration(tx).await?;
            Ok(())
        })
    }
}
