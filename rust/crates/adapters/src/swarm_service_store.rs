use chrono::{DateTime, Utc};
use citadel_domain::{
    ActivityEvent, ActivityEventInfo, ActivityStatus, ActorId, ResourceType,
    SwarmServiceActivitySnapshot,
};
use citadel_swarm_services::{
    AutoUpdateState, CreateSwarmServiceInput, ManagedSwarmServiceView, RenameSwarmServiceInput,
    RuntimeServiceResult, ServiceDeletionClaim, ServiceOperationClaim, ServiceOperationKind,
    SwarmServiceCapabilities, SwarmServiceError, SwarmServiceFilter, SwarmServiceOperationView,
    SwarmServiceSpec, SwarmServiceStore, UpdateSwarmServiceInput,
};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::insert_activity;

mod updates;

const RESOURCE_TYPE: i32 = ResourceType::SwarmService as i32;
const PLATFORM_RESOURCE_TYPE: i32 = ResourceType::Platform as i32;
const READ: i32 = 1;
const WRITE: i32 = 2;
const EXECUTE: i32 = 4;
const APPLY: i32 = 1 << 2;

const AUTHORIZED_CTES: &str = r#"
WITH actor_scope AS (
    SELECT id actorid FROM actors WHERE id=$1 AND isenabled
    UNION
    SELECT team.actorid FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors actor ON actor.id=team.actorid AND actor.isenabled
    WHERE membership.memberactorid=$1
), global_permission AS (
    SELECT COALESCE(MAX(permission.permissionlevel),0)::integer level_mask,
           COALESCE(bit_or(permission.specificpermissions),0)::integer specific_mask
    FROM actor_scope scope JOIN actorroles assignment ON assignment.actorid=scope.actorid
    JOIN permissions permission ON permission.roleid=assignment.roleid
    WHERE permission.resourcetype=20
), resource_permissions AS (
    SELECT access.resourceid, COALESCE(MAX(access.permissionlevel),0)::integer level_mask,
           COALESCE(bit_or(access.specificpermissions),0)::integer specific_mask
    FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=20 GROUP BY access.resourceid
)
"#;

const PROJECTION: &str = r#"
SELECT s.*, p.name platform_name, p.status platform_status,
       projection.runningtaskcount, projection.desiredtaskcount,
       projection.updatestate projection_update_state, projection.updatemessage,
       projection.liveruntimehash, projection.isstale,
       COALESCE(tags.value,'[]'::jsonb) tags,
       COALESCE(tasks.value,'[]'::jsonb) tasks,
       CASE WHEN s.autoupdatestate_lastcheckedat='-infinity'::timestamptz
            THEN TIMESTAMPTZ '0001-01-01 00:00:00+00'
            ELSE s.autoupdatestate_lastcheckedat
       END effective_last_checked_at,
       CASE
         WHEN s.dockerserviceid IS NULL THEN s.health
         WHEN projection.dockerserviceid IS NULL OR projection.isstale THEN 'Unknown'
         WHEN lower(COALESCE(projection.updatestate,'')) IN ('paused','rollback_paused','rollback_completed') THEN 'Failed'
         WHEN projection.desiredtaskcount=0 THEN 'Stopped'
         WHEN projection.runningtaskcount>=projection.desiredtaskcount THEN 'Healthy'
         WHEN projection.runningtaskcount>0 THEN 'Degraded'
         ELSE 'Progressing'
       END effective_health,
       CASE
         WHEN s.dockerserviceid IS NULL THEN s.synchronizationstate
         WHEN projection.dockerserviceid IS NULL OR projection.isstale THEN 'RuntimeMissing'
         WHEN projection.ownership='OwnershipConflict' THEN 'OwnershipConflict'
         WHEN s.lastappliedruntimehash IS DISTINCT FROM projection.liveruntimehash THEN 'Drifted'
         WHEN s.lastapplieddesiredspechash IS DISTINCT FROM s.desiredspechash THEN 'DesiredChangesPending'
         ELSE 'InSync'
       END effective_synchronization_state,
       CASE WHEN $2 THEN 7 ELSE GREATEST(COALESCE(g.level_mask,0),COALESCE(r.level_mask,0)) END permission_level,
       CASE WHEN $2 THEN 63 ELSE (COALESCE(g.specific_mask,0)|COALESCE(r.specific_mask,0)) END permission_specific
