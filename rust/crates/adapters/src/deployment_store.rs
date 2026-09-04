use chrono::{DateTime, Utc};
use citadel_deployments::{
    ApplyClaim, AutoUpdateState, CreateDeploymentInput, CreateDeploymentInputView, DeletionClaim,
    DeploymentBindingSnapshot, DeploymentCapabilities, DeploymentDuplicateDraftView,
    DeploymentError, DeploymentFilter, DeploymentImageInfo, DeploymentSpec, DeploymentStore,
    DeploymentView, DuplicateSourceInput, DuplicateWarning, EffectiveDeploymentPermission,
    FieldPatch, PatchDeploymentMetadataInput, RuntimeContainerState, RuntimeDeploymentResult,
    TagSummary,
};
use citadel_domain::{
    ActivityEvent, ActivityEventInfo, ActivityResourceType, ActivitySourceResource, ActivityStatus,
    ActorId, DeploymentActivitySnapshot, DeploymentResultActivitySnapshot,
};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::insert_activity;

const DEPLOYMENT_RESOURCE_TYPE: i32 = 1;
const PLATFORM_RESOURCE_TYPE: i32 = 0;
const READ_LEVEL: i32 = 1;
const WRITE_LEVEL: i32 = 2;
const EXECUTE_LEVEL: i32 = 4;
const RESOURCE_BINDINGS_PERMISSION: i32 = 1 << 5;
const APPLY_PERMISSION: i32 = 1 << 2;

const AUTHORIZED_CTES: &str = r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid
    FROM actors actor
    WHERE actor.id = $1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid = $1
), global_permission AS (
    SELECT COALESCE(MAX(permission.permissionlevel), 0)::integer AS level_mask,
           COALESCE(bit_or(permission.specificpermissions), 0)::integer AS specific_mask
    FROM actor_scope scope
    JOIN actorroles assignment ON assignment.actorid = scope.actorid
    JOIN permissions permission ON permission.roleid = assignment.roleid
    WHERE permission.resourcetype = 1
), resource_permissions AS (
    SELECT access.resourceid,
           COALESCE(MAX(access.permissionlevel), 0)::integer AS level_mask,
           COALESCE(bit_or(access.specificpermissions), 0)::integer AS specific_mask
    FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid = scope.actorid
    WHERE access.resourcetype = 1
    GROUP BY access.resourceid
)
"#;

const PROJECTION: &str = r#"
SELECT d.id, d.name, d.description, d.platformid, d.createdat, d.createdbyactorid,
       d.status, d.controlstate, d.rowversion, d.spec,
       CASE WHEN d.autoupdatestate_lastcheckedat = '-infinity'::timestamptz
            THEN TIMESTAMPTZ '0001-01-01 00:00:00+00'
            ELSE d.autoupdatestate_lastcheckedat END AS autoupdatestate_lastcheckedat,
       d.autoupdatestate_status,
       d.autoupdatestate_currentdigest, d.autoupdatestate_remotedigest,
       d.autoupdatestate_lasterror,
       p.status AS platform_status, p.name AS platform_name,
       c.id AS container_id, c.dockercontainerid, c.dockerimageid,
       i.id AS image_id, i.name AS image_name,
       COALESCE(tags.value, '[]'::jsonb) AS tags,
       activity.value AS latest_activity,
       CASE WHEN $2 THEN 7 ELSE GREATEST(COALESCE(g.level_mask, 0), COALESCE(r.level_mask, 0)) END AS permission_level,
       CASE WHEN $2 THEN 63 ELSE (COALESCE(g.specific_mask, 0) | COALESCE(r.specific_mask, 0)) END AS permission_specific
FROM deployments d
JOIN platforms p ON p.id = d.platformid
LEFT JOIN global_permission g ON TRUE
LEFT JOIN resource_permissions r ON r.resourceid = d.id
LEFT JOIN LATERAL (
    SELECT container.id, container.dockercontainerid, container.dockerimageid, container.imageid
    FROM containers container
    WHERE container.deploymentid = d.id
    ORDER BY container.updated DESC, container.id DESC
    LIMIT 1
) c ON TRUE
LEFT JOIN images i ON i.id = COALESCE(
    c.imageid,
    CASE WHEN d.spec -> 'Image' ->> '$type' = 'Local'
              AND d.spec -> 'Image' ->> 'ImageId' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
         THEN CAST(d.spec -> 'Image' ->> 'ImageId' AS uuid) END)
LEFT JOIN LATERAL (
    SELECT jsonb_agg(jsonb_build_object('id', tag.id, 'name', tag.name, 'color', tag.color)
                     ORDER BY tag.name, tag.id) AS value
    FROM resourcetags link
    JOIN tags tag ON tag.id = link.tagid
    WHERE link.resourcetype = 'Deployment' AND link.resourceid = d.id
) tags ON TRUE
LEFT JOIN LATERAL (
    SELECT jsonb_build_object(
        'id', event.id, 'resourceType', event.resourcetype, 'eventType', event.eventtype,
        'status', event.status, 'info', event.info::jsonb, 'createdAt', event.createdat) AS value
    FROM activityevents event
    WHERE event.resourceid = d.id AND event.resourcetype = 'Deployment'
      AND event.eventtype NOT IN ('DeploymentUpdated', 'DeploymentRenamed')
    ORDER BY event.createdat DESC, event.id DESC
    LIMIT 1
) activity ON TRUE
"#;

#[derive(Clone)]
pub struct PostgresDeploymentStore {
    pool: PgPool,
}

impl PostgresDeploymentStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl DeploymentStore for PostgresDeploymentStore {
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        filter: &'a DeploymentFilter,
    ) -> BoxFuture<'a, Result<Vec<DeploymentView>, DeploymentError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTES}{PROJECTION}
