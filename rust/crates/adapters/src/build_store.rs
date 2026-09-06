use chrono::{DateTime, Utc};
use citadel_builds::{
    BuildAgentPoolInput, BuildAgentPoolView, BuildClaim, BuildError, BuildExecutionResult,
    BuildProjectInput, BuildProjectView, BuildRunView, BuildStore,
};
use citadel_domain::{ActorId, ResourceType};
use futures_util::future::BoxFuture;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::resource_tags;

const READ_MASK: i32 = 1 | 2 | 4;
const AUTHORIZED_CTE: &str = r#"
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
), global_access AS (
    SELECT EXISTS (
        SELECT 1 FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid = scope.actorid
        JOIN permissions permission ON permission.roleid = assignment.roleid
        WHERE permission.resourcetype = $2
          AND (permission.permissionlevel & $3) <> 0
    ) AS allowed
)
"#;

#[derive(Clone)]
pub struct PostgresBuildStore {
    pool: PgPool,
}

impl PostgresBuildStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl BuildStore for PostgresBuildStore {
    fn create_pool<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildAgentPoolInput,
    ) -> BoxFuture<'a, Result<BuildAgentPoolView, BuildError>> {
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
            Ok(pool)
        })
    }
    fn list_pools(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildAgentPoolView>, BuildError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT pool.* FROM buildagentpools pool
WHERE pool.archivedat IS NULL
  AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=pool.id
        AND (access.permissionlevel & $3) <> 0
  ))
ORDER BY pool.name,pool.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BuildAgentPool as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_pool)
                .collect()
        })
    }
    fn pool_permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<std::collections::BTreeMap<Uuid, i32>, BuildError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT ids.id, COALESCE(bit_or(grants.level),0)::int4
FROM unnest($4::uuid[]) ids(id)
LEFT JOIN LATERAL (
    SELECT p.permissionlevel AS level FROM actor_scope scope
    JOIN actorroles assignment ON assignment.actorid=scope.actorid
    JOIN permissions p ON p.roleid=assignment.roleid AND p.resourcetype=$2
    UNION ALL
    SELECT access.permissionlevel FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=$2 AND access.resourceid=ids.id
) grants ON true GROUP BY ids.id"#
            );
            let rows: Vec<(Uuid, i32)> = sqlx::query_as(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BuildAgentPool as i32)
                .bind(READ_MASK)
                .bind(ids)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            Ok(rows.into_iter().collect())
        })
    }
    fn get_pool<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildAgentPoolView, BuildError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM buildagentpools WHERE id=$1 AND archivedat IS NULL")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(BuildError::NotFound)
                .and_then(map_pool)
        })
    }
    fn claim_pool_test(
        &self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'_, Result<BuildAgentPoolView, BuildError>> {
        Box::pin(async move {
            // Reclaim a crashed check only after its bounded 120-second deadline.
            sqlx::query("UPDATE buildagentpools SET controlstate='Processing',controltriggeredby=$2,controlstartedat=EXTRACT(EPOCH FROM now())::bigint,updatedat=now(),rowversion=rowversion+1 WHERE id=$1 AND archivedat IS NULL AND (controlstate='Idle' OR controlstartedat < EXTRACT(EPOCH FROM now())::bigint-180) RETURNING *")
                .bind(id).bind(actor.value()).fetch_optional(&self.pool).await.map_err(storage)?
                .ok_or_else(|| BuildError::Conflict("A Build Pool operation is already running or the pool is archived.".into())).and_then(map_pool)
        })
    }
    fn finish_pool_test<'a>(
        &'a self,
        claim: &'a BuildAgentPoolView,
        result: &'a citadel_builds::BuildPoolCheck,
    ) -> BoxFuture<'a, Result<BuildAgentPoolView, BuildError>> {
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
            Ok(pool)
        })
    }
    fn update_pool<'a>(
        &'a self,
        current: &'a BuildAgentPoolView,
        input: &'a BuildAgentPoolInput,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<BuildAgentPoolView, BuildError>> {
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
            Ok(pool)
        })
    }
    fn archive_pool<'a>(
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
    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildProjectInput,
    ) -> BoxFuture<'a, Result<BuildProjectView, BuildError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            sqlx::query("INSERT INTO buildprojects(id,name,normalizedname,description,enabled,gitrepositoryid,branch,contextpath,dockerfilepath,target,buildargs,buildsecrets,builderkind,platformid,buildagentpoolid,registryid,imagerepository,tagtemplates,webhook,timeoutseconds,retentionruncount,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22)")
                .bind(id).bind(&input.name).bind(input.name.to_uppercase()).bind(input.description.as_deref()).bind(input.enabled)
                .bind(input.git_repository_id).bind(input.branch.as_deref().unwrap_or("main")).bind(input.context_path.as_deref().unwrap_or("."))
                .bind(input.dockerfile_path.as_deref().unwrap_or("Dockerfile")).bind(input.target.as_deref())
                .bind(serde_json::to_value(input.build_args.as_deref().unwrap_or(&[])).map_err(storage)?)
                .bind(serde_json::to_value(input.build_secrets.as_deref().unwrap_or(&[])).map_err(storage)?)
                .bind(&input.builder_kind).bind(input.platform_id).bind(input.build_agent_pool_id).bind(input.registry_id)
                .bind(&input.image_repository).bind(serde_json::to_value(input.tag_templates.as_deref().unwrap_or(&[])).map_err(storage)?)
                .bind(input.webhook.as_ref()).bind(input.timeout_seconds.unwrap_or(1800)).bind(input.retention_run_count.unwrap_or(20)).bind(actor.value())
                .execute(&mut *transaction).await.map_err(database)?;
            resource_tags::insert(&mut transaction, "Build", id, &input.tag_ids, actor.value())
                .await
                .map_err(build_tag_error)?;
            transaction.commit().await.map_err(storage)?;
            self.get(id).await
        })
    }

    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildProjectView>, BuildError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT project.* FROM buildprojects project