FROM swarmservices s
JOIN platforms p ON p.id=s.platformid
LEFT JOIN global_permission g ON TRUE
LEFT JOIN resource_permissions r ON r.resourceid=s.id
LEFT JOIN swarmserviceprojections projection
  ON projection.platformid=s.platformid AND projection.dockerserviceid=s.dockerserviceid
LEFT JOIN LATERAL (
    SELECT jsonb_agg(jsonb_build_object('id',tag.id,'name',tag.name,'color',tag.color) ORDER BY tag.name,tag.id) value
    FROM resourcetags link JOIN tags tag ON tag.id=link.tagid
    WHERE link.resourcetype='SwarmService' AND link.resourceid=s.id
) tags ON TRUE
LEFT JOIN LATERAL (
    SELECT jsonb_agg(jsonb_build_object(
        'id', task.dockertaskid,
        'name', task.name,
        'serviceId', task.dockerserviceid,
        'serviceName', task.servicename,
        'nodeId', task.dockernodeid,
        'nodeHostname', task.nodehostname,
        'containerId', task.dockercontainerid,
        'image', task.image,
        'desiredState', task.desiredstate,
        'state', task.state,
        'statusMessage', task.statusmessage,
        'error', task.error,
        'slot', task.slot,
        'versionIndex', task.versionindex,
        'createdAt', task.dockercreatedat,
        'updatedAt', task.dockerupdatedat,
        'statusTimestamp', task.statustimestamp,
        'ports', task.ports
    ) ORDER BY task.slot NULLS LAST, task.name, task.dockertaskid) value
    FROM swarmtaskprojections task
    WHERE task.platformid=s.platformid
      AND task.dockerserviceid=s.dockerserviceid
      AND NOT task.isstale
) tasks ON TRUE
"#;

#[derive(Clone)]
pub struct PostgresSwarmServiceStore {
    pool: PgPool,
}

impl PostgresSwarmServiceStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl SwarmServiceStore for PostgresSwarmServiceStore {
    fn update_check_candidates(
        &self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, SwarmServiceError>> {
        Box::pin(async move {
            sqlx::query_scalar("SELECT s.id FROM swarmservices s JOIN platforms p ON p.id=s.platformid WHERE ($1::uuid IS NULL OR s.id>$1) AND s.controlstate='Idle' AND s.spec->>'UpdateBehavior' IN ('Notify','AutoDeploy') AND s.spec->'Image'->>'$type'='External' AND s.spec->'Image'->>'ImageTag' NOT LIKE '%@%' AND COALESCE(s.appliedimagedigest,'')<>'' AND p.status='Online' ORDER BY s.id LIMIT $2")
                .bind(after).bind(i64::try_from(limit.clamp(1,100)).unwrap_or(100)).fetch_all(&self.pool).await.map_err(storage)
        })
    }
    fn begin_update_check<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a ManagedSwarmServiceView,
    ) -> BoxFuture<'a, Result<citadel_swarm_services::ServiceUpdateCheck, SwarmServiceError>> {
        Box::pin(self.claim_image_check(actor, administrator, expected))
    }
    fn complete_update_check<'a>(
        &'a self,
        claim: &'a citadel_swarm_services::ServiceUpdateCheck,
        state: Option<&'a AutoUpdateState>,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(self.finish_image_check(claim, state))
    }
    fn recover_update_checks(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, SwarmServiceError>> {
        Box::pin(self.recover_image_checks(started_before, limit))
    }
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        filter: &'a SwarmServiceFilter,
    ) -> BoxFuture<'a, Result<Vec<ManagedSwarmServiceView>, SwarmServiceError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTES}{PROJECTION}
WHERE ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(r.level_mask,0)) >= 1)
  AND ($3::uuid IS NULL OR s.platformid=$3)
  AND NOT EXISTS (SELECT 1 FROM unnest($4::text[]) requested(name) WHERE NOT EXISTS (
      SELECT 1 FROM resourcetags filter_link JOIN tags filter_tag ON filter_tag.id=filter_link.tagid
      WHERE filter_link.resourcetype='SwarmService' AND filter_link.resourceid=s.id
        AND (lower(filter_tag.name)=lower(requested.name) OR filter_tag.id::text=requested.name)))
