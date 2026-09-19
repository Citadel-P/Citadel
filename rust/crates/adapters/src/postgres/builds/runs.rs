use super::*;

impl PostgresBuildRepository {
    pub(super) fn enqueue_run<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        expected_version: Option<i64>,
        branch: Option<&'a str>,
        commit: Option<&'a str>,
    ) -> BoxFuture<'a, Result<BuildRun, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let project = sqlx::query("SELECT project.*,repo.name AS gitrepositoryname,platform.name AS platformname,platform.address AS platformaddress,registry.name AS registryname,registry.registryhost AS registryhost FROM buildprojects project JOIN gitrepositories repo ON repo.id=project.gitrepositoryid LEFT JOIN platforms platform ON platform.id=project.platformid JOIN registries registry ON registry.id=project.registryid WHERE project.id=$1 AND project.archivedat IS NULL FOR UPDATE OF project")
                .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(BuildError::NotFound)?;
            if expected_version.is_some_and(|version| {
                project.try_get::<i64, _>("rowversion").ok() != Some(version)
            }) {
                return Err(BuildError::Conflict(
                    "Build Project configuration changed.".into(),
                ));
            }
            let branch = branch
                .map(str::to_owned)
                .unwrap_or(project.try_get::<String, _>("branch").map_err(storage)?);
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
            let platform = serde_json::json!({"id":project.try_get::<Option<Uuid>,_>("platformid").map_err(storage)?,"name":project.try_get::<Option<String>,_>("platformname").map_err(storage)?,"address":project.try_get::<Option<String>,_>("platformaddress").map_err(storage)?,"builderKind":project.try_get::<String,_>("builderkind").map_err(storage)?,"buildAgentPoolId":project.try_get::<Option<Uuid>,_>("buildagentpoolid").map_err(storage)?});
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
            sqlx::query("INSERT INTO buildruns(id,buildprojectid,projectnamesnapshot,gitrepositoryid,gitrepositorynamesnapshot,branch,resolvedcommitsha,contextpath,dockerfilepath,target,buildargssnapshot,buildsecretidssnapshot,platformsnapshot,registrysnapshot,imagerepository,tagtemplatessnapshot,imagereferences,trigger,triggersourceid,status,timeoutseconds,triggeredbyactorid) VALUES($1,$2,$3,$4,$5,$6,$19,$7,$8,$9,$10,$11,$12,$13,$14,$15,'[]'::jsonb,$16,CASE WHEN $16='Webhook' THEN $2 ELSE NULL END,'Queued',$17,$18)")
                .bind(run_id).bind(id).bind(project.try_get::<String,_>("name").map_err(storage)?).bind(project.try_get::<Uuid,_>("gitrepositoryid").map_err(storage)?)
                .bind(project.try_get::<String,_>("gitrepositoryname").map_err(storage)?).bind(branch)
                .bind(project.try_get::<String,_>("contextpath").map_err(storage)?).bind(project.try_get::<String,_>("dockerfilepath").map_err(storage)?)
                .bind(project.try_get::<Option<String>,_>("target").map_err(storage)?).bind(project.try_get::<serde_json::Value,_>("buildargs").map_err(storage)?)
                .bind(serde_json::to_value(secret_ids).map_err(storage)?).bind(platform).bind(registry).bind(project.try_get::<String,_>("imagerepository").map_err(storage)?)
                .bind(project.try_get::<serde_json::Value,_>("tagtemplates").map_err(storage)?).bind(trigger).bind(project.try_get::<i32,_>("timeoutseconds").map_err(storage)?).bind(actor.value()).bind(commit)
                .execute(&mut *tx).await.map_err(database)?;
            sqlx::query("SELECT pg_notify('citadel_build_work','')")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            // Preserve BuildKit IDs with their secret references, never secret values.
            sqlx::query("UPDATE buildruns SET buildsecretssnapshot=$2 WHERE id=$1")
                .bind(run_id)
                .bind(build_secrets)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            sqlx::query("UPDATE buildprojects SET controlstate='Queued',currentrunid=$2,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1").bind(id).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
            record_run_activity(&mut tx, run_id).await?;
            let run = sqlx::query("SELECT * FROM buildruns WHERE id=$1")
                .bind(run_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(storage)
                .and_then(map_run)?;
            tx.commit().await.map_err(storage)?;
            Ok(run)
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn enqueue_impl<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
    ) -> BoxFuture<'a, Result<BuildRun, BuildError>> {
        self.enqueue_run(actor, id, trigger, None, None, None)
    }
}

