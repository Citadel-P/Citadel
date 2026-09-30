use super::*;
use citadel_primitives::AuthorizedResource;
impl PostgresSwarmServiceRepository {
    pub(super) fn update_check_candidates_impl(
        &self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, SwarmServiceError>> {
        Box::pin(async move {
            sqlx::query_scalar("SELECT s.id FROM swarmservices s JOIN platforms p ON p.id=s.platformid WHERE ($1::uuid IS NULL OR s.id>$1) AND s.controlstate='Idle' AND s.spec->>'UpdateBehavior' IN ('Notify','AutoDeploy') AND s.spec->'Image'->>'$type'='External' AND s.spec->'Image'->>'ImageTag' NOT LIKE '%@%' AND COALESCE(s.appliedimagedigest,'')<>'' AND p.status='Online' AND p.platformdescriptor->>'$type'='DockerSwarm' ORDER BY s.id LIMIT $2")
                .bind(after).bind(i64::try_from(limit.clamp(1,100)).unwrap_or(100)).fetch_all(&self.pool).await.map_err(storage)
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn begin_update_check_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a citadel_swarm_services::SwarmService,
    ) -> BoxFuture<'a, Result<citadel_swarm_services::ServiceUpdateCheck, SwarmServiceError>> {
        Box::pin(self.claim_image_check(actor, administrator, expected))
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn complete_update_check_impl<'a>(
        &'a self,
        claim: &'a citadel_swarm_services::ServiceUpdateCheck,
        state: Option<&'a AutoUpdateState>,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(self.finish_image_check(claim, state))
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn recover_update_checks_impl(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, SwarmServiceError>> {
        Box::pin(self.recover_image_checks(started_before, limit))
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn create_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a CreateSwarmService,
    ) -> BoxFuture<
        'a,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_platform(&mut tx, actor_id, administrator, input.platform_id, false).await?;
            validate_references(&mut tx, actor_id, administrator, input).await?;
            validate_tags(&mut tx, &input.tag_ids).await?;
            let duplicate = if let Some(source) = &input.duplicate_source {
                ensure_access(
                    &mut tx,
                    actor_id,
                    administrator,
                    source.resource_id,
                    policy::ReadSwarmService::REQUIREMENT,
                )
                .await?;
                let name: String =
                    sqlx::query_scalar("SELECT name FROM swarmservices WHERE id=$1 FOR SHARE")
                        .bind(source.resource_id)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(storage)?
                        .ok_or(SwarmServiceError::NotFound)?;
                let has_bindings: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM resourcebindings WHERE scope='SwarmService' AND resourceid=$1)")
                    .bind(source.resource_id).fetch_one(&mut *tx).await.map_err(storage)?;
                if has_bindings && !administrator {
                    ensure_access(
                        &mut tx,
                        actor_id,
                        false,
                        source.resource_id,
                        policy::ReadSwarmServiceBindings::REQUIREMENT,
                    )
                    .await?;
                    if !has_access(
                        &mut tx,
                        actor_id,
                        Uuid::nil(),
                        policy::WriteSwarmServiceBindings::REQUIREMENT,
                    )
                    .await?
                    {
                        return Err(SwarmServiceError::Forbidden);
                    }
                }
                Some(citadel_activities::ActivitySourceResource {
                    resource_type: citadel_activities::ActivityResourceType::SwarmService,
                    resource_id: source.resource_id,
                    resource_name: name,
                })
            } else {
                None
            };
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
            if let Some(source) = &duplicate {
                sqlx::query("INSERT INTO resourcebindings(id,createdat,kind,name,resourceid,scope,secretdeliverymode,secretid,targetpath,updatedat,value) SELECT gen_random_uuid(),CURRENT_TIMESTAMP,kind,name,$2,'SwarmService',secretdeliverymode,secretid,targetpath,CURRENT_TIMESTAMP,value FROM resourcebindings WHERE scope='SwarmService' AND resourceid=$1")
                    .bind(source.resource_id).bind(id).execute(&mut *tx).await.map_err(database_error)?;
            }
            insert_swarm_activity(
                &mut tx,
                id,
                &input.name,
                input.platform_id,
                actor_id,
                service_creation_activity(
                    SwarmServiceActivitySnapshot {
                        id,
                        platform_id: input.platform_id,
                        name: input.name.clone(),
                        description: input.description.clone(),
                        docker_name: docker_name.clone(),
                        docker_service_id: None,
                        spec: input.spec.to_storage_value()?,
                    },
                    duplicate,
                ),
                ActivityStatus::Information,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor_id, administrator, id).await
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn update_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: &'a UpdateSwarmService,
    ) -> BoxFuture<
        'a,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor_id,
                administrator,
                id,
                policy::WriteSwarmService::REQUIREMENT,
            )
            .await?;
            let old_row = sqlx::query("SELECT * FROM swarmservices WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(SwarmServiceError::NotFound)?;
            let platform_id = old_row.try_get("platformid").map_err(storage)?;
            let old_snapshot = activity_snapshot(&old_row)?;
            let synthetic = CreateSwarmService {
                name: String::new(),
                platform_id,
                description: None,
                spec: input.spec.clone(),
                tag_ids: Vec::new(),
                duplicate_source: None,
            };
            validate_references(&mut tx, actor_id, administrator, &synthetic).await?;
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
}
impl PostgresSwarmServiceRepository {
    pub(super) fn update_description_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<
        'a,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor_id,
                administrator,
                id,
                policy::WriteSwarmService::REQUIREMENT,
            )
            .await?;
            let row = sqlx::query("SELECT * FROM swarmservices WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(SwarmServiceError::NotFound)?;
            ensure_idle(&row)?;
            let old = activity_snapshot(&row)?;
            let new = SwarmServiceActivitySnapshot {
                description: description.map(str::to_owned),
                ..old.clone()
            };
            sqlx::query("UPDATE swarmservices SET description=$2,rowversion=rowversion+1,updatedat=$3 WHERE id=$1")
                .bind(id).bind(description).bind(Utc::now()).execute(&mut *tx).await.map_err(storage)?;
            let name = old.name.clone();
            let platform = old.platform_id;
            insert_swarm_activity(
                &mut tx,
                id,
                &name,
                platform,
                actor_id,
                ActivityEventInfo::swarm_service_updated(old, new),
                ActivityStatus::Information,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor_id, administrator, id).await
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn rename_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a RenameSwarmService,
    ) -> BoxFuture<
        'a,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor_id,
                administrator,
                input.id,
                policy::WriteSwarmService::REQUIREMENT,
            )
            .await?;
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
}