ORDER BY s.createdat DESC,s.name,s.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(administrator)
                .bind(filter.platform_id)
                .bind(&filter.tags)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_service)
                .collect()
        })
    }

    fn get_authorized(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<ManagedSwarmServiceView, SwarmServiceError>> {
        Box::pin(async move { get_authorized(&self.pool, actor_id, administrator, id).await })
    }

    fn create<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a CreateSwarmServiceInput,
    ) -> BoxFuture<'a, Result<ManagedSwarmServiceView, SwarmServiceError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_platform(&mut tx, actor_id, administrator, input.platform_id, false).await?;
            validate_references(&mut tx, input).await?;
            validate_tags(&mut tx, &input.tag_ids).await?;
            let id = Uuid::now_v7();
            let now = Utc::now();
            let docker_name = docker_name(&input.name, id);
            let spec = input.spec.to_storage_value()?;
            let desired = input.spec.desired_hash();
            sqlx::query(r#"INSERT INTO swarmservices(
                id,platformid,name,description,dockername,spec,desiredspechash,health,synchronizationstate,
                controlstate,rowversion,createdbyactorid,createdat,updatedat,
                autoupdatestate_lastcheckedat,autoupdatestate_status)
                VALUES($1,$2,$3,$4,$5,$6,$7,'Created','NeverApplied','Idle',0,$8,$9,$9,'-infinity','Unknown')"#)
                .bind(id).bind(input.platform_id).bind(&input.name).bind(&input.description).bind(&docker_name)
                .bind(spec).bind(desired).bind(actor_id.value()).bind(now).execute(&mut *tx).await.map_err(database_error)?;
            replace_tags(&mut tx, id, actor_id, &input.tag_ids).await?;
            insert_swarm_activity(
                &mut tx,
                id,
                &input.name,
                input.platform_id,
                actor_id,
                ActivityEventInfo::swarm_service_created(SwarmServiceActivitySnapshot {
                    id,
                    platform_id: input.platform_id,
                    name: input.name.clone(),
                    description: input.description.clone(),
                    docker_name: docker_name.clone(),
                    docker_service_id: None,
                    spec: input.spec.to_storage_value()?,
                }),
                ActivityStatus::Information,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor_id, administrator, id).await
        })
    }

    fn update<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: &'a UpdateSwarmServiceInput,
    ) -> BoxFuture<'a, Result<ManagedSwarmServiceView, SwarmServiceError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(&mut tx, actor_id, administrator, id, WRITE, 0).await?;
            let old_row = sqlx::query("SELECT * FROM swarmservices WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(SwarmServiceError::NotFound)?;
            let platform_id = old_row.try_get("platformid").map_err(storage)?;
            let old_snapshot = activity_snapshot(&old_row)?;
            let synthetic = CreateSwarmServiceInput {
                name: String::new(),
                platform_id,
                description: None,
                spec: input.spec.clone(),
                tag_ids: Vec::new(),
                duplicate_source: None,
            };
            validate_references(&mut tx, &synthetic).await?;
            let spec = input.spec.to_storage_value()?;
            let desired = input.spec.desired_hash();
            let changed=sqlx::query("UPDATE swarmservices SET spec=$3,desiredspechash=$4,synchronizationstate=CASE WHEN lastapplieddesiredspechash=$4 THEN 'InSync' ELSE 'DesiredChangesPending' END,rowversion=rowversion+1,updatedat=$5 WHERE id=$1 AND rowversion=$2 AND controlstate='Idle'")
                .bind(id).bind(input.row_version).bind(spec).bind(desired).bind(Utc::now()).execute(&mut *tx).await.map_err(database_error)?.rows_affected();
            if changed != 1 {
                return Err(SwarmServiceError::Conflict(
                    "The Service was changed by another operation.".to_owned(),
                ));
            }
            let new_snapshot = SwarmServiceActivitySnapshot {
                spec: input.spec.to_storage_value()?,
                ..old_snapshot.clone()
            };
            let activity_name = old_snapshot.name.clone();
            insert_swarm_activity(
                &mut tx,
                id,
                &activity_name,
                platform_id,
                actor_id,
                ActivityEventInfo::swarm_service_updated(old_snapshot, new_snapshot),
                ActivityStatus::Success,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor_id, administrator, id).await
        })
    }

    fn rename<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a RenameSwarmServiceInput,
    ) -> BoxFuture<'a, Result<ManagedSwarmServiceView, SwarmServiceError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(&mut tx, actor_id, administrator, input.id, WRITE, 0).await?;
            let previous =
                sqlx::query("SELECT name,platformid FROM swarmservices WHERE id=$1 FOR UPDATE")
                    .bind(input.id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(storage)?
                    .ok_or(SwarmServiceError::NotFound)?;
            let old_name: String = previous.try_get("name").map_err(storage)?;
            let platform_id: Uuid = previous.try_get("platformid").map_err(storage)?;
            let changed=sqlx::query("UPDATE swarmservices SET name=$2,rowversion=rowversion+1,updatedat=$3 WHERE id=$1 AND controlstate='Idle'")
                .bind(input.id).bind(&input.name).bind(Utc::now()).execute(&mut *tx).await.map_err(database_error)?.rows_affected();
            if changed != 1 {
                return Err(SwarmServiceError::Conflict(
                    "The Service has an operation in progress.".to_owned(),
                ));
            }
            insert_swarm_activity(
                &mut tx,
                input.id,
                &input.name,
                platform_id,
                actor_id,
                ActivityEventInfo::swarm_service_renamed(old_name, input.name.clone()),
                ActivityStatus::Success,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor_id, administrator, input.id).await
        })
    }

    fn delete<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<ServiceDeletionClaim>, SwarmServiceError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let mut claims = Vec::with_capacity(ids.len());
            for id in ids {
                ensure_access(&mut tx, actor_id, administrator, *id, EXECUTE, 0).await?;
                let row=sqlx::query("SELECT platformid,dockerserviceid,controlstate FROM swarmservices WHERE id=$1 FOR UPDATE")
                    .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(SwarmServiceError::NotFound)?;
                ensure_idle(&row)?;
                sqlx::query("UPDATE swarmservices SET controlstate='Processing',controlstartedat=$2,controltriggeredby=$3,rowversion=rowversion+1 WHERE id=$1")
                    .bind(id).bind(Utc::now().timestamp()).bind(actor_id.value()).execute(&mut *tx).await.map_err(storage)?;
                claims.push(ServiceDeletionClaim {
                    id: *id,
                    platform_id: row.try_get("platformid").map_err(storage)?,
                    docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
                });
            }
            tx.commit().await.map_err(storage)?;
            Ok(claims)
        })
    }

    fn complete_delete<'a>(
        &'a self,
        actor_id: ActorId,
        deleted: &'a [ServiceDeletionClaim],
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            for claim in deleted {
                let row = sqlx::query("SELECT * FROM swarmservices WHERE id=$1 AND controlstate='Processing' FOR UPDATE")
                    .bind(claim.id).fetch_optional(&mut *tx).await.map_err(storage)?
                    .ok_or(SwarmServiceError::NotFound)?;
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
    fn release_delete<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            sqlx::query("UPDATE swarmservices SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE id=ANY($1::uuid[]) AND controlstate='Processing' AND updatecheckid IS NULL").bind(ids).execute(&self.pool).await.map_err(storage)?;
            Ok(())
        })
    }

    fn stale_deletion_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceDeletionClaim)>, SwarmServiceError>> {
        Box::pin(async move {
            sqlx::query("SELECT id,platformid,dockerserviceid,controltriggeredby FROM swarmservices WHERE controlstate='Processing' AND updatecheckid IS NULL AND controlstartedat <= $1 AND (operationstate IS NULL OR operationstate NOT IN ('Prepared','PendingAcceptance','Accepted')) AND controltriggeredby IS NOT NULL ORDER BY controlstartedat,id LIMIT $2")
                .bind(started_before).bind(limit).fetch_all(&self.pool).await.map_err(storage)?
                .into_iter().map(|row| Ok((
                    ActorId::new(row.try_get("controltriggeredby").map_err(storage)?),
                    ServiceDeletionClaim {
                        id: row.try_get("id").map_err(storage)?,
                        platform_id: row.try_get("platformid").map_err(storage)?,
                        docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
                    },
                ))).collect()
        })
    }

    fn claim_operation(
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
            let required = if kind == ServiceOperationKind::Scale {
                WRITE
            } else {
                READ
            };
            ensure_access(&mut tx, actor_id, administrator, id, required, APPLY).await?;
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
            sqlx::query("UPDATE swarmservices SET spec=$2,desiredspechash=$3,operationid=$4,operationkind=$5,operationstate='Prepared',targetdesiredspechash=$3,targetrowversion=$6,preparedat=$7,operationactorid=$8,operationclusterid=$10,basedockerversion=dockerversionindex,controlstate='Processing',controlstartedat=$9,controltriggeredby=$8,rowversion=rowversion+1,updatedat=$7 WHERE id=$1")
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

    fn mark_attempted<'a>(
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

    fn mark_accepted<'a>(
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
                return Err(SwarmServiceError::Conflict(
                    "The Service operation changed before Docker acceptance was recorded."
                        .to_owned(),
                ));
            }
            Ok(())
        })
    }

    fn complete_operation<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ServiceOperationClaim,
        result: &'a RuntimeServiceResult,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query("SELECT name,platformid,operationkind,spec FROM swarmservices WHERE id=$1 AND operationid=$2 FOR UPDATE")
                .bind(claim.id).bind(claim.operation_id).fetch_optional(&mut *tx).await.map_err(storage)?
                .ok_or(SwarmServiceError::NotFound)?;
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

    fn fail_operation<'a>(
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

    fn stale_operation_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceOperationClaim)>, SwarmServiceError>> {
        Box::pin(async move {
            sqlx::query("SELECT id,platformid,dockername,dockerserviceid,dockerversionindex,rowversion,desiredspechash,spec,operationid,operationactorid FROM swarmservices WHERE controlstate='Processing' AND updatecheckid IS NULL AND controlstartedat <= $1 AND operationid IS NOT NULL ORDER BY controlstartedat,id LIMIT $2")
            .bind(started_before).bind(limit).fetch_all(&self.pool).await.map_err(storage)?.into_iter().map(|row| Ok((ActorId::new(row.try_get("operationactorid").map_err(storage)?),ServiceOperationClaim{operation_id:row.try_get("operationid").map_err(storage)?,id:row.try_get("id").map_err(storage)?,platform_id:row.try_get("platformid").map_err(storage)?,docker_name:row.try_get("dockername").map_err(storage)?,docker_service_id:row.try_get("dockerserviceid").map_err(storage)?,docker_version_index:row.try_get("dockerversionindex").map_err(storage)?,row_version:row.try_get("rowversion").map_err(storage)?,desired_hash:row.try_get("desiredspechash").map_err(storage)?,spec:SwarmServiceSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?}))).collect()
        })
    }
}

