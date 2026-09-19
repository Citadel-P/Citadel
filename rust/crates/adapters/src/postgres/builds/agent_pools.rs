use super::*;

impl PostgresBuildRepository {
    pub(super) fn health_pools_impl(
        &self,
        after: Uuid,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BuildAgentPool>, BuildError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM buildagentpools WHERE id>$1 AND enabled AND archivedat IS NULL AND provider='SelfManagedVm' AND (controlstate='Idle' OR controlstartedat < EXTRACT(EPOCH FROM now())::bigint-180) ORDER BY id LIMIT $2")
                .bind(after).bind(limit.clamp(1,64) as i64).fetch_all(&self.pool).await.map_err(storage)?.into_iter().map(map_pool).collect()
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn record_pool_health_impl<'a>(
        &'a self,
        pool: &'a BuildAgentPool,
        result: &'a citadel_builds::BuildPoolCheck,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        Box::pin(async move {
            let message: String = result.message.chars().take(2048).collect();
            let count = sqlx::query("UPDATE buildagentpools SET lastvalidationstatus=$3,lastvalidationmessage=$4,lastvalidatedat=now(),controlstate='Idle',controltriggeredby=NULL,controlstartedat=NULL,updatedat=now(),rowversion=rowversion+1 WHERE id=$1 AND rowversion=$2 AND enabled AND archivedat IS NULL AND (controlstate='Idle' OR controlstartedat < EXTRACT(EPOCH FROM now())::bigint-180) AND (lastvalidationstatus IS DISTINCT FROM $3 OR lastvalidationmessage IS DISTINCT FROM $4 OR lastvalidatedat IS NULL OR lastvalidatedat < now()-INTERVAL '5 minutes' OR controlstate<>'Idle')")
                .bind(pool.id).bind(pool.row_version).bind(if result.ready {"Ready"} else {"Invalid"}).bind(message).execute(&self.pool).await.map_err(storage)?.rows_affected();
            Ok(count == 1)
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn create_pool_impl<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildAgentPoolConfiguration,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let provider = input
                .provider_spec
                .get("$type")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    BuildError::Validation("Build Agent Pool provider is required.".to_owned())
                })?;
            sqlx::query("INSERT INTO buildagentpools(id,name,normalizedname,description,enabled,provider,providerspec,maxactivebuilders,queuetimeoutseconds,provisioningtimeoutseconds,registrationtimeoutseconds,heartbeattimeoutseconds,cleanuptimeoutseconds,maximuminstancelifetimeseconds,failureretentionminutes,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)")
                .bind(id).bind(&input.name).bind(input.name.to_uppercase()).bind(input.description.as_deref()).bind(input.enabled).bind(provider).bind(&input.provider_spec)
                .bind(input.max_active_builders.unwrap_or(1)).bind(input.queue_timeout_seconds.unwrap_or(3600)).bind(input.provisioning_timeout_seconds.unwrap_or(600))
                .bind(input.registration_timeout_seconds.unwrap_or(300)).bind(input.heartbeat_timeout_seconds.unwrap_or(90)).bind(input.cleanup_timeout_seconds.unwrap_or(600))
                .bind(input.maximum_instance_lifetime_seconds.unwrap_or(7200)).bind(input.failure_retention_minutes.unwrap_or(0)).bind(actor.value())
                .execute(&mut *transaction).await.map_err(database)?;
            resource_tags::insert(
                &mut transaction,
                "BuildAgentPool",
                id,
                &input.tag_ids,
                actor.value(),
            )
            .await
            .map_err(build_tag_error)?;
            let pool = sqlx::query("SELECT * FROM buildagentpools WHERE id=$1")
                .bind(id)
                .fetch_one(&mut *transaction)
                .await
                .map_err(storage)
                .and_then(map_pool)?;
            record_pool_activity(
                &mut transaction,
                &pool,
                actor,
                citadel_domain::ActivityEventInfo::BuildAgentPoolCreated {
                    pool: pool.snapshot(),
                },
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            self.get_pool(pool.id).await
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn list_pools_impl(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildAgentPool>, BuildError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT pool.* FROM buildagentpools pool
WHERE pool.archivedat IS NULL
  AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=pool.id
        AND access.permissionlevel = ANY($3)
  ))
