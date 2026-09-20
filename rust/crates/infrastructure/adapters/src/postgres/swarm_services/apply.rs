use super::*;
impl PostgresSwarmServiceRepository {
    pub(super) fn claim_operation_impl(
        &self,
        actor_id: ActorId,
        administrator: bool,
        request: citadel_swarm_services::ServiceOperationRequest,
    ) -> BoxFuture<'_, Result<ServiceOperationClaim, SwarmServiceError>> {
        Box::pin(async move {
            let citadel_swarm_services::ServiceOperationRequest {
                id,
                kind,
                replicas,
                expected_version,
            } = request;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let requirement = match kind {
                ServiceOperationKind::Scale => policy::ScaleSwarmService::REQUIREMENT,
                ServiceOperationKind::ForceUpdate => policy::ForceUpdateSwarmService::REQUIREMENT,
                ServiceOperationKind::Apply => policy::ApplySwarmService::REQUIREMENT,
            };
            ensure_access(&mut tx, actor_id, administrator, id, requirement).await?;
            let row=sqlx::query("SELECT s.*,p.status platform_status,p.platformdescriptor FROM swarmservices s JOIN platforms p ON p.id=s.platformid WHERE s.id=$1 FOR UPDATE OF s")
                .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(SwarmServiceError::NotFound)?;
            ensure_idle(&row)?;
            if expected_version.is_some_and(|version| row.get::<i64, _>("rowversion") != version) {
                return Err(SwarmServiceError::Conflict(
                    "The Service configuration changed before automatic Apply.".into(),
                ));
            }
            ensure_swarm_manager(&row)?;
            let mut spec =
                SwarmServiceSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
            if kind == ServiceOperationKind::Scale {
                if spec.scheduling_mode != citadel_swarm_services::SchedulingMode::Replicated {
                    return Err(SwarmServiceError::Validation(
                        "Global Services cannot be scaled by replica count.".to_owned(),
                    ));
                }
                spec.replicas = replicas;
            }
            let operation_id = Uuid::now_v7();
            let desired = spec.desired_hash();
            let now = Utc::now();
            let new_version = row.try_get::<i64, _>("rowversion").map_err(storage)? + 1;
            let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
            let cluster_id = descriptor
                .get("clusterId")
                .or_else(|| descriptor.get("ClusterId"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if cluster_id.trim().is_empty() {
                return Err(SwarmServiceError::Conflict(
                    "The Docker Swarm cluster identity is unavailable.".to_owned(),
                ));
            }
            sqlx::query("UPDATE swarmservices SET spec=$2,desiredspechash=$3,operationid=$4,operationkind=$5,operationstate='Prepared',targetdesiredspechash=$3,targetruntimehash=NULL,expectedforceupdate=NULL,attemptedat=NULL,completedat=NULL,observeddockerversion=NULL,resultcode=NULL,resultmessage=NULL,warnings=NULL,targetrowversion=$6,preparedat=$7,operationactorid=$8,operationclusterid=$10,basedockerversion=dockerversionindex,controlstate='Processing',controlstartedat=$9,controltriggeredby=$8,rowversion=rowversion+1,updatedat=$7 WHERE id=$1")
                .bind(id).bind(spec.to_storage_value()?).bind(&desired).bind(operation_id).bind(kind.as_str()).bind(new_version).bind(now).bind(actor_id.value()).bind(now.timestamp()).bind(cluster_id)
                .execute(&mut *tx).await.map_err(database_error)?;
            let claim = ServiceOperationClaim {
                operation_id,
                id,
                platform_id: row.try_get("platformid").map_err(storage)?,
                docker_name: row.try_get("dockername").map_err(storage)?,
                docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
                docker_version_index: row.try_get("dockerversionindex").map_err(storage)?,
                row_version: new_version,
                desired_hash: desired,
                spec,
            };
            tx.commit().await.map_err(storage)?;
            Ok(claim)
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn mark_attempted_impl<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            let changed=sqlx::query("UPDATE swarmservices SET operationstate='PendingAcceptance',attemptedat=$3,rowversion=rowversion+1 WHERE id=$1 AND operationid=$2 AND operationstate='Prepared'")
            .bind(claim.id).bind(claim.operation_id).bind(Utc::now()).execute(&self.pool).await.map_err(storage)?.rows_affected();
            if changed != 1 {
                return Err(SwarmServiceError::Conflict(
                    "The Service operation claim changed before dispatch.".to_owned(),
                ));
            }
            Ok(())
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn mark_accepted_impl<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        result: &'a RuntimeServiceResult,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            let changed = sqlx::query(
                "UPDATE swarmservices SET dockerserviceid=$3,dockerversionindex=$4,operationstate='Accepted',observeddockerversion=$4,warnings=$5,rowversion=rowversion+1,updatedat=$6 WHERE id=$1 AND operationid=$2 AND operationstate='PendingAcceptance'",
            )
            .bind(claim.id)
            .bind(claim.operation_id)
            .bind(&result.docker_service_id)
            .bind(result.version_index)
            .bind(serde_json::to_value(&result.warnings).map_err(storage)?)
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .map_err(storage)?
            .rows_affected();
            if changed != 1 {
                let settled: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmservices WHERE id=$1 AND operationid=$2 AND operationstate IN ('Accepted','Completed'))")
                    .bind(claim.id).bind(claim.operation_id).fetch_one(&self.pool).await.map_err(storage)?;
                if settled {
                    return Ok(());
                }
                return Err(SwarmServiceError::Conflict(
                    "The Service operation changed before Docker acceptance was recorded."
                        .to_owned(),
                ));
            }
            Ok(())
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn complete_operation_impl<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ServiceOperationClaim,
        result: &'a RuntimeServiceResult,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query("SELECT name,platformid,operationkind,operationstate,spec FROM swarmservices WHERE id=$1 AND operationid=$2 FOR UPDATE")
                .bind(claim.id).bind(claim.operation_id).fetch_optional(&mut *tx).await.map_err(storage)?
                .ok_or(SwarmServiceError::NotFound)?;
            if row
                .try_get::<String, _>("operationstate")
                .map_err(storage)?
                == "Completed"
            {
                tx.commit().await.map_err(storage)?;
                return Ok(());
            }
            let changed=sqlx::query("UPDATE swarmservices SET dockerserviceid=$3,dockerversionindex=$4,operationstate='Completed',observeddockerversion=$4,completedat=$5,warnings=$6,lastapplieddesiredspechash=targetdesiredspechash,lastappliedruntimehash=$7,appliedimagedigest=COALESCE($8,appliedimagedigest),health='Healthy',synchronizationstate='InSync',controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1,updatedat=$5 WHERE id=$1 AND operationid=$2 AND operationstate IN ('PendingAcceptance','Accepted')")
            .bind(claim.id).bind(claim.operation_id).bind(&result.docker_service_id).bind(result.version_index).bind(Utc::now()).bind(serde_json::to_value(&result.warnings).map_err(storage)?).bind(&result.runtime_hash).bind(&result.applied_digest)
            .execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed != 1 {
                return Err(SwarmServiceError::Conflict(
                    "The Service operation changed before completion.".to_owned(),
                ));
            }
            let operation_kind: String = row.try_get("operationkind").map_err(storage)?;
            let operation_spec =
                SwarmServiceSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
            insert_swarm_activity(
                &mut tx,
                claim.id,
                &row.try_get::<String, _>("name").map_err(storage)?,
                row.try_get("platformid").map_err(storage)?,
                actor_id,
                ActivityEventInfo::swarm_service_completed(
                    &operation_kind,
                    claim.operation_id,
                    operation_spec.replicas,
                    result.warnings.clone(),
                ),
                ActivityStatus::Success,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn fail_operation_impl<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ServiceOperationClaim,
        message: &'a str,
        outcome_unknown: bool,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            let state = if outcome_unknown {
                "OutcomeUnknown"
            } else {
                "Rejected"
            };
            let sync = if outcome_unknown {
                "OutcomeUnknown"
            } else {
                "DesiredChangesPending"
            };
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query("SELECT name,platformid,operationkind FROM swarmservices WHERE id=$1 AND operationid=$2 FOR UPDATE")
                .bind(claim.id).bind(claim.operation_id).fetch_optional(&mut *tx).await.map_err(storage)?
                .ok_or(SwarmServiceError::NotFound)?;
            let changed=sqlx::query("UPDATE swarmservices SET operationstate=$3,completedat=$4,resultcode=$5,resultmessage=$6,health=CASE WHEN $7 THEN health ELSE 'Failed' END,synchronizationstate=$8,controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1,updatedat=$4 WHERE id=$1 AND operationid=$2 AND operationstate IN ('Prepared','PendingAcceptance','Accepted')")
                .bind(claim.id).bind(claim.operation_id).bind(state).bind(Utc::now()).bind(if outcome_unknown{"OutcomeUnknown"}else{"RolloutFailed"}).bind(message).bind(outcome_unknown).bind(sync)
                .execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed != 1 {
                return Err(SwarmServiceError::Conflict(
                    "The Service operation changed before failure was recorded.".to_owned(),
                ));
            }
            if !outcome_unknown {
                insert_swarm_activity(
                    &mut tx,
                    claim.id,
                    &row.try_get::<String, _>("name").map_err(storage)?,
                    row.try_get("platformid").map_err(storage)?,
                    actor_id,
                    ActivityEventInfo::swarm_service_operation_failed(
                        claim.operation_id,
                        row.try_get("operationkind").map_err(storage)?,
                        message.to_owned(),
                    ),
                    ActivityStatus::Failure,
                )
                .await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn stale_operation_claims_impl(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceOperationClaim)>, SwarmServiceError>> {
        Box::pin(async move {
            sqlx::query("SELECT id,platformid,dockername,dockerserviceid,dockerversionindex,rowversion,desiredspechash,spec,operationid,operationactorid FROM swarmservices WHERE controlstate='Processing' AND updatecheckid IS NULL AND controlstartedat <= $1 AND operationid IS NOT NULL AND operationkind<>'Delete' ORDER BY controlstartedat,id LIMIT $2")
            .bind(started_before).bind(limit).fetch_all(&self.pool).await.map_err(storage)?.into_iter().map(|row| Ok((ActorId::new(row.try_get("operationactorid").map_err(storage)?),ServiceOperationClaim{operation_id:row.try_get("operationid").map_err(storage)?,id:row.try_get("id").map_err(storage)?,platform_id:row.try_get("platformid").map_err(storage)?,docker_name:row.try_get("dockername").map_err(storage)?,docker_service_id:row.try_get("dockerserviceid").map_err(storage)?,docker_version_index:row.try_get("dockerversionindex").map_err(storage)?,row_version:row.try_get("rowversion").map_err(storage)?,desired_hash:row.try_get("desiredspechash").map_err(storage)?,spec:SwarmServiceSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?}))).collect()
        })
    }
}
