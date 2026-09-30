use super::*;
use crate::persistence::postgres::identity::authorization_cache::{Impact, Mutation};
use crate::persistence::postgres::platforms::runtime_index;
use citadel_platforms::jobs::{ProjectionKind, ProjectionWrite};
impl PostgresDeploymentRepository {
    pub(super) fn claim_delete_impl<'a>(
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
                   WHERE d.id=ANY($1::uuid[]) ORDER BY d.id FOR NO KEY UPDATE OF d"#,
            )
            .bind(ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(storage)?;
            if rows.len() != ids.len() {
                return Err(DeploymentError::NotFound);
            }
            for id in ids {
                ensure_access(
                    &mut tx,
                    actor_id,
                    administrator,
                    *id,
                    DeleteDeployment::REQUIREMENT,
                )
                .await?;
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
                        previous_status: row
                            .try_get::<&str, _>("status")
                            .map_err(storage)?
                            .parse()
                            .map_err(storage)?,
                        description: row.try_get("description").map_err(storage)?,
                        spec: DeploymentSpec::from_storage_value(spec)?,
                    })
                })
                .collect::<Result<Vec<_>, DeploymentError>>()?;
            tx.commit().await.map_err(storage)?;
            Ok(claims)
        })
    }

    pub(super) fn complete_delete_impl<'a>(
        &'a self,
        actor_id: ActorId,
        claims: &'a [DeletionClaim],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        Box::pin(async move {
            let platforms: std::collections::BTreeSet<_> =
                claims.iter().map(|claim| claim.platform_id).collect();
            let writes = ProjectionWrite::begin_many(
                platforms
                    .iter()
                    .map(|id| (*id, None, ProjectionKind::Containers)),
            )
            .await;
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Resources(1, claims.iter().map(|claim| claim.id).collect());
            authorization
                .capture(&mut tx, &impact)
                .await
                .map_err(storage)?;
            let ids = claims.iter().map(|claim| claim.id).collect::<Vec<_>>();
            lock_delete_platforms(&mut tx, &ids).await?;
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
            let mut identities = Vec::new();
            for platform in platforms {
                identities.extend(
                    runtime_index::stage_scope(&self.pool, &mut tx, platform, None)
                        .await
                        .map_err(storage)?,
                );
            }
            authorization.commit(tx, impact).await.map_err(storage)?;
            for update in identities {
                update.committed();
            }
            for write in writes {
                write.committed();
            }
            Ok(())
        })
    }

    pub(super) fn release_delete_impl<'a>(
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
                .bind(claim.previous_status.as_str())
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

    pub(super) fn claim_apply_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<ApplyClaim, DeploymentError>> {
        self.claim_apply_versioned(actor_id, administrator, id, None)
    }

    pub(super) fn claim_apply_versioned_impl(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        expected_version: Option<i64>,
    ) -> BoxFuture<'_, Result<ApplyClaim, DeploymentError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_specific_access(
                &mut tx,
                actor_id,
                administrator,
                id,
                ApplyDeployment::REQUIREMENT,
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
                   WHERE d.id=$1 FOR NO KEY UPDATE OF d"#,
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(DeploymentError::NotFound)?;
            if expected_version
                .is_some_and(|version| row.try_get::<i64, _>("rowversion").ok() != Some(version))
            {
                return Err(DeploymentError::Conflict(
                    "The Deployment changed before automatic Apply.".into(),
                ));
            }
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
            sqlx::query("SELECT pg_notify($1, '')")
                .bind(citadel_runtime::RuntimeSignal::DeploymentRecovery.channel())
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(claim)
        })
    }

    pub(super) fn complete_apply_impl<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ApplyClaim,
        result: &'a RuntimeDeploymentResult,
        digest: Option<&'a str>,
        bindings: &'a [DeploymentBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        Box::pin(async move {
            let write =
                ProjectionWrite::begin(claim.platform_id, None, ProjectionKind::Containers).await;
            let mut tx = self.pool.begin().await.map_err(storage)?;
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
            if let Err(error) = lock_apply_claim(&mut tx, actor_id, claim).await {
                if !matches!(error, DeploymentError::Conflict(_)) {
                    return Err(error);
                }
                // A lost commit acknowledgement may cause the same completion
                // to be retried. Accept only this exact completed version/result;
                // a later operation must still reject the stale claim.
                let completed: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM deployments d WHERE d.id=$1 AND d.rowversion=$2 AND d.controlstate='Idle' AND d.status='Healthy' AND d.spec::jsonb=$3::jsonb AND EXISTS(SELECT 1 FROM containers c WHERE c.deploymentid=d.id AND c.dockercontainerid=$4 AND c.dockerimageid=$5))",
                )
                .bind(claim.id)
                .bind(claim.row_version + 1)
                .bind(&spec_value)
                .bind(&result.docker_container_id)
                .bind(&result.docker_image_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(storage)?;
                return if completed { Ok(()) } else { Err(error) };
            }
            let container_id = upsert_apply_container(&mut tx, claim, result).await?;
            let affected = sqlx::query(
                "UPDATE deployments SET status='Healthy',spec=$4,controlstate='Idle',controltriggeredby=NULL,controlstartedat=NULL,rowversion=rowversion+1,autoupdatestate_status=CASE WHEN $5::text IS NOT NULL THEN 'UpToDate' ELSE autoupdatestate_status END,autoupdatestate_currentdigest=COALESCE($5,autoupdatestate_currentdigest),autoupdatestate_remotedigest=COALESCE($5,autoupdatestate_remotedigest),autoupdatestate_lasterror=CASE WHEN $5 IS NOT NULL THEN NULL ELSE autoupdatestate_lasterror END WHERE id=$1 AND rowversion=$2 AND controlstate='Processing' AND controltriggeredby=$3",
            )
            .bind(claim.id)
            .bind(claim.row_version)
            .bind(actor_id.value())
            .bind(&spec_value)
            .bind(digest.filter(|_| matches!(spec.image, DeploymentImageInfo::External { .. })))
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
            let identities =
                runtime_index::stage_scope(&self.pool, &mut tx, claim.platform_id, None)
                    .await
                    .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            if let Some(identities) = identities {
                identities.committed();
            }
            write.committed();
            Ok(())
        })
    }

    pub(super) fn fail_apply_impl<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ApplyClaim,
        message: &'a str,
        result: Option<&'a RuntimeDeploymentResult>,
        bindings: &'a [DeploymentBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        Box::pin(async move {
            let write = if result.is_some() {
                Some(
                    ProjectionWrite::begin(claim.platform_id, None, ProjectionKind::Containers)
                        .await,
                )
            } else {
                None
            };
            let mut tx = self.pool.begin().await.map_err(storage)?;
            lock_apply_claim(&mut tx, actor_id, claim).await?;
            if let Some(result) = result {
                upsert_apply_container(&mut tx, claim, result).await?;
            }
            let affected = sqlx::query(
                "UPDATE deployments SET status='Failed',controlstate='Idle',controltriggeredby=NULL,controlstartedat=NULL,rowversion=rowversion+1,autoupdatestate_status=CASE WHEN $4 THEN 'Failed' ELSE autoupdatestate_status END,autoupdatestate_lasterror=CASE WHEN $4 THEN 'Deployment Apply failed. Review its activity for details.' ELSE autoupdatestate_lasterror END WHERE id=$1 AND rowversion=$2 AND controlstate='Processing' AND controltriggeredby=$3",
            )
            .bind(claim.id)
            .bind(claim.row_version)
            .bind(actor_id.value())
            .bind(claim.spec.update_behavior == citadel_deployments::UpdateBehavior::AutoDeploy)
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
            let identities = if write.is_some() {
                runtime_index::stage_scope(&self.pool, &mut tx, claim.platform_id, None)
                    .await
                    .map_err(storage)?
            } else {
                None
            };
            tx.commit().await.map_err(storage)?;
            if let Some(identities) = identities {
                identities.committed();
            }
            if let Some(write) = write {
                write.committed();
            }
            Ok(())
        })
    }

    pub(super) fn stale_apply_claims_impl<'a>(
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
                     AND d.containeroperationid IS NULL AND d.updatecheckid IS NULL AND d.controlstartedat IS NOT NULL AND d.controlstartedat < $1
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
pub(super) async fn lock_apply_claim(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    claim: &ApplyClaim,
) -> Result<(), DeploymentError> {
    let found = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM deployments WHERE id=$1 AND rowversion=$2 AND controlstate='Processing' AND controltriggeredby=$3 FOR NO KEY UPDATE",
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

pub(super) async fn upsert_apply_container(
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
            "UPDATE containers SET deploymentid=$2,dockerimageid=$3,hascitadelownershiplabels=TRUE,name=$4,state=$5,updated=$6,projectionobservedat=GREATEST(COALESCE(projectionobservedat,0),$6),rowversion=rowversion+1 WHERE id=$1",
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
                      deploymentid=$7,projectionobservedat=GREATEST(COALESCE(projectionobservedat,0),$6),rowversion=rowversion+1
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
               isswarmtask,issystem,name,platformid,ports,rowversion,state,updated,projectionobservedat)
           VALUES($1,$2,$3,$4,$5,TRUE,FALSE,FALSE,$6,$7,'{}'::json,0,$8,$2,$2)
           ON CONFLICT (dockercontainerid,platformid) WHERE dockernodeid IS NULL
           DO UPDATE SET deploymentid=EXCLUDED.deploymentid,dockerimageid=EXCLUDED.dockerimageid,
                         hascitadelownershiplabels=TRUE,name=EXCLUDED.name,state=EXCLUDED.state,
                         updated=EXCLUDED.updated,projectionobservedat=GREATEST(COALESCE(containers.projectionobservedat,0),EXCLUDED.projectionobservedat),rowversion=containers.rowversion+1"#,
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