WHERE project.archivedat IS NULL
  AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=project.id
        AND (access.permissionlevel & $3) <> 0
  ))
ORDER BY project.name,project.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Build as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_project)
                .collect()
        })
    }

    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildProjectView, BuildError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM buildprojects WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(BuildError::NotFound)
                .and_then(map_project)
        })
    }

    fn archive<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            let affected = sqlx::query("UPDATE buildprojects SET enabled=false,archivedat=CURRENT_TIMESTAMP,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 AND archivedat IS NULL AND controlstate='Idle'")
                .bind(id).execute(&self.pool).await.map_err(storage)?.rows_affected();
            if affected == 1 {
                Ok(())
            } else if exists(&self.pool, "buildprojects", id).await? {
                Err(BuildError::Conflict(
                    "Build Project has an active Run or is already archived.".to_owned(),
                ))
            } else {
                Err(BuildError::NotFound)
            }
        })
    }

    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
    ) -> BoxFuture<'a, Result<BuildRunView, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let project = sqlx::query("SELECT project.*,repo.name AS gitrepositoryname,platform.name AS platformname,platform.address AS platformaddress,registry.name AS registryname,registry.registryhost AS registryhost FROM buildprojects project JOIN gitrepositories repo ON repo.id=project.gitrepositoryid LEFT JOIN platforms platform ON platform.id=project.platformid JOIN registries registry ON registry.id=project.registryid WHERE project.id=$1 AND project.archivedat IS NULL FOR UPDATE OF project")
                .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(BuildError::NotFound)?;
            if !project.try_get::<bool, _>("enabled").map_err(storage)? {
                return Err(BuildError::Conflict(
                    "Build Project is disabled.".to_owned(),
                ));
            }
            if project
                .try_get::<String, _>("controlstate")
                .map_err(storage)?
                != "Idle"
            {
                return Err(BuildError::Conflict(
                    "Another Build Run is already active.".to_owned(),
                ));
            }
            let run_id = Uuid::now_v7();
            let platform = serde_json::json!({"id":project.try_get::<Option<Uuid>,_>("platformid").map_err(storage)?,"name":project.try_get::<Option<String>,_>("platformname").map_err(storage)?,"address":project.try_get::<Option<String>,_>("platformaddress").map_err(storage)?});
            let registry = serde_json::json!({"id":project.try_get::<Uuid,_>("registryid").map_err(storage)?,"name":project.try_get::<String,_>("registryname").map_err(storage)?,"registryHost":project.try_get::<String,_>("registryhost").map_err(storage)?});
            let build_secrets: serde_json::Value =
                project.try_get("buildsecrets").map_err(storage)?;
            let secret_ids = build_secrets
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.get("secretId").and_then(|id| id.as_str()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            sqlx::query("INSERT INTO buildruns(id,buildprojectid,projectnamesnapshot,gitrepositoryid,gitrepositorynamesnapshot,branch,resolvedcommitsha,contextpath,dockerfilepath,target,buildargssnapshot,buildsecretidssnapshot,platformsnapshot,registrysnapshot,imagerepository,tagtemplatessnapshot,imagereferences,trigger,triggersourceid,status,timeoutseconds,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,NULL,$7,$8,$9,$10,$11,$12,$13,$14,$15,'[]'::jsonb,$16,NULL,'Queued',$17,$18)")
                .bind(run_id).bind(id).bind(project.try_get::<String,_>("name").map_err(storage)?).bind(project.try_get::<Uuid,_>("gitrepositoryid").map_err(storage)?)
                .bind(project.try_get::<String,_>("gitrepositoryname").map_err(storage)?).bind(project.try_get::<String,_>("branch").map_err(storage)?)
                .bind(project.try_get::<String,_>("contextpath").map_err(storage)?).bind(project.try_get::<String,_>("dockerfilepath").map_err(storage)?)
                .bind(project.try_get::<Option<String>,_>("target").map_err(storage)?).bind(project.try_get::<serde_json::Value,_>("buildargs").map_err(storage)?)
                .bind(serde_json::to_value(secret_ids).map_err(storage)?).bind(platform).bind(registry).bind(project.try_get::<String,_>("imagerepository").map_err(storage)?)
                .bind(project.try_get::<serde_json::Value,_>("tagtemplates").map_err(storage)?).bind(trigger).bind(project.try_get::<i32,_>("timeoutseconds").map_err(storage)?).bind(actor.value())
                .execute(&mut *tx).await.map_err(database)?;
            sqlx::query("UPDATE buildprojects SET controlstate='Queued',currentrunid=$2,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1").bind(id).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            get_run(&self.pool, run_id).await
        })
    }

    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<BuildClaim>, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            recover(&mut tx, stale_before).await?;
            // Serialize only the short claim transaction, not Build execution.
            // Otherwise concurrent workers can both observe a free pool slot.
            let owns_claim = sqlx::query_scalar::<_, bool>(
                "SELECT pg_try_advisory_xact_lock(hashtext('citadel-build-claim'))",
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
            if !owns_claim {
                return Ok(None);
            }
            let Some(row) = sqlx::query("SELECT run.id,run.buildprojectid FROM buildruns run JOIN buildprojects project ON project.id=run.buildprojectid WHERE run.status='Queued' AND project.currentrunid=run.id AND (project.buildagentpoolid IS NULL OR (SELECT count(*) FROM buildruns active JOIN buildprojects other ON other.id=active.buildprojectid WHERE other.buildagentpoolid=project.buildagentpoolid AND active.status IN ('Preparing','Running')) < COALESCE((SELECT maxactivebuilders FROM buildagentpools WHERE id=project.buildagentpoolid),1)) ORDER BY run.queuedat,run.id FOR UPDATE OF run,project SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await.map_err(storage)? else { tx.commit().await.map_err(storage)?; return Ok(None) };
            let run_id: Uuid = row.try_get("id").map_err(storage)?;
            let project_id: Uuid = row.try_get("buildprojectid").map_err(storage)?;
            sqlx::query("UPDATE buildruns SET status='Preparing',startedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Queued'").bind(run_id).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("UPDATE buildprojects SET controlstate='Processing',controlstartedat=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(project_id).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(Some(BuildClaim {
                project: self.get(project_id).await?,
                run: get_run(&self.pool, run_id).await?,
            }))
        })
    }

    fn finish<'a>(
        &'a self,
        claim: &'a BuildClaim,
        result: &'a BuildExecutionResult,
    ) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let affected = sqlx::query("UPDATE buildruns SET status=$2,completedat=CURRENT_TIMESTAMP,exitcode=$3,imagedigest=$4,imagereferences=$5,errorcode=$6,errormessage=$7,resolvedcommitsha=$8 WHERE id=$1 AND status IN ('Preparing','Running')")
                .bind(claim.run.id).bind(result.status).bind(result.exit_code).bind(result.image_digest.as_deref()).bind(serde_json::to_value(&result.image_references).map_err(storage)?).bind(result.error_code.as_deref()).bind(result.error_message.as_deref()).bind(result.resolved_commit_sha.as_deref()).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if affected == 1 {
                for log in &result.logs {
                    sqlx::query("INSERT INTO buildrunlogs(id,buildrunid,createdat,message,stream) VALUES($1,$2,CURRENT_TIMESTAMP,$3,$4)").bind(Uuid::now_v7()).bind(claim.run.id).bind(&log.message).bind(&log.stream).execute(&mut *tx).await.map_err(storage)?;
                }
                sqlx::query("UPDATE buildprojects SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND currentrunid=$2").bind(claim.project.id).bind(claim.run.id).execute(&mut *tx).await.map_err(storage)?;
                sqlx::query(
                    "WITH retained AS (SELECT id FROM buildruns WHERE buildprojectid=$1 AND status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted') ORDER BY completedat DESC NULLS LAST,id DESC LIMIT $2) DELETE FROM buildruns WHERE buildprojectid=$1 AND status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted') AND id NOT IN (SELECT id FROM retained)",
                )
                .bind(claim.project.id)
                .bind(i64::from(claim.project.retention_run_count.max(1)))
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn list_runs<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        project_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<BuildRunView>, BuildError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT run.* FROM buildruns run
WHERE ($4::uuid IS NULL OR run.buildprojectid=$4)
  AND ($5 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=run.buildprojectid
        AND (access.permissionlevel & $3) <> 0
  ))
