use super::*;
impl PostgresStackRepository {
    pub(super) fn stale_delete_claims_impl(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackDeletionClaim)>, StackError>> {
        Box::pin(async move {
            sqlx::query("SELECT s.id,s.name,s.controltriggeredby,r.platformid,r.spec,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.controlstate='Processing' AND s.containeroperationid IS NULL AND s.controlstartedat <= $1 AND r.status NOT IN ('Applying','Pending') AND s.controltriggeredby IS NOT NULL ORDER BY s.controlstartedat,s.id LIMIT $2")
                .bind(started_before).bind(limit).fetch_all(&self.pool).await.map_err(storage)?.into_iter().map(|row| {
                    let id:Uuid=row.try_get("id").map_err(storage)?;
                    let spec=StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                    let descriptor:Value=row.try_get("platformdescriptor").map_err(storage)?;
                    let name:String=row.try_get("name").map_err(storage)?;
                    Ok((ActorId::new(row.try_get("controltriggeredby").map_err(storage)?), StackDeletionClaim {
                        stack_id:id,
                        platform_id:row.try_get("platformid").map_err(storage)?,
                        project_name:spec.common().project_name.clone().unwrap_or_else(|| normalize_project_name(&name,id)),
                        platform_type:descriptor.get("$type").and_then(Value::as_str).unwrap_or("Docker").to_owned(),
                    }))
                }).collect()
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn claim_delete_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StackDeletionClaim>, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let mut claims = Vec::with_capacity(ids.len());
            for id in ids {
                ensure_access(
                    &mut tx,
                    actor,
                    administrator,
                    *id,
                    policy::DeleteStack::REQUIREMENT,
                )
                .await?;
                let row=sqlx::query("SELECT s.name,s.controlstate,r.platformid,r.spec,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1 FOR UPDATE OF s")
                    .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StackError::NotFound)?;
                ensure_idle(&row)?;
                let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
                let name: String = row.try_get("name").map_err(storage)?;
                claims.push(StackDeletionClaim {
                    stack_id: *id,
                    platform_id: row.try_get("platformid").map_err(storage)?,
                    project_name: spec
                        .common()
                        .project_name
                        .clone()
                        .unwrap_or_else(|| normalize_project_name(&name, *id)),
                    platform_type: descriptor
                        .get("$type")
                        .and_then(Value::as_str)
                        .unwrap_or("Docker")
                        .to_owned(),
                });
            }
            for id in ids {
                sqlx::query("UPDATE stacks SET controlstate='Processing',controlstartedat=$2,controltriggeredby=$3,rowversion=rowversion+1 WHERE id=$1").bind(id).bind(Utc::now().timestamp()).bind(actor.value()).execute(&mut *tx).await.map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(claims)
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn complete_delete_impl<'a>(
        &'a self,
        actor: ActorId,
        claims: &'a [StackDeletionClaim],
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            for claim in claims {
                let row=sqlx::query("SELECT s.name,s.description,s.stacksource,s.driftpolicy,r.spec,r.version FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1 AND s.controlstate='Processing' FOR UPDATE OF s")
                    .bind(claim.stack_id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StackError::NotFound)?;
                let name: String = row.try_get("name").map_err(storage)?;
                let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                let snapshot = stack_snapshot(
                    claim.stack_id,
                    &name,
                    row.try_get("description").map_err(storage)?,
                    parse_stack_source(row.try_get("stacksource").map_err(storage)?)?,
                    StackDriftPolicy::from_storage_value(
                        row.try_get("driftpolicy").map_err(storage)?,
                    )?,
                    claim.platform_id,
                    &spec,
                    actor,
                    row.try_get("version").map_err(storage)?,
                );
                insert_stack_activity(
                    &mut tx,
                    claim.stack_id,
                    &name,
                    claim.platform_id,
                    actor,
                    ActivityEventInfo::StackDeleted { stack: snapshot },
                    ActivityStatus::Success,
                )
                .await?;
                sqlx::query("DELETE FROM stacks WHERE id=$1 AND controlstate='Processing'")
                    .bind(claim.stack_id)
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
    pub(super) fn release_delete_impl<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            sqlx::query("UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE id=ANY($1::uuid[]) AND controlstate='Processing'").bind(ids).execute(&self.pool).await.map_err(storage)?;
            Ok(())
        })
    }
}