WHERE ($2 OR GREATEST(COALESCE(g.level_mask, 0), COALESCE(r.level_mask, 0)) >= 1)
  AND ($3::uuid IS NULL OR d.platformid = $3)
  AND NOT EXISTS (
      SELECT 1 FROM unnest($4::text[]) requested(name)
      WHERE NOT EXISTS (
          SELECT 1 FROM resourcetags filter_link
          JOIN tags filter_tag ON filter_tag.id = filter_link.tagid
          WHERE filter_link.resourcetype = 'Deployment'
            AND filter_link.resourceid = d.id
            AND (lower(filter_tag.name) = lower(requested.name)
                 OR filter_tag.id::text = requested.name)))
ORDER BY d.createdat DESC, d.name, d.id"#
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
                .map(map_deployment)
                .collect()
        })
    }

    fn get_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>> {
        Box::pin(async move { get_authorized(&self.pool, actor_id, administrator, id).await })
    }

    fn create<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a CreateDeploymentInput,
    ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_platform(&mut tx, actor_id, administrator, input.platform_id).await?;
            validate_image_references(&mut tx, &input.spec.image).await?;
            validate_tags(&mut tx, &input.tag_ids).await?;

            let duplicate = if let Some(source) = &input.duplicate_source {
                ensure_access(
                    &mut tx,
                    actor_id,
                    administrator,
                    source.resource_id,
                    READ_LEVEL,
                )
                .await?;
                let row = sqlx::query(
                    "SELECT d.name, p.platformdescriptor FROM deployments d JOIN platforms p ON p.id=d.platformid WHERE d.id=$1",
                )
                .bind(source.resource_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(DeploymentError::NotFound)?;
                let source_descriptor: Value =
                    row.try_get("platformdescriptor").map_err(storage)?;
                if platform_kind(&source_descriptor) != "Docker" {
                    return Err(DeploymentError::Validation(
                        "A Deployment duplicate must use a Docker Standalone source.".to_owned(),
                    ));
                }
                require_duplicate_binding_access(
                    &mut tx,
                    actor_id,
                    administrator,
                    source.resource_id,
                )
                .await?;
                Some((
                    source.resource_id,
                    row.try_get::<String, _>("name").map_err(storage)?,
                ))
            } else {
                None
            };

            let id = Uuid::now_v7();
            let now = Utc::now();
            let spec = input.spec.to_storage_value()?;
            sqlx::query(
                r#"INSERT INTO deployments (
                    id,name,description,platformid,status,createdat,createdbyactorid,spec,
                    autoupdatestate_lastcheckedat,autoupdatestate_status,controlstate,rowversion)
                   VALUES ($1,$2,$3,$4,'Created',$5,$6,$7,'-infinity','Unknown','Idle',0)"#,
            )
            .bind(id)
            .bind(&input.name)
            .bind(&input.description)
            .bind(input.platform_id)
            .bind(now)
            .bind(actor_id.value())
            .bind(spec.clone())
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
            insert_tags(&mut tx, id, actor_id, &input.tag_ids).await?;
            if let Some((source_id, _)) = duplicate.as_ref() {
                copy_bindings(&mut tx, *source_id, id).await?;
            }
            let snapshot = snapshot(
                id,
                &input.name,
                input.platform_id,
                input.description.clone(),
                spec,
            );
            let info = match duplicate {
                Some((source_id, source_name)) => ActivityEventInfo::deployment_duplicated(
                    snapshot,
                    ActivitySourceResource {
                        resource_type: ActivityResourceType::Deployment,
                        resource_id: source_id,
                        resource_name: source_name,
                    },
                ),
                None => ActivityEventInfo::deployment_created(snapshot),
            };
            insert_deployment_activity(
                &mut tx,
                id,
                &input.name,
                input.platform_id,
                actor_id,
                info,
                now,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor_id, administrator, id).await
        })
    }

    fn update_config<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        expected_row_version: i64,
        spec: &'a DeploymentSpec,
    ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(&mut tx, actor_id, administrator, id, WRITE_LEVEL).await?;
            let row = sqlx::query(
                "SELECT name,description,platformid,spec,controlstate,rowversion FROM deployments WHERE id=$1 FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(DeploymentError::NotFound)?;
            ensure_idle(&row)?;
            let platform_id: Uuid = row.try_get("platformid").map_err(storage)?;
            ensure_platform(&mut tx, actor_id, administrator, platform_id).await?;
            validate_image_references(&mut tx, &spec.image).await?;
            let old_spec: Value = row.try_get("spec").map_err(storage)?;
            if row.try_get::<i64, _>("rowversion").map_err(storage)? != expected_row_version {
                return Err(DeploymentError::Conflict(
                    "The Deployment configuration changed while it was being updated.".to_owned(),
                ));
            }
            let next_spec = spec.to_storage_value()?;
            sqlx::query("UPDATE deployments SET spec=$2,rowversion=rowversion+1 WHERE id=$1")
                .bind(id)
                .bind(&next_spec)
                .execute(&mut *tx)
                .await
                .map_err(database_error)?;
            let name: String = row.try_get("name").map_err(storage)?;
            let description: Option<String> = row.try_get("description").map_err(storage)?;
            insert_deployment_activity(
                &mut tx,
                id,
                &name,
                platform_id,
                actor_id,
                ActivityEventInfo::deployment_updated(
                    snapshot(id, &name, platform_id, description.clone(), old_spec),
                    snapshot(id, &name, platform_id, description, next_spec),
                ),
                Utc::now(),
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor_id, administrator, id).await
        })
    }

    fn update_metadata<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: &'a PatchDeploymentMetadataInput,
    ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(&mut tx, actor_id, administrator, id, WRITE_LEVEL).await?;
            let row = sqlx::query(
                "SELECT name,description,platformid,spec,controlstate FROM deployments WHERE id=$1 FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(DeploymentError::NotFound)?;
            ensure_idle(&row)?;
            let old_description: Option<String> = row.try_get("description").map_err(storage)?;
            let next_description = match &input.description {
                FieldPatch::Unchanged => old_description.clone(),
                FieldPatch::Set(value) => Some(value.clone()),
                FieldPatch::Clear => None,
            };
            sqlx::query(
                "UPDATE deployments SET description=$2,rowversion=rowversion+1 WHERE id=$1",
            )
            .bind(id)
            .bind(&next_description)
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
            let name: String = row.try_get("name").map_err(storage)?;
            let platform_id: Uuid = row.try_get("platformid").map_err(storage)?;
            let spec: Value = row.try_get("spec").map_err(storage)?;
            insert_deployment_activity(
                &mut tx,
                id,
                &name,
                platform_id,
                actor_id,
                ActivityEventInfo::deployment_updated(
                    snapshot(id, &name, platform_id, old_description, spec.clone()),
                    snapshot(id, &name, platform_id, next_description, spec),
                ),
                Utc::now(),
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
        id: Uuid,
        name: &'a str,
    ) -> BoxFuture<'a, Result<DeploymentView, DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(&mut tx, actor_id, administrator, id, WRITE_LEVEL).await?;
            let row = sqlx::query(
                "SELECT name,platformid,controlstate FROM deployments WHERE id=$1 FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(DeploymentError::NotFound)?;
            ensure_idle(&row)?;
            let old_name: String = row.try_get("name").map_err(storage)?;
            let platform_id: Uuid = row.try_get("platformid").map_err(storage)?;
            if old_name == name {
                tx.commit().await.map_err(storage)?;
                return get_authorized(&self.pool, actor_id, administrator, id).await;
            }
            sqlx::query("UPDATE deployments SET name=$2,rowversion=rowversion+1 WHERE id=$1")
                .bind(id)
                .bind(name)
                .execute(&mut *tx)
                .await
                .map_err(database_error)?;
            insert_deployment_activity(
                &mut tx,
                id,
                name,
                platform_id,
                actor_id,
                ActivityEventInfo::deployment_renamed(old_name, name.to_owned()),
                Utc::now(),
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor_id, administrator, id).await
        })
    }

    fn duplicate_draft<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<DeploymentDuplicateDraftView, DeploymentError>> {
        Box::pin(async move {
            let source = get_authorized(&self.pool, actor_id, administrator, id).await?;
            let name =
                available_duplicate_name(&self.pool, &source.name, source.platform_id).await?;
            let warnings = has_likely_host_bind(source.spec.volumes.as_deref())
                .then(|| DuplicateWarning {
                    code: "HOST_BIND_MOUNT".to_owned(),
                    message: "This deployment contains host paths that may not exist on another platform.".to_owned(),
                    field_path: Some("spec.volumes".to_owned()),
                })
                .into_iter()
                .collect();
            Ok(DeploymentDuplicateDraftView {
                draft: CreateDeploymentInputView {
                    name,
                    platform_id: source.platform_id,
                    description: source.description,
                    spec: source.spec.for_create(),
                    tag_ids: source.tags.iter().map(|tag| tag.id).collect(),
                    duplicate_source: DuplicateSourceInput {
                        resource_type: "Deployment".to_owned(),
                        resource_id: source.id,
                        resource_name: source.name,
                    },
                },
                warnings,
            })
        })
    }

    fn claim_delete<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<DeletionClaim>, DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let rows = sqlx::query(
                r#"SELECT d.id,d.platformid,d.name,d.description,d.spec,d.status,d.controlstate,d.rowversion,
                          ARRAY(
                              SELECT container.dockercontainerid FROM containers container
                              WHERE container.deploymentid=d.id AND container.dockercontainerid <> ''
                              ORDER BY container.updated DESC,container.id DESC
                          ) AS dockercontainerids
                   FROM deployments d
                   WHERE d.id=ANY($1::uuid[]) ORDER BY d.id FOR UPDATE OF d"#,
            )
            .bind(ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(storage)?;
            if rows.len() != ids.len() {
                return Err(DeploymentError::NotFound);
            }
            for id in ids {
                ensure_access(&mut tx, actor_id, administrator, *id, EXECUTE_LEVEL).await?;
            }
            if rows.iter().any(|row| {
                row.try_get::<String, _>("controlstate")
                    .is_ok_and(|state| state != "Idle")
            }) {
                return Err(DeploymentError::Conflict(
                    "One or more Deployments already have an operation in progress.".to_owned(),
                ));
            }
            let now = Utc::now().timestamp();
            sqlx::query(
                "UPDATE deployments SET status='Pending',controlstate='Processing',controltriggeredby=$2,controlstartedat=$3,rowversion=rowversion+1 WHERE id=ANY($1::uuid[])",
            )
            .bind(ids)
            .bind(actor_id.value())
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
            let claims = rows
                .into_iter()
                .map(|row| {
                    let spec: Value = row.try_get("spec").map_err(storage)?;
                    Ok(DeletionClaim {
                        id: row.try_get("id").map_err(storage)?,
                        platform_id: row.try_get("platformid").map_err(storage)?,
                        name: row.try_get("name").map_err(storage)?,
                        docker_container_ids: row.try_get("dockercontainerids").map_err(storage)?,
                        row_version: row.try_get::<i64, _>("rowversion").map_err(storage)? + 1,
                        previous_status: row.try_get("status").map_err(storage)?,
                        description: row.try_get("description").map_err(storage)?,
                        spec: DeploymentSpec::from_storage_value(spec)?,
                    })
                })
                .collect::<Result<Vec<_>, DeploymentError>>()?;
            tx.commit().await.map_err(storage)?;
            Ok(claims)
        })
    }

    fn complete_delete<'a>(
        &'a self,
        actor_id: ActorId,
        claims: &'a [DeletionClaim],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            for claim in claims {
                let locked = sqlx::query_scalar::<_, Uuid>(
                    "SELECT id FROM deployments WHERE id=$1 AND rowversion=$2 AND controlstate='Processing' AND controltriggeredby=$3 FOR UPDATE",
                )
                .bind(claim.id)
                .bind(claim.row_version)
                .bind(actor_id.value())
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?;
                if locked.is_none() {
                    return Err(DeploymentError::Conflict(
                        "A Deployment changed while deletion was in progress.".to_owned(),
                    ));
                }
                let stored_spec = claim.spec.to_storage_value()?;
                insert_deployment_activity(
                    &mut tx,
                    claim.id,
                    &claim.name,
                    claim.platform_id,
                    actor_id,
                    ActivityEventInfo::deployment_deleted(snapshot(
                        claim.id,
                        &claim.name,
                        claim.platform_id,
                        claim.description.clone(),
                        stored_spec,
                    )),
                    Utc::now(),
                )
                .await?;
            }
            let ids = claims.iter().map(|claim| claim.id).collect::<Vec<_>>();
            sqlx::query("DELETE FROM resourcetags WHERE resourcetype='Deployment' AND resourceid=ANY($1::uuid[])")
                .bind(&ids).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("DELETE FROM resourcebindings WHERE scope='Deployment' AND resourceid=ANY($1::uuid[])")
                .bind(&ids).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query(
                "DELETE FROM resourceaccesses WHERE resourcetype=1 AND resourceid=ANY($1::uuid[])",
            )
            .bind(&ids)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
            sqlx::query("DELETE FROM containers WHERE deploymentid=ANY($1::uuid[])")
                .bind(&ids)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            let affected = sqlx::query("DELETE FROM deployments WHERE id=ANY($1::uuid[])")
                .bind(&ids)
                .execute(&mut *tx)
                .await
                .map_err(database_error)?
                .rows_affected();
            if affected != ids.len() as u64 {
                return Err(DeploymentError::Conflict(
                    "Not all claimed Deployments could be deleted.".to_owned(),
                ));
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn release_delete<'a>(
        &'a self,
        claims: &'a [DeletionClaim],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            for claim in claims {
                let affected = sqlx::query(
                    "UPDATE deployments SET status=$3,controlstate='Idle',controltriggeredby=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND rowversion=$2 AND controlstate='Processing'",
                )
                .bind(claim.id)
                .bind(claim.row_version)
                .bind(&claim.previous_status)
                .execute(&mut *tx)
                .await
                .map_err(storage)?
                .rows_affected();
                if affected != 1 {
                    return Err(DeploymentError::Conflict(
                        "A Deployment changed while its deletion claim was being released."
                            .to_owned(),
                    ));
                }
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn claim_apply<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<ApplyClaim, DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_specific_access(
                &mut tx,
                actor_id,
                administrator,
                id,
                EXECUTE_LEVEL,
                APPLY_PERMISSION,
            )
            .await?;
            let row = sqlx::query(
                r#"SELECT d.id,d.platformid,d.name,d.description,d.spec,d.controlstate,d.rowversion,
                          p.address,p.platformdescriptor,
                          c.id AS containerid,c.dockercontainerid,c.platformid AS containerplatformid
                   FROM deployments d
                   JOIN platforms p ON p.id=d.platformid
                   LEFT JOIN LATERAL (
                       SELECT id,dockercontainerid,platformid FROM containers
                       WHERE deploymentid=d.id ORDER BY updated DESC,id DESC LIMIT 1
                   ) c ON TRUE
                   WHERE d.id=$1 FOR UPDATE OF d"#,
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(DeploymentError::NotFound)?;
            ensure_idle(&row)?;
            let platform_id: Uuid = row.try_get("platformid").map_err(storage)?;
            let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
            if platform_kind(&descriptor) != "Docker" {
                return Err(DeploymentError::Validation(
                    "Applying Docker Swarm services is not available through Deployments."
                        .to_owned(),
                ));
            }
            let container_platform_id: Option<Uuid> =
                row.try_get("containerplatformid").map_err(storage)?;
            if container_platform_id.is_some_and(|value| value != platform_id) {
                return Err(DeploymentError::Conflict(
                    "This deployment references a different platform than its existing container. Duplicate it on the target platform or restore the original platform before applying."
                        .to_owned(),
                ));
            }
            let row_version = row.try_get::<i64, _>("rowversion").map_err(storage)? + 1;
            let affected = sqlx::query(
                "UPDATE deployments SET status='Applying',controlstate='Processing',controltriggeredby=$2,controlstartedat=$3,rowversion=rowversion+1 WHERE id=$1 AND controlstate='Idle'",
            )
            .bind(id)
            .bind(actor_id.value())
            .bind(Utc::now().timestamp())
            .execute(&mut *tx)
            .await
            .map_err(database_error)?
            .rows_affected();
            if affected != 1 {
                return Err(DeploymentError::Conflict(
                    "The Deployment already has an operation in progress.".to_owned(),
                ));
            }
            let spec = DeploymentSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
            let claim = ApplyClaim {
                id,
                platform_id,
                platform_address: row.try_get("address").map_err(storage)?,
                name: row.try_get("name").map_err(storage)?,
                row_version,
                description: row.try_get("description").map_err(storage)?,
                spec,
                existing_container_id: row.try_get("containerid").map_err(storage)?,
                existing_docker_container_id: row.try_get("dockercontainerid").map_err(storage)?,
            };
            tx.commit().await.map_err(storage)?;
            Ok(claim)
        })
    }

    fn complete_apply<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ApplyClaim,
        result: &'a RuntimeDeploymentResult,
        digest: Option<&'a str>,
        bindings: &'a [DeploymentBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            lock_apply_claim(&mut tx, actor_id, claim).await?;
            let container_id = upsert_apply_container(&mut tx, claim, result).await?;
            let mut spec = claim.spec.clone();
            if let DeploymentImageInfo::External {
                registry_id,
                image_tag,
                resolved_digest,
            } = &spec.image
            {
                spec.image = DeploymentImageInfo::External {
                    registry_id: *registry_id,
                    image_tag: image_tag.clone(),
                    resolved_digest: digest
                        .map(str::to_owned)
                        .or_else(|| resolved_digest.clone()),
                };
            }
            let spec_value = spec.to_storage_value()?;
            let affected = sqlx::query(
                "UPDATE deployments SET status='Healthy',spec=$4,controlstate='Idle',controltriggeredby=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND rowversion=$2 AND controlstate='Processing' AND controltriggeredby=$3",
            )
            .bind(claim.id)
            .bind(claim.row_version)
            .bind(actor_id.value())
            .bind(&spec_value)
            .execute(&mut *tx)
            .await
            .map_err(storage)?
            .rows_affected();
            if affected != 1 {
                return Err(DeploymentError::Conflict(
                    "The Deployment changed while Apply was completing.".to_owned(),
                ));
            }
            insert_apply_activity(
                &mut tx,
                actor_id,
                claim,
                ActivityStatus::Success,
                apply_result(
                    Some(vec![result.docker_container_id.clone()]),
                    None,
                    bindings,
                )?,
                Some(&spec),
            )
            .await?;
            // Keep the explicit link visible before the inventory event replaces the projection.
            sqlx::query("UPDATE containers SET deploymentid=$2 WHERE id=$1")
                .bind(container_id)
                .bind(claim.id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn fail_apply<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ApplyClaim,
        message: &'a str,
        result: Option<&'a RuntimeDeploymentResult>,
        bindings: &'a [DeploymentBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            lock_apply_claim(&mut tx, actor_id, claim).await?;
            if let Some(result) = result {
                upsert_apply_container(&mut tx, claim, result).await?;
            }
            let affected = sqlx::query(
                "UPDATE deployments SET status='Failed',controlstate='Idle',controltriggeredby=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND rowversion=$2 AND controlstate='Processing' AND controltriggeredby=$3",
            )
            .bind(claim.id)
            .bind(claim.row_version)
            .bind(actor_id.value())
            .execute(&mut *tx)
            .await
            .map_err(storage)?
            .rows_affected();
            if affected != 1 {
                return Err(DeploymentError::Conflict(
                    "The Deployment changed while Apply failure was being recorded.".to_owned(),
                ));
            }
            insert_apply_activity(
                &mut tx,
                actor_id,
                claim,
                ActivityStatus::Failure,
                apply_result(
                    result.map(|value| vec![value.docker_container_id.clone()]),
                    Some(message.to_owned()),
                    bindings,
                )?,
                None,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn stale_apply_claims<'a>(
        &'a self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<(ActorId, ApplyClaim)>, DeploymentError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"SELECT d.id,d.platformid,d.name,d.description,d.spec,d.rowversion,
                          d.controltriggeredby,p.address,
                          c.id AS containerid,c.dockercontainerid
                   FROM deployments d
                   JOIN platforms p ON p.id=d.platformid
                   LEFT JOIN LATERAL (
                       SELECT id,dockercontainerid FROM containers
                       WHERE deploymentid=d.id ORDER BY updated DESC,id DESC LIMIT 1
                   ) c ON TRUE
                   WHERE d.controlstate='Processing' AND d.status='Applying'
                     AND d.controlstartedat IS NOT NULL AND d.controlstartedat < $1
                     AND d.controltriggeredby IS NOT NULL
                   ORDER BY d.controlstartedat,d.id LIMIT $2"#,
            )
            .bind(started_before)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
            rows.into_iter()
                .map(|row| {
                    let actor_id =
                        ActorId::new(row.try_get("controltriggeredby").map_err(storage)?);
                    let spec =
                        DeploymentSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                    Ok((
                        actor_id,
                        ApplyClaim {
                            id: row.try_get("id").map_err(storage)?,
                            platform_id: row.try_get("platformid").map_err(storage)?,
                            platform_address: row.try_get("address").map_err(storage)?,
                            name: row.try_get("name").map_err(storage)?,
                            row_version: row.try_get("rowversion").map_err(storage)?,
                            description: row.try_get("description").map_err(storage)?,
                            spec,
                            existing_container_id: row.try_get("containerid").map_err(storage)?,
                            existing_docker_container_id: row
                                .try_get("dockercontainerid")
                                .map_err(storage)?,
                        },
                    ))
                })
                .collect()
        })
    }
}