ORDER BY run.queuedat DESC,run.id DESC LIMIT $6"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Build as i32)
                .bind(READ_MASK)
                .bind(project_id)
                .bind(administrator)
                .bind(i64::try_from(limit.clamp(1, 100)).unwrap_or(100))
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_run)
                .collect()
        })
    }
    fn get_run<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<BuildRunView, BuildError>> {
        Box::pin(async move { get_run(&self.pool, id).await })
    }
    fn logs<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<citadel_builds::BuildLogEntry>, BuildError>> {
        Box::pin(async move {
            if !exists_run(&self.pool, id).await? {
                return Err(BuildError::NotFound);
            }
            sqlx::query(
                "SELECT id,buildrunid,createdat,stream,message FROM buildrunlogs WHERE buildrunid=$1 ORDER BY createdat,id",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(|row| {
                Ok(citadel_builds::BuildLogEntry {
                    id: row.try_get("id").map_err(storage)?,
                    build_run_id: row.try_get("buildrunid").map_err(storage)?,
                    created_at: row.try_get("createdat").map_err(storage)?,
                    stream: row.try_get("stream").map_err(storage)?,
                    message: row.try_get("message").map_err(storage)?,
                })
            })
            .collect()
        })
    }
    fn cancel_queued<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<bool, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row=sqlx::query("UPDATE buildruns SET status='Cancelled',completedat=CURRENT_TIMESTAMP,errorcode='build.cancelled',errormessage='Build run cancelled.' WHERE id=$1 AND status='Queued' RETURNING buildprojectid").bind(id).fetch_optional(&mut *tx).await.map_err(storage)?;
            if let Some(row) = row {
                let project: Uuid = row.try_get("buildprojectid").map_err(storage)?;
                sqlx::query("UPDATE buildprojects SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(project).bind(id).execute(&mut *tx).await.map_err(storage)?;
                tx.commit().await.map_err(storage)?;
                Ok(true)
            } else {
                tx.rollback().await.map_err(storage)?;
                Ok(false)
            }
        })
    }
    fn append_log<'a>(
        &'a self,
        run_id: Uuid,
        log: &'a citadel_builds::BuildLog,
    ) -> BoxFuture<'a, Result<citadel_builds::BuildLogEntry, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let active: Option<Uuid> = sqlx::query_scalar("SELECT id FROM buildruns WHERE id=$1 AND status IN ('Preparing','Running') FOR UPDATE")
                .bind(run_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            if active.is_none() {
                return Err(BuildError::Conflict(
                    "Build Run is no longer active.".into(),
                ));
            }
            let entry = citadel_builds::BuildLogEntry {
                id: Uuid::now_v7(),
                build_run_id: run_id,
                created_at: Utc::now(),
                stream: log.stream.clone(),
                message: log.message.clone(),
            };
            sqlx::query("INSERT INTO buildrunlogs(id,buildrunid,createdat,stream,message) VALUES($1,$2,$3,$4,$5)")
                .bind(entry.id).bind(run_id).bind(entry.created_at).bind(&entry.stream).bind(&entry.message).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(entry)
        })
    }
}

