use super::*;

impl PostgresBuildRepository {
    pub(super) fn project_permissions_impl<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>, BuildError>,
    > {
        Box::pin(async move {
            crate::persistence::postgres::permissions::levels_for_resources(
                &self.pool,
                actor,
                ResourceType::Build,
                ids,
            )
            .await
            .map_err(storage)
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn create_impl<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BuildProjectConfiguration,
    ) -> BoxFuture<'a, Result<BuildProject, BuildError>> {
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
            let project = sqlx::query("SELECT * FROM buildprojects WHERE id=$1")
                .bind(id)
                .fetch_one(&mut *transaction)
                .await
                .map_err(storage)
                .and_then(map_project)?;
            record_project_activity(
                &mut transaction,
                &project,
                actor,
                citadel_activities::ActivityEventInfo::BuildCreated {
                    build: project.snapshot(),
                },
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            self.get(id).await
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn list_impl(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BuildProject>, BuildError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT project.* FROM buildprojects project
WHERE project.archivedat IS NULL
  AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=project.id
        AND access.permissionlevel = ANY($3)
  ))
ORDER BY project.name,project.id"#
            );
            let mut values = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Build as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_project)
                .collect::<Result<Vec<_>, _>>()?;
            enrich_projects(
                &mut *self.pool.acquire().await.map_err(storage)?,
                &mut values,
            )
            .await?;
            Ok(values)
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn get_impl<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<BuildProject, BuildError>> {
        Box::pin(async move {
            let mut value =
                sqlx::query("SELECT * FROM buildprojects WHERE id=$1 AND archivedat IS NULL")
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(storage)?
                    .ok_or(BuildError::NotFound)
                    .and_then(map_project)?;
            enrich_projects(
                &mut *self.pool.acquire().await.map_err(storage)?,
                std::slice::from_mut(&mut value),
            )
            .await?;
            Ok(value)
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn update_impl<'a>(
        &'a self,
        current: &'a BuildProject,
        input: &'a BuildProjectConfiguration,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<BuildProject, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query("UPDATE buildprojects SET name=$3,normalizedname=$4,description=$5,enabled=$6,gitrepositoryid=$7,branch=$8,contextpath=$9,dockerfilepath=$10,target=$11,buildargs=$12,buildsecrets=$13,builderkind=$14,platformid=$15,buildagentpoolid=$16,registryid=$17,imagerepository=$18,tagtemplates=$19,webhook=$20,timeoutseconds=$21,retentionruncount=$22,updatedat=now(),rowversion=rowversion+1 WHERE id=$1 AND rowversion=$2 AND archivedat IS NULL AND controlstate='Idle' RETURNING *")
                .bind(current.id).bind(current.row_version).bind(&input.name).bind(input.name.to_uppercase()).bind(&input.description).bind(input.enabled)
                .bind(input.git_repository_id).bind(&input.branch).bind(&input.context_path).bind(&input.dockerfile_path).bind(&input.target)
                .bind(serde_json::to_value(input.build_args.as_deref().unwrap_or(&[])).map_err(storage)?)
                .bind(serde_json::to_value(input.build_secrets.as_deref().unwrap_or(&[])).map_err(storage)?)
                .bind(&input.builder_kind).bind(input.platform_id).bind(input.build_agent_pool_id).bind(input.registry_id).bind(&input.image_repository)
                .bind(serde_json::to_value(input.tag_templates.as_deref().unwrap_or(&[])).map_err(storage)?)
                .bind(&input.webhook).bind(input.timeout_seconds).bind(input.retention_run_count)
                .fetch_optional(&mut *tx).await.map_err(database)?.ok_or_else(|| BuildError::Conflict("Build was changed, archived or is processing. Reload before saving.".into()))?;
            let project = map_project(row)?;
            if !metadata_only {
                let info = if current.name != project.name {
                    citadel_activities::ActivityEventInfo::BuildRenamed {
                        old_name: current.name.clone(),
                        new_name: project.name.clone(),
                    }
                } else {
                    citadel_activities::ActivityEventInfo::BuildUpdated {
                        old_build: current.snapshot(),
                        new_build: project.snapshot(),
                    }
                };
                record_project_activity(&mut tx, &project, actor, info).await?;
            }
            tx.commit().await.map_err(storage)?;
            self.get(project.id).await
        })
    }
}

impl PostgresBuildRepository {
    pub(super) fn archive_impl<'a>(
        &'a self,
        id: Uuid,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let current = sqlx::query(
                "SELECT * FROM buildprojects WHERE id=$1 AND archivedat IS NULL FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(BuildError::NotFound)
            .and_then(map_project)?;
            if current.control_state != "Idle" {
                return Err(BuildError::Conflict("Build has an active Run.".into()));
            }
            sqlx::query("UPDATE buildprojects SET enabled=false,archivedat=now(),updatedat=now(),rowversion=rowversion+1 WHERE id=$1").bind(id).execute(&mut *tx).await.map_err(storage)?;
            record_project_activity(
                &mut tx,
                &current,
                actor,
                citadel_activities::ActivityEventInfo::BuildDeleted {
                    build: current.snapshot(),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