async fn get_authorized(
    pool: &PgPool,
    actor_id: ActorId,
    administrator: bool,
    id: Uuid,
) -> Result<DeploymentView, DeploymentError> {
    let query = format!(
        r#"{AUTHORIZED_CTES}{PROJECTION}
WHERE d.id=$3 AND ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(r.level_mask,0)) >= 1)
LIMIT 1"#
    );
    sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(actor_id.value())
        .bind(administrator)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .map(map_deployment)
        .transpose()?
        .ok_or(DeploymentError::NotFound)
}

async fn ensure_platform(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    administrator: bool,
    platform_id: Uuid,
) -> Result<(), DeploymentError> {
    let descriptor =
        sqlx::query_scalar::<_, Value>("SELECT platformdescriptor FROM platforms WHERE id=$1")
            .bind(platform_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?
            .ok_or(DeploymentError::NotFound)?;
    if !administrator
        && !has_resource_access(
            tx,
            actor_id,
            PLATFORM_RESOURCE_TYPE,
            platform_id,
            READ_LEVEL,
        )
        .await?
    {
        return Err(DeploymentError::NotFound);
    }
    let kind = platform_kind(&descriptor);
    if kind == "DockerSwarm" {
        return Err(DeploymentError::NotFound);
    }
    if kind != "Docker" {
        return Err(DeploymentError::Validation(
            "Deployments require a Docker Standalone platform.".to_owned(),
        ));
    }
    Ok(())
}

async fn validate_image_references(
    tx: &mut Transaction<'_, Postgres>,
    image: &DeploymentImageInfo,
) -> Result<(), DeploymentError> {
    let (table, id) = match image {
        DeploymentImageInfo::External { registry_id, .. } => ("registries", *registry_id),
        DeploymentImageInfo::Build {
            build_project_id, ..
        } => ("buildprojects", *build_project_id),
        DeploymentImageInfo::Local { .. } => return Ok(()),
    };
    let query = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=$1)");
    let exists = sqlx::query_scalar::<_, bool>(AssertSqlSafe(query.as_str()))
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    if exists {
        Ok(())
    } else {
        Err(DeploymentError::Validation(format!(
            "The selected {} does not exist.",
            if table == "registries" {
                "Registry"
            } else {
                "Build Project"
            }
        )))
    }
}