pub(super) async fn validate_references(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    input: &CreateSwarmService,
) -> Result<(), SwarmServiceError> {
    ensure_platform(tx, actor, administrator, input.platform_id, false).await?;
    match &input.spec.image {
        citadel_swarm_services::SwarmServiceImageInfo::External { registry_id, .. } => {
            ensure_exists(tx, "registries", *registry_id, "Registry").await?;
            if !administrator
                && !has_access(
                    tx,
                    actor,
                    *registry_id,
                    PermissionRequirement {
                        resource_type: ResourceType::Registry,
                        level: PermissionLevel::Read,
                        specific: None,
                    },
                )
                .await?
            {
                return Err(SwarmServiceError::NotFound);
            }
        }
        citadel_swarm_services::SwarmServiceImageInfo::Build {
            build_project_id, ..
        } => {
            ensure_exists(tx, "buildprojects", *build_project_id, "Build Project").await?;
            if !administrator
                && !has_access(
                    tx,
                    actor,
                    *build_project_id,
                    PermissionRequirement {
                        resource_type: ResourceType::Build,
                        level: PermissionLevel::Read,
                        specific: None,
                    },
                )
                .await?
            {
                return Err(SwarmServiceError::NotFound);
            }
        }
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
pub(super) async fn ensure_exists(
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
pub(super) async fn validate_tags(
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
pub(super) async fn replace_tags(
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