async fn record_pool_activity(
    tx: &mut Transaction<'_, Postgres>,
    pool: &BuildAgentPoolView,
    actor: ActorId,
    info: citadel_domain::ActivityEventInfo,
) -> Result<(), BuildError> {
    let activity = citadel_domain::ActivityEvent::new_build_pool_event(
        pool.id,
        pool.name.clone(),
        actor,
        info,
        Utc::now(),
    )
    .map_err(storage)?;
    crate::activity_store::insert_activity(tx, &activity)
        .await
        .map_err(storage)
}

async fn recover(
    tx: &mut Transaction<'_, Postgres>,
    stale_before: DateTime<Utc>,
) -> Result<(), BuildError> {
    let rows=sqlx::query("UPDATE buildruns SET status='Interrupted',completedat=CURRENT_TIMESTAMP,errorcode='build.interrupted',errormessage='Build interrupted by Core restart.' WHERE status IN ('Preparing','Running') AND startedat<$1 AND startedat+make_interval(secs=>timeoutseconds+300)<CURRENT_TIMESTAMP RETURNING id,buildprojectid").bind(stale_before).fetch_all(&mut **tx).await.map_err(storage)?;
    for row in rows {
        let id: Uuid = row.try_get("id").map_err(storage)?;
        let project: Uuid = row.try_get("buildprojectid").map_err(storage)?;
        sqlx::query("UPDATE buildprojects SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(project).bind(id).execute(&mut **tx).await.map_err(storage)?;
    }
    Ok(())
}

async fn exists(pool: &PgPool, table: &str, id: Uuid) -> Result<bool, BuildError> {
    let query = match table {
        "buildprojects" => "SELECT EXISTS(SELECT 1 FROM buildprojects WHERE id=$1)",
        _ => return Ok(false),
    };
    sqlx::query_scalar(query)
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(storage)
}
async fn exists_run(pool: &PgPool, id: Uuid) -> Result<bool, BuildError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM buildruns WHERE id=$1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(storage)
}
async fn get_run(pool: &PgPool, id: Uuid) -> Result<BuildRunView, BuildError> {
    sqlx::query("SELECT * FROM buildruns WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(BuildError::NotFound)
        .and_then(map_run)
}
fn map_project(row: sqlx::postgres::PgRow) -> Result<BuildProjectView, BuildError> {
    Ok(BuildProjectView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        normalized_name: row.try_get("normalizedname").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        enabled: row.try_get("enabled").map_err(storage)?,
        git_repository_id: row.try_get("gitrepositoryid").map_err(storage)?,
        branch: row.try_get("branch").map_err(storage)?,
        context_path: row.try_get("contextpath").map_err(storage)?,
        dockerfile_path: row.try_get("dockerfilepath").map_err(storage)?,
        target: row.try_get("target").map_err(storage)?,
        build_args: serde_json::from_value(row.try_get("buildargs").map_err(storage)?)
            .map_err(storage)?,
        build_secrets: serde_json::from_value(row.try_get("buildsecrets").map_err(storage)?)
            .map_err(storage)?,
        builder_kind: row.try_get("builderkind").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        build_agent_pool_id: row.try_get("buildagentpoolid").map_err(storage)?,
        registry_id: row.try_get("registryid").map_err(storage)?,
        image_repository: row.try_get("imagerepository").map_err(storage)?,
        tag_templates: serde_json::from_value(row.try_get("tagtemplates").map_err(storage)?)
            .map_err(storage)?,
        webhook: row.try_get("webhook").map_err(storage)?,
        timeout_seconds: row.try_get("timeoutseconds").map_err(storage)?,
        retention_run_count: row.try_get("retentionruncount").map_err(storage)?,
        current_run_id: row.try_get("currentrunid").map_err(storage)?,
        control_state: row.try_get("controlstate").map_err(storage)?,
        control_started_at: row.try_get("controlstartedat").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        updated_at: row.try_get("updatedat").map_err(storage)?,
        archived_at: row.try_get("archivedat").map_err(storage)?,
        row_version: row.try_get("rowversion").map_err(storage)?,
    })
}
fn map_pool(row: sqlx::postgres::PgRow) -> Result<BuildAgentPoolView, BuildError> {
    Ok(BuildAgentPoolView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        normalized_name: row.try_get("normalizedname").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        enabled: row.try_get("enabled").map_err(storage)?,
        provider: row.try_get("provider").map_err(storage)?,
        provider_spec: row.try_get("providerspec").map_err(storage)?,
        max_active_builders: row.try_get("maxactivebuilders").map_err(storage)?,
        queue_timeout_seconds: row.try_get("queuetimeoutseconds").map_err(storage)?,
        provisioning_timeout_seconds: row.try_get("provisioningtimeoutseconds").map_err(storage)?,
        registration_timeout_seconds: row.try_get("registrationtimeoutseconds").map_err(storage)?,
        heartbeat_timeout_seconds: row.try_get("heartbeattimeoutseconds").map_err(storage)?,
        cleanup_timeout_seconds: row.try_get("cleanuptimeoutseconds").map_err(storage)?,
        maximum_instance_lifetime_seconds: row
            .try_get("maximuminstancelifetimeseconds")
            .map_err(storage)?,
        failure_retention_minutes: row.try_get("failureretentionminutes").map_err(storage)?,
        last_validation_status: row.try_get("lastvalidationstatus").map_err(storage)?,
        last_validation_message: row.try_get("lastvalidationmessage").map_err(storage)?,
        last_validated_at: row.try_get("lastvalidatedat").map_err(storage)?,
        control_state: row.try_get("controlstate").map_err(storage)?,
        control_triggered_by: row.try_get("controltriggeredby").map_err(storage)?,
        control_started_at: row.try_get("controlstartedat").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        updated_at: row.try_get("updatedat").map_err(storage)?,
        archived_at: row.try_get("archivedat").map_err(storage)?,
        row_version: row.try_get("rowversion").map_err(storage)?,
    })
}
fn map_run(row: sqlx::postgres::PgRow) -> Result<BuildRunView, BuildError> {
    let registry_snapshot: serde_json::Value = row.try_get("registrysnapshot").map_err(storage)?;
    let registry_id = registry_snapshot
        .get("id")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| BuildError::Storage("Build Run Registry snapshot is invalid.".to_owned()))?;
    let registry_host = registry_snapshot
        .get("registryHost")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| BuildError::Storage("Build Run Registry host is missing.".to_owned()))?
        .to_owned();
    Ok(BuildRunView {
        id: row.try_get("id").map_err(storage)?,
        build_project_id: row.try_get("buildprojectid").map_err(storage)?,
        project_name_snapshot: row.try_get("projectnamesnapshot").map_err(storage)?,
        git_repository_id: row.try_get("gitrepositoryid").map_err(storage)?,
        branch: row.try_get("branch").map_err(storage)?,
        resolved_commit_sha: row.try_get("resolvedcommitsha").map_err(storage)?,
        context_path: row.try_get("contextpath").map_err(storage)?,
        dockerfile_path: row.try_get("dockerfilepath").map_err(storage)?,
        target: row.try_get("target").map_err(storage)?,
        registry_id,
        registry_host,
        image_repository: row.try_get("imagerepository").map_err(storage)?,
        image_references: serde_json::from_value(row.try_get("imagereferences").map_err(storage)?)
            .map_err(storage)?,
        trigger: row.try_get("trigger").map_err(storage)?,
        status: row.try_get("status").map_err(storage)?,
        image_digest: row.try_get("imagedigest").map_err(storage)?,
        timeout_seconds: row.try_get("timeoutseconds").map_err(storage)?,
        queued_at: row.try_get("queuedat").map_err(storage)?,
        started_at: row.try_get("startedat").map_err(storage)?,
        completed_at: row.try_get("completedat").map_err(storage)?,
        exit_code: row.try_get("exitcode").map_err(storage)?,
        error_code: row.try_get("errorcode").map_err(storage)?,
        error_message: row.try_get("errormessage").map_err(storage)?,
        triggered_by_actor_id: row.try_get("triggeredbyactorid").map_err(storage)?,
    })
}
fn storage(error: impl std::fmt::Display) -> BuildError {
    BuildError::Storage(error.to_string())
}
fn build_tag_error(error: resource_tags::ResourceTagError) -> BuildError {
    match error {
        resource_tags::ResourceTagError::Missing => {
            BuildError::Validation("One or more selected Tags do not exist.".to_owned())
        }
        resource_tags::ResourceTagError::Database(error) => database(error),
    }
}
fn database(error: sqlx::Error) -> BuildError {
    if error
        .as_database_error()
        .is_some_and(|value| value.is_unique_violation())
    {
        BuildError::Conflict("Build Project already exists or has an active Run.".to_owned())
    } else {
        storage(error)
    }
}