async fn validate_tags(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<(), DeploymentError> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tags WHERE id=ANY($1::uuid[])")
        .bind(ids)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    if count == ids.len() as i64 {
        Ok(())
    } else {
        Err(DeploymentError::Validation(
            "One or more Tags do not exist.".to_owned(),
        ))
    }
}

async fn insert_tags(
    tx: &mut Transaction<'_, Postgres>,
    deployment_id: Uuid,
    actor_id: ActorId,
    ids: &[Uuid],
) -> Result<(), DeploymentError> {
    for tag_id in ids {
        sqlx::query("INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid) VALUES('Deployment',$1,$2,$3)")
            .bind(deployment_id).bind(tag_id).bind(actor_id.value())
            .execute(&mut **tx).await.map_err(database_error)?;
    }
    Ok(())
}

async fn copy_bindings(
    tx: &mut Transaction<'_, Postgres>,
    source_id: Uuid,
    destination_id: Uuid,
) -> Result<(), DeploymentError> {
    sqlx::query(
        r#"INSERT INTO resourcebindings(
               id,createdat,kind,name,resourceid,scope,secretdeliverymode,secretid,targetpath,updatedat,value)
           SELECT gen_random_uuid(),CURRENT_TIMESTAMP,kind,name,$2,'Deployment',secretdeliverymode,
                  secretid,targetpath,CURRENT_TIMESTAMP,value
           FROM resourcebindings WHERE scope='Deployment' AND resourceid=$1"#,
    )
    .bind(source_id)
    .bind(destination_id)
    .execute(&mut **tx)
    .await
    .map_err(database_error)?;
    Ok(())
}