ORDER BY pool.name,pool.id"#
            );
            let mut values = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BuildAgentPool as i32)
                .bind(citadel_domain::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_pool)
                .collect::<Result<Vec<_>, _>>()?;
            enrich_pools(
                &mut *self.pool.acquire().await.map_err(storage)?,
                &mut values,
            )
            .await?;
            Ok(values)
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn pool_permissions_impl<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_domain::PermissionLevel>, BuildError>,
    > {
        Box::pin(async move {
            crate::resource_permissions::levels_for_resources(
                &self.pool,
                actor,
                ResourceType::BuildAgentPool,
                ids,
            )
            .await
            .map_err(storage)
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn get_pool_impl<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        Box::pin(async move {
            let mut value =
                sqlx::query("SELECT * FROM buildagentpools WHERE id=$1 AND archivedat IS NULL")
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(storage)?
                    .ok_or(BuildError::NotFound)
                    .and_then(map_pool)?;
            enrich_pools(
                &mut *self.pool.acquire().await.map_err(storage)?,
                std::slice::from_mut(&mut value),
            )
            .await?;
            Ok(value)
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn claim_pool_test_impl(
        &self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'_, Result<BuildAgentPool, BuildError>> {
        Box::pin(async move {
            // Reclaim a crashed check only after its bounded 120-second deadline.
            sqlx::query("UPDATE buildagentpools SET controlstate='Processing',controltriggeredby=$2,controlstartedat=EXTRACT(EPOCH FROM now())::bigint,updatedat=now(),rowversion=rowversion+1 WHERE id=$1 AND archivedat IS NULL AND (controlstate='Idle' OR controlstartedat < EXTRACT(EPOCH FROM now())::bigint-180) RETURNING *")
                .bind(id).bind(actor.value()).fetch_optional(&self.pool).await.map_err(storage)?
                .ok_or_else(|| BuildError::Conflict("A Build Pool operation is already running or the pool is archived.".into())).and_then(map_pool)
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn finish_pool_test_impl<'a>(
        &'a self,
        claim: &'a BuildAgentPool,
        result: &'a citadel_builds::BuildPoolCheck,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let message: String = result.message.chars().take(2048).collect();
            let row = sqlx::query("UPDATE buildagentpools SET controlstate='Idle',controltriggeredby=NULL,controlstartedat=NULL,lastvalidationstatus=$3,lastvalidationmessage=$4,lastvalidatedat=now(),updatedat=now(),rowversion=rowversion+1 WHERE id=$1 AND rowversion=$2 AND controlstate='Processing' RETURNING *")
                .bind(claim.id).bind(claim.row_version).bind(if result.ready { "Ready" } else { "Invalid" }).bind(&message)
                .fetch_optional(&mut *tx).await.map_err(storage)?
                .ok_or_else(|| BuildError::Conflict("The Build Pool check has been superseded.".into()))?;
            let pool = map_pool(row)?;
            let activity =
                citadel_domain::ActivityEvent::new_build_pool_event(
                    pool.id,
                    pool.name.clone(),
                    ActorId::new(claim.control_triggered_by.ok_or_else(|| {
                        BuildError::Storage("Build Pool claim has no actor.".into())
                    })?),
                    citadel_domain::ActivityEventInfo::BuildAgentPoolTested {
                        pool: pool.snapshot(),
                        status: pool.last_validation_status.clone(),
                        message,
                    },
                    Utc::now(),
                )
                .map_err(storage)?;
            crate::activity_store::insert_activity(&mut tx, &activity)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            self.get_pool(pool.id).await
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn update_pool_impl<'a>(
        &'a self,
        current: &'a BuildAgentPool,
        input: &'a BuildAgentPoolConfiguration,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<BuildAgentPool, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query("UPDATE buildagentpools SET name=$3,normalizedname=$4,description=$5,enabled=$6,provider=$7,providerspec=$8,maxactivebuilders=$9,queuetimeoutseconds=$10,provisioningtimeoutseconds=$11,registrationtimeoutseconds=$12,heartbeattimeoutseconds=$13,cleanuptimeoutseconds=$14,maximuminstancelifetimeseconds=$15,failureretentionminutes=$16,updatedat=now(),rowversion=rowversion+1 WHERE id=$1 AND rowversion=$2 AND archivedat IS NULL AND controlstate='Idle' RETURNING *")
                .bind(current.id).bind(current.row_version).bind(&input.name).bind(input.name.to_uppercase())
                .bind(&input.description).bind(input.enabled).bind(input.provider_spec["$type"].as_str()).bind(&input.provider_spec)
                .bind(input.max_active_builders).bind(input.queue_timeout_seconds).bind(input.provisioning_timeout_seconds)
                .bind(input.registration_timeout_seconds).bind(input.heartbeat_timeout_seconds).bind(input.cleanup_timeout_seconds)
                .bind(input.maximum_instance_lifetime_seconds).bind(input.failure_retention_minutes)
                .fetch_optional(&mut *tx).await.map_err(database)?
                .ok_or_else(|| BuildError::Conflict("The Build Pool was modified, archived or is processing. Refresh and try again.".into()))?;
            let pool = map_pool(row)?;
            let info = if current.name != pool.name {
                citadel_domain::ActivityEventInfo::BuildAgentPoolRenamed {
                    old_name: current.name.clone(),
                    new_name: pool.name.clone(),
                }
            } else {
                citadel_domain::ActivityEventInfo::BuildAgentPoolUpdated {
                    old_pool: current.snapshot(),
                    new_pool: pool.snapshot(),
                }
            };
            record_pool_activity(&mut tx, &pool, actor, info).await?;
            tx.commit().await.map_err(storage)?;
            self.get_pool(pool.id).await
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn archive_pool_impl<'a>(
        &'a self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let current = sqlx::query(
                "SELECT * FROM buildagentpools WHERE id=$1 AND archivedat IS NULL FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(BuildError::NotFound)
            .and_then(map_pool)?;
            if current.control_state != "Idle" {
                return Err(BuildError::Conflict("The Build Pool is processing.".into()));
            }
            sqlx::query("UPDATE buildagentpools SET enabled=false,archivedat=CURRENT_TIMESTAMP,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1")
                .bind(id).execute(&mut *tx).await.map_err(storage)?;
            record_pool_activity(
                &mut tx,
                &current,
                actor,
                citadel_domain::ActivityEventInfo::BuildAgentPoolDeleted {
                    pool: current.snapshot(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
