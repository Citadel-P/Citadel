use super::*;

impl PostgresAutomationRepository {
    pub(super) fn permissions_impl<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<
            std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>,
            AutomationError,
        >,
    > {
        Box::pin(async move {
            crate::persistence::postgres::permissions::levels_for_resources(
                &self.pool,
                actor,
                ResourceType::AutomationAction,
                ids,
            )
            .await
            .map_err(storage)
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn create_impl<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AutomationActionConfiguration,
    ) -> BoxFuture<'a, Result<AutomationAction, AutomationError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let mut tx = self.pool.begin().await.map_err(storage)?;
            sqlx::query("INSERT INTO actions(id,alertonfailure,code,createdbyactorid,defaultargsjson,description,enabled,name,runasactorid,schedulecron,scheduleenabled,scheduletimezone,timeoutseconds,webhook) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
                .bind(id).bind(input.alert_on_failure).bind(&input.code).bind(actor.value())
                .bind(parse_args(input.default_args_json.as_deref().unwrap_or("{}"))?)
                .bind(input.description.as_deref()).bind(input.enabled).bind(&input.name)
                .bind(input.run_as_actor_id.ok_or_else(|| AutomationError::Validation("Run-as Actor is required.".to_owned()))?)
                .bind(input.schedule_cron.as_deref()).bind(input.schedule_enabled)
                .bind(input.schedule_time_zone.as_deref().unwrap_or("UTC"))
                .bind(input.timeout_seconds.unwrap_or(60)).bind(input.webhook.as_ref().map(sqlx::types::Json))
                .execute(&mut *tx).await.map_err(database)?;
            crate::persistence::postgres::tags::links::insert(
                &mut tx,
                "AutomationAction",
                id,
                &input.tag_ids,
                actor.value(),
            )
            .await
            .map_err(|error| match error {
                crate::persistence::postgres::tags::links::ResourceTagError::Missing => {
                    AutomationError::Validation("One or more tags were not found.".into())
                }
                crate::persistence::postgres::tags::links::ResourceTagError::Database(error) => {
                    storage(error)
                }
            })?;
            let action = get_action_tx(&mut tx, id).await?;
            add_activity(
                &mut tx,
                &action,
                actor,
                ActivityEventInfo::ActionCreated {
                    action: action.snapshot(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(action)
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn list_impl(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AutomationAction>, AutomationError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT action.* FROM actions action
WHERE $4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=$2 AND access.resourceid=action.id
      AND access.permissionlevel = ANY($3)
)
ORDER BY action.name,action.id"#
            );
            let mut actions = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::AutomationAction as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_action)
                .collect::<Result<Vec<_>, _>>()?;
            enrich_actions(
                &mut *self.pool.acquire().await.map_err(storage)?,
                &mut actions,
            )
            .await?;
            Ok(actions)
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn get_impl<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<AutomationAction, AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            get_action_tx(&mut tx, id).await
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn update_impl<'a>(
        &'a self,
        current: &'a AutomationAction,
        input: &'a AutomationActionConfiguration,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<AutomationAction, AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let affected = sqlx::query("UPDATE actions SET alertonfailure=$2,code=$3,defaultargsjson=$4,description=$5,enabled=$6,runasactorid=$7,schedulecron=$8,scheduleenabled=$9,scheduletimezone=$10,timeoutseconds=$11,webhook=$12,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND rowversion=$13")
                .bind(current.id).bind(input.alert_on_failure).bind(&input.code)
                .bind(parse_args(input.default_args_json.as_deref().unwrap_or("{}"))?)
                .bind(input.description.as_deref()).bind(input.enabled)
                .bind(input.run_as_actor_id.ok_or_else(|| AutomationError::Validation("Run-as Actor is required.".to_owned()))?)
                .bind(input.schedule_cron.as_deref()).bind(input.schedule_enabled)
                .bind(input.schedule_time_zone.as_deref().unwrap_or("UTC"))
                .bind(input.timeout_seconds.unwrap_or(60)).bind(input.webhook.as_ref().map(sqlx::types::Json))
                .bind(current.row_version)
                .execute(&mut *tx).await.map_err(database)?.rows_affected();
            if affected == 0 {
                return Err(AutomationError::Conflict(
                    "The Automation Action has changed. Reload before saving.".into(),
                ));
            }
            let action = get_action_tx(&mut tx, current.id).await?;
            if !metadata_only {
                add_activity(
                    &mut tx,
                    &action,
                    actor,
                    ActivityEventInfo::ActionUpdated {
                        old_action: current.snapshot(),
                        new_action: action.snapshot(),
                    },
                )
                .await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(action)
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn rename_impl<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<AutomationAction, AutomationError>> {
        Box::pin(async move {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > 128 {
                return Err(AutomationError::Validation(
                    "Automation Action name must contain between 1 and 128 characters.".to_owned(),
                ));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let current = lock_action(&mut tx, id).await?;
            sqlx::query("UPDATE actions SET name=$2,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1")
                .bind(id).bind(name).execute(&mut *tx).await.map_err(database)?;
            let action = get_action_tx(&mut tx, id).await?;
            add_activity(
                &mut tx,
                &action,
                actor,
                ActivityEventInfo::ActionRenamed {
                    old_name: current.name,
                    new_name: action.name.clone(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(action)
        })
    }
}

impl PostgresAutomationRepository {
    pub(super) fn delete_impl<'a>(
        &'a self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<(), AutomationError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let action = lock_action(&mut tx, id).await?;
            if action.control_state != "Idle" {
                return Err(AutomationError::Conflict(
                    "Automation Action has an active run.".to_owned(),
                ));
            }
            sqlx::query("DELETE FROM actions WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            add_activity(
                &mut tx,
                &action,
                actor,
                ActivityEventInfo::ActionDeleted {
                    action: action.snapshot(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