async fn ensure_access(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    administrator: bool,
    resource_id: Uuid,
    required: i32,
) -> Result<(), DeploymentError> {
    if administrator
        || has_resource_access(
            tx,
            actor_id,
            DEPLOYMENT_RESOURCE_TYPE,
            resource_id,
            required,
        )
        .await?
    {
        Ok(())
    } else {
        Err(DeploymentError::Forbidden)
    }
}

async fn ensure_specific_access(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    administrator: bool,
    resource_id: Uuid,
    required_level: i32,
    required_specific: i32,
) -> Result<(), DeploymentError> {
    if administrator {
        return Ok(());
    }
    let (level, specific) =
        effective_permission(tx, actor_id, DEPLOYMENT_RESOURCE_TYPE, Some(resource_id)).await?;
    if level >= required_level && specific & required_specific == required_specific {
        Ok(())
    } else {
        Err(DeploymentError::Forbidden)
    }
}

async fn require_duplicate_binding_access(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    administrator: bool,
    source_id: Uuid,
) -> Result<(), DeploymentError> {
    if administrator {
        return Ok(());
    }
    let has_bindings = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM resourcebindings WHERE scope='Deployment' AND resourceid=$1)",
    )
    .bind(source_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;
    if !has_bindings {
        return Ok(());
    }
    let (source_level, source_specific) =
        effective_permission(tx, actor_id, DEPLOYMENT_RESOURCE_TYPE, Some(source_id)).await?;
    let (target_level, target_specific) =
        effective_permission(tx, actor_id, DEPLOYMENT_RESOURCE_TYPE, None).await?;
    if source_level < READ_LEVEL || source_specific & RESOURCE_BINDINGS_PERMISSION == 0 {
        return Err(DeploymentError::Forbidden);
    }
    if target_level < WRITE_LEVEL || target_specific & RESOURCE_BINDINGS_PERMISSION == 0 {
        return Err(DeploymentError::Forbidden);
    }
    Ok(())
}

