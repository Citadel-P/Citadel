use super::*;

impl PostgresScriptRepository {
    pub(super) fn permissions_impl<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>, ScriptError>,
    > {
        Box::pin(async move {
            crate::persistence::postgres::permissions::levels_for_resources(
                &self.pool,
                actor,
                ResourceType::Script,
                ids,
            )
            .await
            .map_err(storage)
        })
    }
}

impl PostgresScriptRepository {
    pub(super) fn create_impl<'a>(
        &'a self,
        actor: ActorId,
        input: &'a ScriptConfiguration,
    ) -> BoxFuture<'a, Result<Script, ScriptError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let source = if let Some(source_id) = input.duplicate_source_id {
                let name: Option<String> =
                    sqlx::query_scalar("SELECT name FROM scripts WHERE id=$1 FOR SHARE")
                        .bind(source_id)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(storage)?;
                Some(citadel_activities::ActivitySourceResource {
                    resource_id: source_id,
                    resource_name: name.ok_or(ScriptError::NotFound)?,
                    resource_type: citadel_activities::ActivityResourceType::Script,
                })
            } else {
                None
            };

            sqlx::query("INSERT INTO scripts(id,alertonfailure,code,createdbyactorid,defaultargsjson,description,enabled,name,runasactorid,schedulecron,scheduleenabled,scheduletimezone,timeoutseconds,webhook) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
                .bind(id).bind(input.alert_on_failure).bind(&input.code).bind(actor.value())
                .bind(parse_args(input.default_args_json.as_deref().unwrap_or("{}"))?)
                .bind(input.description.as_deref()).bind(input.enabled).bind(&input.name)
                .bind(input.run_as_actor_id.ok_or_else(|| ScriptError::Validation("Run-as Actor is required.".to_owned()))?)
                .bind(input.schedule_cron.as_deref()).bind(input.schedule_enabled)
                .bind(input.schedule_time_zone.as_deref().unwrap_or("UTC"))
                .bind(input.timeout_seconds.unwrap_or(60)).bind(input.webhook.as_ref().map(sqlx::types::Json))
                .execute(&mut *tx).await.map_err(database)?;
            crate::persistence::postgres::tags::links::insert(
                &mut tx,
                "Script",
                id,
                &input.tag_ids,
                actor.value(),
            )
            .await
            .map_err(|error| match error {
                crate::persistence::postgres::tags::links::ResourceTagError::Missing => {
                    ScriptError::Validation("One or more tags were not found.".into())
                }
                crate::persistence::postgres::tags::links::ResourceTagError::Database(error) => {
                    storage(error)
                }
            })?;
            sqlx::query("UPDATE scripts SET executor=$2 WHERE id=$1")
                .bind(id)
                .bind(sqlx::types::Json(&input.executor))
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            let script = get_action_tx(&mut tx, id).await?;
            add_activity(
                &mut tx,
                &script,
                actor,
                ActivityEventInfo::ScriptCreated {
                    source,
                    script: script.snapshot(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(script)
        })
    }
}

impl PostgresScriptRepository {
    pub(super) fn list_impl(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<Script>, ScriptError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT script.* FROM scripts script
WHERE $4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=$2 AND access.resourceid=script.id
      AND access.permissionlevel = ANY($3)
)
ORDER BY script.name,script.id"#
            );
            let mut scripts = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Script as i32)
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
                &mut scripts,
            )
            .await?;
            Ok(scripts)
        })
    }
}

impl PostgresScriptRepository {
    pub(super) fn get_impl<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<Script, ScriptError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            get_action_tx(&mut tx, id).await
        })
    }
}

impl PostgresScriptRepository {
    pub(super) fn update_impl<'a>(
        &'a self,
        current: &'a Script,
        input: &'a ScriptConfiguration,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<Script, ScriptError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let affected = sqlx::query("UPDATE scripts SET alertonfailure=$2,code=$3,defaultargsjson=$4,description=$5,enabled=$6,runasactorid=$7,schedulecron=$8,scheduleenabled=$9,scheduletimezone=$10,timeoutseconds=$11,webhook=$12,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND rowversion=$13")
                .bind(current.id).bind(input.alert_on_failure).bind(&input.code)
                .bind(parse_args(input.default_args_json.as_deref().unwrap_or("{}"))?)
                .bind(input.description.as_deref()).bind(input.enabled)
                .bind(input.run_as_actor_id.ok_or_else(|| ScriptError::Validation("Run-as Actor is required.".to_owned()))?)
                .bind(input.schedule_cron.as_deref()).bind(input.schedule_enabled)
                .bind(input.schedule_time_zone.as_deref().unwrap_or("UTC"))
                .bind(input.timeout_seconds.unwrap_or(60)).bind(input.webhook.as_ref().map(sqlx::types::Json))
                .bind(current.row_version)
                .execute(&mut *tx).await.map_err(database)?.rows_affected();
            if affected == 0 {
                return Err(ScriptError::Conflict(
                    "The Script has changed. Reload before saving.".into(),
                ));
            }
            sqlx::query("UPDATE scripts SET executor=$2 WHERE id=$1")
                .bind(current.id)
                .bind(sqlx::types::Json(&input.executor))
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            let script = get_action_tx(&mut tx, current.id).await?;
            if !metadata_only {
                add_activity(
                    &mut tx,
                    &script,
                    actor,
                    ActivityEventInfo::ScriptUpdated {
                        old_script: current.snapshot(),
                        new_script: script.snapshot(),
                    },
                )
                .await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(script)
        })
    }
}

impl PostgresScriptRepository {
    pub(super) fn rename_impl<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<Script, ScriptError>> {
        Box::pin(async move {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > 128 {
                return Err(ScriptError::Validation(
                    "Script name must contain between 1 and 128 characters.".to_owned(),
                ));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let current = lock_action(&mut tx, id).await?;
            sqlx::query("UPDATE scripts SET name=$2,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1")
                .bind(id).bind(name).execute(&mut *tx).await.map_err(database)?;
            let script = get_action_tx(&mut tx, id).await?;
            add_activity(
                &mut tx,
                &script,
                actor,
                ActivityEventInfo::ScriptRenamed {
                    old_name: current.name,
                    new_name: script.name.clone(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(script)
        })
    }
}

impl PostgresScriptRepository {
    pub(super) fn delete_impl<'a>(
        &'a self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<(), ScriptError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let script = lock_action(&mut tx, id).await?;
            if script.control_state != citadel_primitives::ResourceControlState::Idle {
                return Err(ScriptError::Conflict(
                    "Script has an active run.".to_owned(),
                ));
            }
            sqlx::query("DELETE FROM scripts WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            add_activity(
                &mut tx,
                &script,
                actor,
                ActivityEventInfo::ScriptDeleted {
                    script: script.snapshot(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