async fn get_authorized(
    pool: &PgPool,
    actor_id: ActorId,
    administrator: bool,
    id: Uuid,
) -> Result<ManagedSwarmServiceView, SwarmServiceError> {
    let query = format!(
        "{AUTHORIZED_CTES}{PROJECTION} WHERE s.id=$3 AND ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(r.level_mask,0)) >= 1) LIMIT 1"
    );
    sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(actor_id.value())
        .bind(administrator)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .map(map_service)
        .transpose()?
        .ok_or(SwarmServiceError::NotFound)
}

fn map_service(row: PgRow) -> Result<ManagedSwarmServiceView, SwarmServiceError> {
    let spec = SwarmServiceSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
    let desired: String = row.try_get("desiredspechash").map_err(storage)?;
    let applied: Option<String> = row.try_get("lastapplieddesiredspechash").map_err(storage)?;
    let live: Option<String> = row.try_get("liveruntimehash").map_err(storage)?;
    let applied_runtime: Option<String> = row.try_get("lastappliedruntimehash").map_err(storage)?;
    let level: i32 = row.try_get("permission_level").map_err(storage)?;
    let specific: i32 = row.try_get("permission_specific").map_err(storage)?;
    let operation_id: Option<Uuid> = row.try_get("operationid").map_err(storage)?;
    let current_operation = operation_id
        .map(|id| {
            Ok(SwarmServiceOperationView {
                id,
                kind: row.try_get("operationkind").map_err(storage)?,
                state: row.try_get("operationstate").map_err(storage)?,
                prepared_at: row.try_get("preparedat").map_err(storage)?,
                attempted_at: row.try_get("attemptedat").map_err(storage)?,
                completed_at: row.try_get("completedat").map_err(storage)?,
                result_code: row.try_get("resultcode").map_err(storage)?,
                warnings: row
                    .try_get::<Option<Value>, _>("warnings")
                    .map_err(storage)?
                    .map(json)
                    .transpose()?
                    .unwrap_or_default(),
                result_message: row.try_get("resultmessage").map_err(storage)?,
            })
        })
        .transpose()?;
    let last_checked = match row.try_get::<Option<DateTime<Utc>>, _>("effective_last_checked_at") {
        Ok(Some(value)) => value,
        _ => dotnet_min_datetime(),
    };
    Ok(ManagedSwarmServiceView {
        id: row.try_get("id").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        docker_name: row.try_get("dockername").map_err(storage)?,
        docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
        spec,
        health: row.try_get("effective_health").map_err(storage)?,
        synchronization_state: row
            .try_get("effective_synchronization_state")
            .map_err(storage)?,
        control_state: row.try_get("controlstate").map_err(storage)?,
        auto_update_state: AutoUpdateState {
            last_checked_at: last_checked,
            status: row.try_get("autoupdatestate_status").map_err(storage)?,
            current_digest: row
                .try_get("autoupdatestate_currentdigest")
                .map_err(storage)?,
            remote_digest: row
                .try_get("autoupdatestate_remotedigest")
                .map_err(storage)?,
            last_error: row.try_get("autoupdatestate_lasterror").map_err(storage)?,
        },
        applied_image_digest: row.try_get("appliedimagedigest").map_err(storage)?,
        has_pending_desired_changes: applied.as_deref() != Some(desired.as_str()),
        has_runtime_drift: matches!((&applied_runtime,&live),(Some(a),Some(b)) if a!=b),
        row_version: row.try_get("rowversion").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        updated_at: row.try_get("updatedat").map_err(storage)?,
        platform_name: row.try_get("platform_name").map_err(storage)?,
        platform_status: row.try_get("platform_status").map_err(storage)?,
        running_task_count: row.try_get("runningtaskcount").map_err(storage)?,
        desired_task_count: row.try_get("desiredtaskcount").map_err(storage)?,
        update_state: row.try_get("projection_update_state").map_err(storage)?,
        update_message: row.try_get("updatemessage").map_err(storage)?,
        current_operation,
        tags: json(row.try_get("tags").map_err(storage)?)?,
        tasks: Some(json(row.try_get("tasks").map_err(storage)?)?),
        capabilities: Some(SwarmServiceCapabilities {
            can_view_logs: level >= READ,
            can_inspect: level >= READ,
            can_apply: level >= READ && specific & APPLY != 0,
            can_view_resource_bindings: level >= READ && specific & (1 << 5) != 0,
            can_read: level >= READ,
            can_write: level >= WRITE,
            can_execute: level >= EXECUTE,
        }),
    })
}