async fn effective_permission(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    resource_type: i32,
    resource_id: Option<Uuid>,
) -> Result<(i32, i32), DeploymentError> {
    sqlx::query_as::<_, (i32, i32)>(
        r#"WITH actor_scope AS (
               SELECT id actorid FROM actors WHERE id=$1 AND isenabled
               UNION
               SELECT team.actorid FROM actorteammemberships membership
               JOIN teams team ON team.id=membership.teamid
               JOIN actors actor ON actor.id=team.actorid AND actor.isenabled
               WHERE membership.memberactorid=$1), effective AS (
               SELECT permission.permissionlevel, permission.specificpermissions
               FROM actor_scope scope
               JOIN actorroles assignment ON assignment.actorid=scope.actorid
               JOIN permissions permission ON permission.roleid=assignment.roleid
               WHERE permission.resourcetype=$2
               UNION ALL
               SELECT access.permissionlevel, access.specificpermissions
               FROM actor_scope scope
               JOIN resourceaccesses access ON access.actorid=scope.actorid
               WHERE $3::uuid IS NOT NULL AND access.resourcetype=$2 AND access.resourceid=$3)
           SELECT COALESCE(MAX(permissionlevel),0)::integer,
                  COALESCE(bit_or(specificpermissions),0)::integer
           FROM effective"#,
    )
    .bind(actor_id.value())
    .bind(resource_type)
    .bind(resource_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)
}

async fn has_resource_access(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    resource_type: i32,
    resource_id: Uuid,
    required: i32,
) -> Result<bool, DeploymentError> {
    sqlx::query_scalar::<_, bool>(
        r#"WITH actor_scope AS (
               SELECT id actorid FROM actors WHERE id=$1 AND isenabled
               UNION
               SELECT team.actorid FROM actorteammemberships membership
               JOIN teams team ON team.id=membership.teamid
               JOIN actors actor ON actor.id=team.actorid AND actor.isenabled
               WHERE membership.memberactorid=$1)
           SELECT EXISTS(
               SELECT 1 FROM actor_scope scope
               JOIN actorroles assignment ON assignment.actorid=scope.actorid
               JOIN permissions permission ON permission.roleid=assignment.roleid
               WHERE permission.resourcetype=$2 AND permission.permissionlevel >= $4
               UNION ALL
               SELECT 1 FROM actor_scope scope
               JOIN resourceaccesses access ON access.actorid=scope.actorid
               WHERE access.resourcetype=$2 AND access.resourceid=$3 AND access.permissionlevel >= $4)"#,
    )
    .bind(actor_id.value())
    .bind(resource_type)
    .bind(resource_id)
    .bind(required)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)
}

