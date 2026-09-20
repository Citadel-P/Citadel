use super::*;
impl PostgresStackRepository {
    pub(super) fn claim_state_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StackStateClaim>, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let mut claims = Vec::with_capacity(ids.len());
            for id in ids {
                ensure_access(
                    &mut tx,
                    actor,
                    administrator,
                    *id,
                    policy::ChangeStackState::REQUIREMENT,
                )
                .await?;
                let row = sqlx::query("SELECT s.name,s.controlstate,s.currentstackreleaseid,r.platformid,r.status,r.spec,p.status platform_status,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1 FOR UPDATE OF s,r")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(storage)?
                    .ok_or(StackError::NotFound)?;
                ensure_idle(&row)?;
                if row
                    .try_get::<String, _>("platform_status")
                    .map_err(storage)?
                    != "Online"
                {
                    return Err(StackError::Conflict(
                        "The Stack Platform is offline.".to_owned(),
                    ));
                }
                let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
                let platform_type =
                    crate::persistence::postgres::platforms::classification::platform_kind(
                        descriptor
                            .get("$type")
                            .or_else(|| descriptor.get("type"))
                            .and_then(Value::as_str)
                            .unwrap_or("Docker"),
                    )
                    .map_err(storage)?;
                if platform_type == citadel_platforms::PlatformKind::DockerSwarm {
                    return Err(StackError::Validation(
                        "Container state actions are not available for Docker Swarm stacks."
                            .to_owned(),
                    ));
                }
                let name: String = row.try_get("name").map_err(storage)?;
                let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                claims.push(StackStateClaim {
                    stack_id: *id,
                    release_id: row.try_get("currentstackreleaseid").map_err(storage)?,
                    platform_id: row.try_get("platformid").map_err(storage)?,
                    name: name.clone(),
                    project_name: spec
                        .common()
                        .project_name
                        .clone()
                        .unwrap_or_else(|| normalize_project_name(&name, *id)),
                    platform_type,
                    previous_status: StackReleaseStatus::parse(
                        &row.try_get::<String, _>("status").map_err(storage)?,
                    )?,
                    actor_id: actor.value(),
                });
            }
            let now = Utc::now().timestamp();
            for claim in &claims {
                sqlx::query("UPDATE stackreleases SET status='Pending' WHERE id=$1 AND stackid=$2")
                    .bind(claim.release_id)
                    .bind(claim.stack_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
                sqlx::query("UPDATE stacks SET controlstate='Processing',controlstartedat=$2,controltriggeredby=$3,rowversion=rowversion+1 WHERE id=$1")
                    .bind(claim.stack_id)
                    .bind(now)
                    .bind(actor.value())
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(claims)
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn complete_state_impl<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackStateClaim,
        status: StackReleaseStatus,
        container_ids: &'a [String],
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            let info = match status {
                StackReleaseStatus::Healthy => ActivityEventInfo::StackStarted {
                    container_ids: container_ids.to_vec(),
                },
                StackReleaseStatus::Stopped => ActivityEventInfo::StackStopped {
                    container_ids: container_ids.to_vec(),
                },
                StackReleaseStatus::Paused => ActivityEventInfo::StackPaused {
                    container_ids: container_ids.to_vec(),
                },
                _ => {
                    return Err(StackError::Validation(
                        "A Stack state operation did not reach a stable state.".to_owned(),
                    ));
                }
            };
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let locked = sqlx::query_scalar::<_, String>("SELECT name FROM stacks WHERE id=$1 AND currentstackreleaseid=$2 AND controlstate='Processing' AND controltriggeredby=$3 FOR UPDATE")
                .bind(claim.stack_id)
                .bind(claim.release_id)
                .bind(claim.actor_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or_else(|| StackError::Conflict("The Stack state operation was superseded.".to_owned()))?;
            let release_changed = sqlx::query("UPDATE stackreleases SET status=$3 WHERE id=$1 AND stackid=$2 AND status='Pending'")
                .bind(claim.release_id)
                .bind(claim.stack_id)
                .bind(status.as_str())
                .execute(&mut *tx)
                .await
                .map_err(storage)?
                .rows_affected();
            let stack_changed = sqlx::query("UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentstackreleaseid=$2 AND controlstate='Processing' AND controltriggeredby=$3")
                .bind(claim.stack_id)
                .bind(claim.release_id)
                .bind(claim.actor_id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?
                .rows_affected();
            if release_changed != 1 || stack_changed != 1 {
                return Err(StackError::Conflict(
                    "The Stack state operation was superseded.".to_owned(),
                ));
            }
            insert_stack_activity(
                &mut tx,
                claim.stack_id,
                &locked,
                claim.platform_id,
                actor,
                info,
                ActivityStatus::Success,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn release_state_impl<'a>(
        &'a self,
        claims: &'a [StackStateClaim],
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            for claim in claims {
                let locked = sqlx::query_scalar::<_, Uuid>("SELECT currentstackreleaseid FROM stacks WHERE id=$1 AND currentstackreleaseid=$2 AND controlstate='Processing' AND controltriggeredby=$3 FOR UPDATE")
                    .bind(claim.stack_id)
                    .bind(claim.release_id)
                    .bind(claim.actor_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(storage)?;
                if locked.is_none() {
                    continue;
                }
                sqlx::query("UPDATE stackreleases SET status=$3 WHERE id=$1 AND stackid=$2 AND status='Pending'")
                    .bind(claim.release_id)
                    .bind(claim.stack_id)
                    .bind(claim.previous_status.as_str())
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
                sqlx::query("UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentstackreleaseid=$2 AND controlstate='Processing' AND controltriggeredby=$3")
                    .bind(claim.stack_id)
                    .bind(claim.release_id)
                    .bind(claim.actor_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn stale_state_claims_impl(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackStateClaim)>, StackError>> {
        Box::pin(async move {
            sqlx::query("SELECT s.id,s.name,s.controltriggeredby,s.currentstackreleaseid,r.platformid,r.spec,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.controlstate='Processing' AND s.containeroperationid IS NULL AND s.controlstartedat <= $1 AND r.status='Pending' AND s.controltriggeredby IS NOT NULL ORDER BY s.controlstartedat,s.id LIMIT $2")
                .bind(started_before)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(|row| {
                    let id: Uuid = row.try_get("id").map_err(storage)?;
                    let actor = ActorId::new(row.try_get("controltriggeredby").map_err(storage)?);
                    let name: String = row.try_get("name").map_err(storage)?;
                    let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
                    Ok((actor, StackStateClaim {
                        stack_id: id,
                        release_id: row.try_get("currentstackreleaseid").map_err(storage)?,
                        platform_id: row.try_get("platformid").map_err(storage)?,
                        project_name: spec.common().project_name.clone().unwrap_or_else(|| normalize_project_name(&name, id)),
                        platform_type: crate::persistence::postgres::platforms::classification::platform_kind(descriptor.get("$type").or_else(|| descriptor.get("type")).and_then(Value::as_str).unwrap_or("Docker")).map_err(storage)?,
                        previous_status: StackReleaseStatus::Unknown,
                        actor_id: actor.value(),
                        name,
                    }))
                })
                .collect()
        })
    }
}