async fn ensure_platform(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    administrator: bool,
    platform_id: Uuid,
    online: bool,
) -> Result<(), SwarmServiceError> {
    let row = sqlx::query("SELECT status,platformdescriptor FROM platforms WHERE id=$1")
        .bind(platform_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(SwarmServiceError::NotFound)?;
    if !administrator
        && !has_access(tx, actor_id, PLATFORM_RESOURCE_TYPE, platform_id, READ, 0).await?
    {
        return Err(SwarmServiceError::NotFound);
    }
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    if descriptor.get("$type").and_then(Value::as_str) != Some("DockerSwarm") {
        return Err(SwarmServiceError::Validation(
            "Managed Swarm Services require a Docker Swarm platform.".to_owned(),
        ));
    }
    if online
        && (row.try_get::<String, _>("status").map_err(storage)? != "Online"
            || descriptor.get("controlAvailable").and_then(Value::as_bool) == Some(false))
    {
        return Err(SwarmServiceError::Conflict(
            "The Docker Swarm manager is not available.".to_owned(),
        ));
    }
    Ok(())
}
async fn validate_references(
    tx: &mut Transaction<'_, Postgres>,
    input: &CreateSwarmServiceInput,
) -> Result<(), SwarmServiceError> {
    ensure_platform(
        tx,
        ActorId::new(Uuid::nil()),
        true,
        input.platform_id,
        false,
    )
    .await?;
    match &input.spec.image {
        citadel_swarm_services::SwarmServiceImageInfo::External { registry_id, .. } => {
            ensure_exists(tx, "registries", *registry_id, "Registry").await?
        }
        citadel_swarm_services::SwarmServiceImageInfo::Build {
            build_project_id, ..
        } => ensure_exists(tx, "buildprojects", *build_project_id, "Build Project").await?,
    }
    for id in &input.spec.network_ids {
        let row=sqlx::query("SELECT ingress,scope FROM swarmnetworkprojections WHERE platformid=$1 AND dockernetworkid=$2").bind(input.platform_id).bind(id).fetch_optional(&mut **tx).await.map_err(storage)?.ok_or_else(||SwarmServiceError::Validation("One or more selected overlay Networks do not exist on this Swarm.".to_owned()))?;
        if row.try_get::<bool, _>("ingress").map_err(storage)?
            || row.try_get::<String, _>("scope").map_err(storage)? != "swarm"
        {
            return Err(SwarmServiceError::Validation("The Swarm ingress network is managed by Docker and cannot be attached to a Service explicitly.".to_owned()));
        }
    }
    for reference in &input.spec.secrets {
        let name = sqlx::query_scalar::<_, String>(
            "SELECT name FROM swarmsecretprojections WHERE platformid=$1 AND dockersecretid=$2",
        )
        .bind(input.platform_id)
        .bind(&reference.secret_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?;
        if name.as_deref() != Some(&reference.secret_name) {
            return Err(SwarmServiceError::Validation("One or more selected Docker Swarm Secrets do not exist or no longer match this Swarm.".to_owned()));
        }
    }
    for reference in &input.spec.configs {
        let name = sqlx::query_scalar::<_, String>(
            "SELECT name FROM swarmconfigprojections WHERE platformid=$1 AND dockerconfigid=$2",
        )
        .bind(input.platform_id)
        .bind(&reference.config_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?;
        if name.as_deref() != Some(&reference.config_name) {
            return Err(SwarmServiceError::Validation("One or more selected Docker Swarm Configs do not exist or no longer match this Swarm.".to_owned()));
        }
    }
    Ok(())
}
async fn ensure_exists(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    id: Uuid,
    label: &str,
) -> Result<(), SwarmServiceError> {
    let query = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=$1)");
    if sqlx::query_scalar::<_, bool>(AssertSqlSafe(query.as_str()))
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?
    {
        Ok(())
    } else {
        Err(SwarmServiceError::Validation(format!(
            "The selected {label} does not exist."
        )))
    }
}
async fn validate_tags(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<(), SwarmServiceError> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tags WHERE id=ANY($1::uuid[])")
        .bind(ids)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    if count == ids.len() as i64 {
        Ok(())
    } else {
        Err(SwarmServiceError::Validation(
            "One or more Tags do not exist.".to_owned(),
        ))
    }
}
async fn replace_tags(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    actor: ActorId,
    tags: &[Uuid],
) -> Result<(), SwarmServiceError> {
    for tag in tags {
        sqlx::query("INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid) VALUES('SwarmService',$1,$2,$3)").bind(id).bind(tag).bind(actor.value()).execute(&mut **tx).await.map_err(database_error)?;
    }
    Ok(())
}
async fn ensure_access(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    id: Uuid,
    level: i32,
    specific: i32,
) -> Result<(), SwarmServiceError> {
    if administrator || has_access(tx, actor, RESOURCE_TYPE, id, level, specific).await? {
        Ok(())
    } else {
        Err(SwarmServiceError::NotFound)
    }
}
async fn has_access(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    resource_type: i32,
    id: Uuid,
    level: i32,
    specific: i32,
) -> Result<bool, SwarmServiceError> {
    let (actual_level,actual_specific)=sqlx::query_as::<_,(i32,i32)>(r#"WITH scope AS(SELECT id actorid FROM actors WHERE id=$1 AND isenabled UNION SELECT team.actorid FROM actorteammemberships m JOIN teams team ON team.id=m.teamid JOIN actors a ON a.id=team.actorid AND a.isenabled WHERE m.memberactorid=$1), grants AS(SELECT p.permissionlevel,p.specificpermissions FROM scope s JOIN actorroles ar ON ar.actorid=s.actorid JOIN permissions p ON p.roleid=ar.roleid WHERE p.resourcetype=$2 UNION ALL SELECT ra.permissionlevel,ra.specificpermissions FROM scope s JOIN resourceaccesses ra ON ra.actorid=s.actorid WHERE ra.resourcetype=$2 AND ra.resourceid=$3) SELECT COALESCE(MAX(permissionlevel),0)::integer,COALESCE(bit_or(specificpermissions),0)::integer FROM grants"#).bind(actor.value()).bind(resource_type).bind(id).fetch_one(&mut **tx).await.map_err(storage)?;
    Ok(actual_level >= level && (specific == 0 || actual_specific & specific == specific))
}
fn ensure_idle(row: &PgRow) -> Result<(), SwarmServiceError> {
    if row.try_get::<String, _>("controlstate").map_err(storage)? == "Idle" {
        Ok(())
    } else {
        Err(SwarmServiceError::Conflict(
            "The Service already has an operation in progress.".to_owned(),
        ))
    }
}
fn ensure_swarm_manager(row: &PgRow) -> Result<(), SwarmServiceError> {
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    if descriptor.get("$type").and_then(Value::as_str) != Some("DockerSwarm") {
        return Err(SwarmServiceError::Validation(
            "Managed Services require a Docker Swarm platform.".to_owned(),
        ));
    }
    if row
        .try_get::<String, _>("platform_status")
        .map_err(storage)?
        != "Online"
        || descriptor.get("controlAvailable").and_then(Value::as_bool) == Some(false)
    {
        return Err(SwarmServiceError::Conflict(
            "The Docker Swarm manager is not available.".to_owned(),
        ));
    }
    Ok(())
}
fn docker_name(name: &str, id: Uuid) -> String {
    let mut out = String::with_capacity(name.len().min(48) + 9);
    let mut separator = false;
    for ch in name.trim().to_ascii_lowercase().chars() {
        let normalized = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
            ch
        } else {
            '-'
        };
        if normalized == '-' && separator {
            continue;
        }
        out.push(normalized);
        separator = normalized == '-';
        if out.len() >= 48 {
            break;
        }
    }
    let base = out.trim_matches(['-', '_']);
    let base = if base.is_empty() { "service" } else { base };
    format!("{base}-{}", &id.simple().to_string()[..8])
}
fn json<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, SwarmServiceError> {
    serde_json::from_value(value).map_err(storage)
}
fn dotnet_min_datetime() -> DateTime<Utc> {
    DateTime::from_timestamp(-62_135_596_800, 0).expect(".NET minimum date is representable")
}
fn activity_snapshot(row: &PgRow) -> Result<SwarmServiceActivitySnapshot, SwarmServiceError> {
    Ok(SwarmServiceActivitySnapshot {
        id: row.try_get("id").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        docker_name: row.try_get("dockername").map_err(storage)?,
        docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
        spec: row.try_get("spec").map_err(storage)?,
    })
}
async fn insert_swarm_activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: &str,
    platform_id: Uuid,
    actor_id: ActorId,
    info: ActivityEventInfo,
    status: ActivityStatus,
) -> Result<(), SwarmServiceError> {
    let activity = ActivityEvent::new_swarm_service_event(
        id,
        name.to_owned(),
        platform_id,
        actor_id,
        info,
        status,
        Utc::now(),
    )
    .map_err(storage)?;
    insert_activity(tx, &activity).await.map_err(storage)
}
fn database_error(error: sqlx::Error) -> SwarmServiceError {
    if let sqlx::Error::Database(db) = &error
        && db.code().as_deref() == Some("23505")
    {
        return SwarmServiceError::Conflict(
            "A Service with the same name already exists on this Platform.".to_owned(),
        );
    }
    storage(error)
}
fn storage(error: impl std::fmt::Display) -> SwarmServiceError {
    SwarmServiceError::Storage(error.to_string())
}