async fn insert_deployment_activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: &str,
    platform_id: Uuid,
    actor_id: ActorId,
    info: ActivityEventInfo,
    now: DateTime<Utc>,
) -> Result<(), DeploymentError> {
    let activity =
        ActivityEvent::new_deployment_event(id, name.to_owned(), platform_id, actor_id, info, now)
            .map_err(|error| {
                DeploymentError::Storage(format!("invalid Deployment activity: {error}"))
            })?;
    insert_activity(tx, &activity)
        .await
        .map_err(|error| DeploymentError::Storage(error.to_string()))
}

async fn lock_apply_claim(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    claim: &ApplyClaim,
) -> Result<(), DeploymentError> {
    let found = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM deployments WHERE id=$1 AND rowversion=$2 AND controlstate='Processing' AND controltriggeredby=$3 FOR UPDATE",
    )
    .bind(claim.id)
    .bind(claim.row_version)
    .bind(actor_id.value())
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    if found.is_some() {
        Ok(())
    } else {
        Err(DeploymentError::Conflict(
            "The Deployment Apply claim is no longer current.".to_owned(),
        ))
    }
}

async fn upsert_apply_container(
    tx: &mut Transaction<'_, Postgres>,
    claim: &ApplyClaim,
    result: &RuntimeDeploymentResult,
) -> Result<Uuid, DeploymentError> {
    let now = Utc::now().timestamp();
    let state = match result.state {
        RuntimeContainerState::Running => "Running",
        RuntimeContainerState::Exited => "Exited",
        RuntimeContainerState::Timeout => "Unknown",
    };
    if let Some(id) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM containers WHERE dockercontainerid=$1 AND platformid=$2 AND dockernodeid IS NULL FOR UPDATE",
    )
    .bind(&result.docker_container_id)
    .bind(claim.platform_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    {
        sqlx::query(
            "UPDATE containers SET deploymentid=$2,dockerimageid=$3,hascitadelownershiplabels=TRUE,name=$4,state=$5,updated=$6,rowversion=rowversion+1 WHERE id=$1",
        )
        .bind(id)
        .bind(claim.id)
        .bind(&result.docker_image_id)
        .bind(&claim.name)
        .bind(state)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
        return Ok(id);
    }
    if let Some(id) = claim.existing_container_id {
        let affected = sqlx::query(
            r#"UPDATE containers SET dockercontainerid=$2,dockerimageid=$3,
                      hascitadelownershiplabels=TRUE,name=$4,state=$5,updated=$6,
                      deploymentid=$7,rowversion=rowversion+1
               WHERE id=$1"#,
        )
        .bind(id)
        .bind(&result.docker_container_id)
        .bind(&result.docker_image_id)
        .bind(&claim.name)
        .bind(state)
        .bind(now)
        .bind(claim.id)
        .execute(&mut **tx)
        .await
        .map_err(database_error)?
        .rows_affected();
        if affected == 1 {
            return Ok(id);
        }
    }
    let id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO containers(
               id,created,deploymentid,dockercontainerid,dockerimageid,hascitadelownershiplabels,
               isswarmtask,issystem,name,platformid,ports,rowversion,state,updated)
           VALUES($1,$2,$3,$4,$5,TRUE,FALSE,FALSE,$6,$7,'{}'::json,0,$8,$2)
           ON CONFLICT (dockercontainerid,platformid) WHERE dockernodeid IS NULL
           DO UPDATE SET deploymentid=EXCLUDED.deploymentid,dockerimageid=EXCLUDED.dockerimageid,
                         hascitadelownershiplabels=TRUE,name=EXCLUDED.name,state=EXCLUDED.state,
                         updated=EXCLUDED.updated,rowversion=containers.rowversion+1"#,
    )
    .bind(id)
    .bind(now)
    .bind(claim.id)
    .bind(&result.docker_container_id)
    .bind(&result.docker_image_id)
    .bind(&claim.name)
    .bind(claim.platform_id)
    .bind(state)
    .execute(&mut **tx)
    .await
    .map_err(database_error)?;
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM containers WHERE dockercontainerid=$1 AND platformid=$2 AND dockernodeid IS NULL",
    )
    .bind(&result.docker_container_id)
    .bind(claim.platform_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)
}

async fn insert_apply_activity(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    claim: &ApplyClaim,
    status: ActivityStatus,
    result: DeploymentResultActivitySnapshot,
    applied_spec: Option<&DeploymentSpec>,
) -> Result<(), DeploymentError> {
    let info = ActivityEventInfo::deployment_applied(
        Some(snapshot(
            claim.id,
            &claim.name,
            claim.platform_id,
            claim.description.clone(),
            applied_spec.unwrap_or(&claim.spec).to_storage_value()?,
        )),
        result,
    );
    let activity = ActivityEvent::new_deployment_result_event(
        claim.id,
        claim.name.clone(),
        claim.platform_id,
        actor_id,
        info,
        status,
        Utc::now(),
    )
    .map_err(|error| DeploymentError::Storage(format!("invalid Deployment activity: {error}")))?;
    insert_activity(tx, &activity)
        .await
        .map_err(|error| DeploymentError::Storage(error.to_string()))
}

fn apply_result(
    container_ids: Option<Vec<String>>,
    message: Option<String>,
    bindings: &[DeploymentBindingSnapshot],
) -> Result<DeploymentResultActivitySnapshot, DeploymentError> {
    Ok(DeploymentResultActivitySnapshot {
        container_ids,
        message,
        resource_bindings: if bindings.is_empty() {
            None
        } else {
            Some(serde_json::to_value(bindings).map_err(storage)?)
        },
    })
}

fn snapshot(
    id: Uuid,
    name: &str,
    platform_id: Uuid,
    description: Option<String>,
    spec: Value,
) -> DeploymentActivitySnapshot {
    DeploymentActivitySnapshot {
        id,
        name: name.to_owned(),
        platform_id,
        description,
        spec,
    }
}

