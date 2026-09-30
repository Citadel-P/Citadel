use super::*;
use citadel_primitives::AuthorizedResource;
impl PostgresDeploymentRepository {
    pub(super) fn create_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a CreateDeployment,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
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
                    ReadDeployment::REQUIREMENT,
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

    pub(super) fn update_config_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        expected_row_version: i64,
        spec: &'a DeploymentSpec,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor_id,
                administrator,
                id,
                WriteDeployment::REQUIREMENT,
            )
            .await?;
            let row = sqlx::query(
                "SELECT name,description,platformid,spec,controlstate,rowversion FROM deployments WHERE id=$1 FOR NO KEY UPDATE",
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

    pub(super) fn update_metadata_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: &'a UpdateDeploymentMetadata,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor_id,
                administrator,
                id,
                WriteDeployment::REQUIREMENT,
            )
            .await?;
            let row = sqlx::query(
                "SELECT name,description,platformid,spec,controlstate FROM deployments WHERE id=$1 FOR NO KEY UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(DeploymentError::NotFound)?;
            ensure_idle(&row)?;
            let old_description: Option<String> = row.try_get("description").map_err(storage)?;
            let next_description = match &input.description {
                PatchField::Missing => old_description.clone(),
                PatchField::Value(value) => Some(value.clone()),
                PatchField::Null => None,
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

    pub(super) fn rename_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        name: &'a str,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor_id,
                administrator,
                id,
                WriteDeployment::REQUIREMENT,
            )
            .await?;
            let row = sqlx::query(
                "SELECT name,platformid,controlstate FROM deployments WHERE id=$1 FOR NO KEY UPDATE",
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
}
// Deletion removes referenced rows, so it must exclude inventory insertions.
// Ordinary Apply/configuration writes retain keys and do not take this lock.
pub(super) async fn lock_delete_platforms(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<(), DeploymentError> {
    sqlx::query("SELECT p.id FROM platforms p WHERE p.id IN (SELECT platformid FROM deployments WHERE id=ANY($1::uuid[])) ORDER BY p.id FOR SHARE OF p")
        .bind(ids).fetch_all(&mut **tx).await.map_err(storage)?;
    Ok(())
}

pub(super) async fn ensure_platform(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    administrator: bool,
    platform_id: Uuid,
) -> Result<(), DeploymentError> {
    let descriptor = sqlx::query_scalar::<_, Value>(
        "SELECT platformdescriptor FROM platforms WHERE id=$1 FOR KEY SHARE",
    )
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
            PermissionLevel::Read,
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

pub(super) async fn validate_image_references(
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

pub(super) async fn validate_tags(
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

pub(super) async fn insert_tags(
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

pub(super) async fn copy_bindings(
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
