use super::*;
impl PostgresSwarmServiceRepository {
    pub(super) fn delete_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<ServiceDeletionClaim>, SwarmServiceError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let mut claims = Vec::with_capacity(ids.len());
            for id in ids {
                ensure_access(
                    &mut tx,
                    actor_id,
                    administrator,
                    *id,
                    policy::DeleteSwarmService::REQUIREMENT,
                )
                .await?;
                let row=sqlx::query("SELECT platformid,dockerserviceid,controlstate FROM swarmservices WHERE id=$1 FOR UPDATE")
                    .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(SwarmServiceError::NotFound)?;
                ensure_idle(&row)?;
                let operation_id = Uuid::now_v7();
                sqlx::query("UPDATE swarmservices s SET controlstate='Processing',controlstartedat=$2,controltriggeredby=$3,rowversion=rowversion+1,operationid=$4,operationkind='Delete',operationstate='Prepared',basedockerversion=dockerversionindex,targetdesiredspechash=desiredspechash,targetruntimehash=NULL,targetrowversion=rowversion+1,expectedforceupdate=NULL,preparedat=now(),attemptedat=NULL,completedat=NULL,observeddockerversion=NULL,resultcode=NULL,resultmessage=NULL,warnings=NULL,operationactorid=$3,operationclusterid=COALESCE(p.clusterid,p.platformdescriptor->>'clusterId','') FROM platforms p WHERE s.id=$1 AND p.id=s.platformid")
                    .bind(id).bind(Utc::now().timestamp()).bind(actor_id.value()).bind(operation_id).execute(&mut *tx).await.map_err(storage)?;
                claims.push(ServiceDeletionClaim {
                    operation_id,
                    id: *id,
                    platform_id: row.try_get("platformid").map_err(storage)?,
                    docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
                });
            }
            tx.commit().await.map_err(storage)?;
            Ok(claims)
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn mark_delete_attempted_impl<'a>(
        &'a self,
        claim: &'a ServiceDeletionClaim,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            let changed=sqlx::query("UPDATE swarmservices SET operationstate='PendingAcceptance',attemptedat=now(),rowversion=rowversion+1 WHERE id=$1 AND operationid=$2 AND operationkind='Delete' AND operationstate='Prepared' AND controlstate='Processing'")
                .bind(claim.id).bind(claim.operation_id).execute(&self.pool).await.map_err(storage)?.rows_affected();
            if changed != 1 {
                return Err(SwarmServiceError::Conflict(
                    "The deletion claim changed before dispatch.".into(),
                ));
            }
            Ok(())
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn complete_delete_impl<'a>(
        &'a self,
        actor_id: ActorId,
        deleted: &'a [ServiceDeletionClaim],
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            for claim in deleted {
                let row = sqlx::query("SELECT * FROM swarmservices WHERE id=$1 AND operationid=$2 AND operationkind='Delete' AND controlstate='Processing' FOR UPDATE")
                    .bind(claim.id).bind(claim.operation_id).fetch_optional(&mut *tx).await.map_err(storage)?;
                let Some(row) = row else {
                    let exists: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM swarmservices WHERE id=$1)",
                    )
                    .bind(claim.id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(storage)?;
                    if exists {
                        return Err(SwarmServiceError::Conflict(
                            "The deletion operation changed before completion.".into(),
                        ));
                    }
                    continue;
                };
                let snapshot = activity_snapshot(&row)?;
                let activity_name = snapshot.name.clone();
                insert_swarm_activity(
                    &mut tx,
                    claim.id,
                    &activity_name,
                    snapshot.platform_id,
                    actor_id,
                    ActivityEventInfo::swarm_service_deleted(snapshot),
                    ActivityStatus::Success,
                )
                .await?;
                sqlx::query("DELETE FROM swarmservices WHERE id=$1 AND controlstate='Processing'")
                    .bind(claim.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn release_delete_impl<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            sqlx::query("UPDATE swarmservices SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1,operationstate=CASE WHEN operationkind='Delete' THEN 'Rejected' ELSE operationstate END,completedat=CASE WHEN operationkind='Delete' THEN now() ELSE completedat END WHERE id=ANY($1::uuid[]) AND controlstate='Processing' AND updatecheckid IS NULL").bind(ids).execute(&self.pool).await.map_err(storage)?;
            Ok(())
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn stale_deletion_claims_impl(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceDeletionClaim)>, SwarmServiceError>> {
        Box::pin(async move {
            sqlx::query("SELECT id,operationid,platformid,dockerserviceid,controltriggeredby FROM swarmservices WHERE controlstate='Processing' AND updatecheckid IS NULL AND controlstartedat <= $1 AND (operationstate IS NULL OR operationstate NOT IN ('Prepared','PendingAcceptance','Accepted')) AND controltriggeredby IS NOT NULL ORDER BY controlstartedat,id LIMIT $2")
                .bind(started_before).bind(limit).fetch_all(&self.pool).await.map_err(storage)?
                .into_iter().map(|row| Ok((
                    ActorId::new(row.try_get("controltriggeredby").map_err(storage)?),
                    ServiceDeletionClaim {
                        operation_id: row.try_get::<Option<Uuid>,_>("operationid").map_err(storage)?.unwrap_or_default(),
                        id: row.try_get("id").map_err(storage)?,
                        platform_id: row.try_get("platformid").map_err(storage)?,
                        docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
                    },
                ))).collect()
        })
    }
}