fn map_deployment(row: PgRow) -> Result<DeploymentView, DeploymentError> {
    let spec = DeploymentSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
    let tags = serde_json::from_value::<Vec<TagSummary>>(row.try_get("tags").map_err(storage)?)
        .map_err(|error| DeploymentError::Storage(format!("invalid Deployment Tags: {error}")))?;
    let permission = EffectiveDeploymentPermission {
        level_mask: row.try_get("permission_level").map_err(storage)?,
        specific_mask: row.try_get("permission_specific").map_err(storage)?,
    };
    let auto_status: Option<String> = row.try_get("autoupdatestate_status").map_err(storage)?;
    let auto_last_checked: Option<DateTime<Utc>> = row
        .try_get("autoupdatestate_lastcheckedat")
        .map_err(storage)?;
    let auto_current_digest: Option<String> = row
        .try_get("autoupdatestate_currentdigest")
        .map_err(storage)?;
    let auto_remote_digest: Option<String> = row
        .try_get("autoupdatestate_remotedigest")
        .map_err(storage)?;
    let auto_last_error: Option<String> =
        row.try_get("autoupdatestate_lasterror").map_err(storage)?;
    Ok(DeploymentView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        status: row.try_get("status").map_err(storage)?,
        control_state: row.try_get("controlstate").map_err(storage)?,
        row_version: row.try_get("rowversion").map_err(storage)?,
        auto_update_state: auto_status.map(|status| AutoUpdateState {
            last_checked_at: auto_last_checked.unwrap_or_else(dotnet_min_datetime),
            status,
            current_digest: auto_current_digest,
            remote_digest: auto_remote_digest,
            last_error: auto_last_error,
        }),
        spec,
        platform_status: row.try_get("platform_status").map_err(storage)?,
        platform_name: row.try_get("platform_name").map_err(storage)?,
        image_name: row.try_get("image_name").map_err(storage)?,
        image_id: row.try_get("image_id").map_err(storage)?,
        container_id: row.try_get("container_id").map_err(storage)?,
        docker_container_id: row.try_get("dockercontainerid").map_err(storage)?,
        docker_image_id: row.try_get("dockerimageid").map_err(storage)?,
        tags,
        latest_activity_view: row.try_get("latest_activity").map_err(storage)?,
        capabilities: Some(capabilities(permission)),
    })
}

fn capabilities(permission: EffectiveDeploymentPermission) -> DeploymentCapabilities {
    DeploymentCapabilities {
        can_read: permission.level_mask >= READ_LEVEL,
        can_write: permission.level_mask >= WRITE_LEVEL,
        can_execute: permission.level_mask >= EXECUTE_LEVEL,
        can_view_logs: permission.specific_mask & (1 << 0) != 0,
        can_inspect: permission.specific_mask & (1 << 1) != 0,
        can_apply: permission.specific_mask & (1 << 2) != 0,
        can_pull: permission.specific_mask & (1 << 3) != 0,
        can_open_terminal: permission.specific_mask & (1 << 4) != 0,
        can_view_resource_bindings: permission.specific_mask & (1 << 5) != 0,
    }
}

async fn available_duplicate_name(
    pool: &PgPool,
    source: &str,
    platform_id: Uuid,
) -> Result<String, DeploymentError> {
    for attempt in 1..=100 {
        let suffix = if attempt == 1 {
            "copy".to_owned()
        } else {
            format!("copy-{attempt}")
        };
        let candidate = duplicate_name(source, &suffix);
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM deployments WHERE platformid=$1 AND name=$2)",
        )
        .bind(platform_id)
        .bind(&candidate)
        .fetch_one(pool)
        .await
        .map_err(storage)?;
        if !exists {
            return Ok(candidate);
        }
    }
    Ok(duplicate_name(
        source,
        &Uuid::now_v7().simple().to_string()[..8],
    ))
}

fn duplicate_name(source: &str, suffix: &str) -> String {
    let suffix = format!("-{suffix}");
    let maximum = 64_usize.saturating_sub(suffix.len()).max(1);
    let mut source = source.chars().take(maximum).collect::<String>();
    source = source.trim_end_matches(['-', '_']).to_owned();
    if source.is_empty() {
        source = "resource".to_owned();
    }
    format!("{source}{suffix}")
}

fn has_likely_host_bind(volumes: Option<&[String]>) -> bool {
    volumes.is_some_and(|volumes| {
        volumes.iter().any(|volume| {
            let volume = volume.trim();
            volume.starts_with('/')
                || volume
                    .split_once(':')
                    .is_some_and(|(source, _)| source.starts_with('.') || source.starts_with('~'))
        })
    })
}

fn platform_kind(descriptor: &Value) -> &str {
    descriptor
        .get("$type")
        .or_else(|| descriptor.get("type"))
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn dotnet_min_datetime() -> DateTime<Utc> {
    DateTime::from_timestamp(-62_135_596_800, 0)
        .expect("the .NET DateTime minimum is representable by chrono")
}

fn storage(error: impl std::fmt::Display) -> DeploymentError {
    DeploymentError::Storage(error.to_string())
}

fn ensure_idle(row: &PgRow) -> Result<(), DeploymentError> {
    let state: String = row.try_get("controlstate").map_err(storage)?;
    if state == "Idle" {
        Ok(())
    } else {
        Err(DeploymentError::Conflict(
            "The Deployment has an operation in progress.".to_owned(),
        ))
    }
}

fn database_error(error: sqlx::Error) -> DeploymentError {
    if let sqlx::Error::Database(database) = &error
        && database.code().as_deref() == Some("23505")
    {
        return DeploymentError::Conflict("Name already exists.".to_owned());
    }
    storage(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_names_remain_bounded() {
        let name = duplicate_name(&"x".repeat(80), "copy-100");
        assert!(name.len() <= 64);
        assert!(name.ends_with("-copy-100"));
    }

    #[test]
    fn only_host_bind_mounts_emit_warning() {
        assert!(has_likely_host_bind(Some(&["./data:/data".to_owned()])));
        assert!(has_likely_host_bind(Some(&["/srv/data:/data".to_owned()])));
        assert!(!has_likely_host_bind(Some(&["named:/data".to_owned()])));
    }
}