impl PostgresBuildRepository {
    pub(super) fn enqueue_webhook_impl<'a>(
        &'a self,
        current: &'a BuildProject,
        branch: &'a str,
        commit: Option<&'a str>,
    ) -> BoxFuture<'a, Result<BuildRun, BuildError>> {
        self.enqueue_run(
            ActorId::new(Uuid::from_u128(1)),
            current.id,
            "Webhook",
            Some(current.row_version),
            Some(branch),
            commit,
        )
    }
}

impl PostgresBuildRepository {
    pub(super) fn claim_next_impl<'a>(
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
            let Some(row) = sqlx::query("SELECT run.id,run.buildprojectid FROM buildruns run JOIN buildprojects project ON project.id=run.buildprojectid WHERE run.status='Queued' AND project.currentrunid=run.id AND ((run.platformsnapshot->>'buildAgentPoolId')::uuid IS NULL OR (SELECT count(*) FROM buildruns active JOIN buildprojects other ON other.id=active.buildprojectid WHERE (active.platformsnapshot->>'buildAgentPoolId')::uuid=(run.platformsnapshot->>'buildAgentPoolId')::uuid AND active.status IN ('Preparing','Running')) < COALESCE((SELECT maxactivebuilders FROM buildagentpools WHERE id=(run.platformsnapshot->>'buildAgentPoolId')::uuid),1)) ORDER BY run.queuedat,run.id FOR UPDATE OF run,project SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await.map_err(storage)? else { tx.commit().await.map_err(storage)?; return Ok(None) };
            let run_id: Uuid = row.try_get("id").map_err(storage)?;
            let project_id: Uuid = row.try_get("buildprojectid").map_err(storage)?;
            sqlx::query("UPDATE buildruns SET status='Preparing',startedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Queued'").bind(run_id).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("UPDATE buildprojects SET controlstate='Processing',controlstartedat=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::bigint,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(project_id).bind(run_id).execute(&mut *tx).await.map_err(storage)?;
            record_run_activity(&mut tx, run_id).await?;
            let mut project = map_project(
                sqlx::query("SELECT * FROM buildprojects WHERE id=$1")
                    .bind(project_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(storage)?,
            )?;
            let row = sqlx::query("SELECT * FROM buildruns WHERE id=$1")
                .bind(run_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(storage)?;
            let target: serde_json::Value = row.try_get("platformsnapshot").map_err(storage)?;
            project.builder_kind =
                serde_json::from_value(target["builderKind"].clone()).map_err(storage)?;
            project.platform_id = serde_json::from_value(target["id"].clone()).map_err(storage)?;
            project.build_agent_pool_id =
                serde_json::from_value(target["buildAgentPoolId"].clone()).map_err(storage)?;
            project.build_args =
                serde_json::from_value(row.try_get("buildargssnapshot").map_err(storage)?)
                    .map_err(storage)?;
            project.build_secrets =
                serde_json::from_value(row.try_get("buildsecretssnapshot").map_err(storage)?)
                    .map_err(storage)?;
            project.tag_templates =
                serde_json::from_value(row.try_get("tagtemplatessnapshot").map_err(storage)?)
                    .map_err(storage)?;
            let run = map_run(row)?;
            tx.commit().await.map_err(storage)?;
            Ok(Some(BuildClaim { project, run }))
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn finish_impl<'a>(
        &'a self,
        claim: &'a BuildClaim,
        result: &'a BuildExecutionResult,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let affected = sqlx::query("UPDATE buildruns SET status=$2,completedat=CURRENT_TIMESTAMP,exitcode=$3,imagedigest=$4,imagereferences=$5,errorcode=$6,errormessage=$7,resolvedcommitsha=$8 WHERE id=$1 AND status IN ('Preparing','Running') AND EXISTS(SELECT 1 FROM buildprojects project WHERE project.id=buildruns.buildprojectid AND project.currentrunid=buildruns.id)")
                .bind(claim.run.id).bind(result.status).bind(result.exit_code).bind(result.image_digest.as_deref()).bind(serde_json::to_value(&result.image_references).map_err(storage)?).bind(result.error_code.as_deref()).bind(result.error_message.as_deref()).bind(result.resolved_commit_sha.as_deref()).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if affected == 1 {
                if result.status == "Succeeded"
                    && result
                        .image_references
                        .first()
                        .is_some_and(|reference| !reference.trim().is_empty())
                {
                    crate::postgres::builds::completion::enqueue(
                        &mut tx,
                        claim.project.id,
                        claim.run.id,
                    )
                    .await?;
                }
                record_run_activity(&mut tx, claim.run.id).await?;
                for log in &result.logs {
                    sqlx::query("INSERT INTO buildrunlogs(id,buildrunid,createdat,message,stream) VALUES($1,$2,CURRENT_TIMESTAMP,$3,$4)").bind(Uuid::now_v7()).bind(claim.run.id).bind(&log.message).bind(&log.stream).execute(&mut *tx).await.map_err(storage)?;
                }
                sqlx::query("UPDATE buildprojects SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND currentrunid=$2").bind(claim.project.id).bind(claim.run.id).execute(&mut *tx).await.map_err(storage)?;
                sqlx::query(
                    "WITH retained AS (SELECT id FROM buildruns WHERE buildprojectid=$1 AND status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted') ORDER BY completedat DESC NULLS LAST,id DESC LIMIT $2) DELETE FROM buildruns b WHERE buildprojectid=$1 AND status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted') AND id NOT IN (SELECT id FROM retained) AND NOT EXISTS(SELECT 1 FROM buildcompletionqueue q WHERE q.buildrunid=b.id) AND NOT EXISTS(SELECT 1 FROM deployments d WHERE jsonb_path_exists(d.spec::jsonb, '$.** ? (@ == $id)', jsonb_build_object('id',b.id::text))) AND NOT EXISTS(SELECT 1 FROM stackreleases r WHERE jsonb_path_exists(r.spec::jsonb, '$.** ? (@ == $id)', jsonb_build_object('id',b.id::text)))",
                )
                .bind(claim.project.id)
                .bind(i64::from(claim.project.retention_run_count.max(1)))
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(affected == 1)
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn list_runs_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        project_id: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<BuildRun>, BuildError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT run.* FROM buildruns run
WHERE ($4::uuid IS NULL OR run.buildprojectid=$4)
  AND ($5 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=run.buildprojectid
        AND access.permissionlevel = ANY($3)
  ))
ORDER BY run.queuedat DESC,run.id DESC LIMIT $6"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Build as i32)
                .bind(citadel_domain::PermissionLevel::Read.accepted_database_levels())
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
}

impl PostgresBuildRepository {
    pub(super) fn get_run_impl<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<BuildRun, BuildError>> {
        Box::pin(async move { get_run(&self.pool, id).await })
    }
}

impl PostgresBuildRepository {
    pub(super) fn logs_impl<'a>(
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
}

impl PostgresBuildRepository {
    pub(super) fn cancel_queued_impl<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row=sqlx::query("UPDATE buildruns SET status='Cancelled',completedat=CURRENT_TIMESTAMP,errorcode='build.cancelled',errormessage='Build run cancelled.' WHERE id=$1 AND status='Queued' RETURNING buildprojectid").bind(id).fetch_optional(&mut *tx).await.map_err(storage)?;
            if let Some(row) = row {
                let project: Uuid = row.try_get("buildprojectid").map_err(storage)?;
                record_run_activity(&mut tx, id).await?;
                sqlx::query("UPDATE buildprojects SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(project).bind(id).execute(&mut *tx).await.map_err(storage)?;
                tx.commit().await.map_err(storage)?;
                Ok(true)
            } else {
                tx.rollback().await.map_err(storage)?;
                Ok(false)
            }
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn append_log_impl<'a>(
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
