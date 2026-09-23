use super::*;
impl PostgresStackRepository {
    pub(super) fn claim_apply_impl(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        rollback_release_id: Option<Uuid>,
        webhook_job_id: Option<Uuid>,
    ) -> BoxFuture<'_, Result<StackOperationClaim, StackError>> {
        self.claim_apply_versioned(
            actor,
            administrator,
            id,
            rollback_release_id,
            webhook_job_id,
            Default::default(),
        )
    }
}
impl PostgresStackRepository {
    pub(super) fn claim_apply_versioned_impl(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        rollback_release_id: Option<Uuid>,
        webhook_job_id: Option<Uuid>,
        mut options: citadel_stacks::StackApplyOptions,
    ) -> BoxFuture<'_, Result<StackOperationClaim, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor,
                administrator,
                id,
                if rollback_release_id.is_some() {
                    policy::RollbackStack::REQUIREMENT
                } else {
                    policy::ApplyStack::REQUIREMENT
                },
            )
            .await?;
            let row=sqlx::query("SELECT s.*,r.platformid,r.status release_status,r.version,r.spec,p.status platform_status,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1 FOR NO KEY UPDATE OF s,r")
                .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StackError::NotFound)?;
            ensure_idle(&row)?;
            if options
                .expected_version
                .is_some_and(|expected| row.try_get::<i64, _>("rowversion").ok() != Some(expected))
            {
                return Err(StackError::Conflict(
                    "The Stack changed before automatic Apply.".into(),
                ));
            }
            if row
                .try_get::<String, _>("platform_status")
                .map_err(storage)?
                != "Online"
            {
                return Err(StackError::Conflict(
                    "The Stack Platform is offline.".to_owned(),
                ));
            }
            let current_release: Uuid = row.try_get("currentstackreleaseid").map_err(storage)?;
            let mut release_id = current_release;
            let mut spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
            if let Some(job_id) = webhook_job_id {
                if rollback_release_id.is_some() {
                    return Err(StackError::NotFound);
                }
                webhooks::claim(&mut tx, job_id, id, current_release, &spec).await?;
            }
            let mut platform_id: Uuid = row.try_get("platformid").map_err(storage)?;
            let has_snapshot: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stackreleases snapshot JOIN stackreleases current ON current.id=$1 WHERE snapshot.stackid=current.stackid AND snapshot.id<>current.id AND snapshot.version=current.version AND snapshot.status='Healthy')")
                .bind(current_release).fetch_one(&mut *tx).await.map_err(storage)?;
            let status =
                StackReleaseStatus::parse(row.try_get("release_status").map_err(storage)?)?;
            let create_next = has_snapshot
                && !matches!(
                    status,
                    StackReleaseStatus::Created | StackReleaseStatus::Failed
                );
            let operation = if let Some(selected) = rollback_release_id {
                let rollback=sqlx::query("SELECT platformid,spec FROM stackreleases WHERE id=$1 AND stackid=$2 AND status='Healthy'")
                    .bind(selected).bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or_else(|| StackError::Validation("Only a previous healthy Stack release can be rolled back.".to_owned()))?;
                platform_id = rollback.try_get("platformid").map_err(storage)?;
                spec = StackSpec::from_storage_value(rollback.try_get("spec").map_err(storage)?)?;
                "Rollback"
            } else {
                "Apply"
            };
            if rollback_release_id.is_some() || create_next {
                // The preserved snapshot is the deployed definition. The edited
                // row must never become a healthy historical release itself.
                if has_snapshot {
                    sqlx::query("UPDATE stackreleases SET status='Created' WHERE id=$1")
                        .bind(current_release)
                        .execute(&mut *tx)
                        .await
                        .map_err(storage)?;
                }
                let next = next_version(row.try_get("version").map_err(storage)?);
                release_id = Uuid::now_v7();
                sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,status,version,spec,createdat,createdbyactorid) VALUES($1,$2,$3,'Applying',$4,$5,$6,$7)")
                    .bind(release_id).bind(id).bind(platform_id).bind(next).bind(spec.to_storage_value()?).bind(Utc::now()).bind(actor.value())
                    .execute(&mut *tx).await.map_err(storage)?;
                sqlx::query("UPDATE stacks SET currentstackreleaseid=$2 WHERE id=$1")
                    .bind(id)
                    .bind(release_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            } else {
                sqlx::query("UPDATE stackreleases SET status='Applying' WHERE id=$1")
                    .bind(current_release)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            let platform = sqlx::query("SELECT platformdescriptor FROM platforms WHERE id=$1")
                .bind(platform_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(storage)?;
            let descriptor: Value = platform.try_get("platformdescriptor").map_err(storage)?;
            let platform_type =
                crate::persistence::postgres::platforms::classification::platform_kind(
                    descriptor
                        .get("$type")
                        .or_else(|| descriptor.get("type"))
                        .and_then(Value::as_str)
                        .unwrap_or("Docker"),
                )
                .map_err(storage)?;
            let project_name = spec.common().project_name.clone().unwrap_or_else(|| {
                normalize_project_name(
                    row.try_get::<String, _>("name")
                        .unwrap_or_default()
                        .as_str(),
                    id,
                )
            });
            if !options.service_names.is_empty() {
                if platform_type != citadel_platforms::PlatformKind::Docker
                    || rollback_release_id.is_some()
                    || webhook_job_id.is_some()
                    || options.service_names.len() > 256
                {
                    return Err(StackError::Validation("Service-scoped Apply requires a Standalone Stack and at most 256 Services.".into()));
                }
                if let StackSpec::WebEditor { compose_file, .. } = &spec {
                    let model = citadel_stacks::parse_compose(std::slice::from_ref(compose_file))?;
                    if options
                        .service_names
                        .iter()
                        .any(|name| !model.services.iter().any(|service| service.name == *name))
                    {
                        return Err(StackError::Validation(
                            "A selected Service does not exist in the Stack configuration.".into(),
                        ));
                    }
                }
                options.service_names.sort_unstable();
                options.service_names.dedup();
            }
            let now = Utc::now().timestamp();
            sqlx::query("UPDATE stacks SET controlstate='Processing',controlstartedat=$2,controltriggeredby=$3,applyservices=$4,rowversion=rowversion+1 WHERE id=$1")
                .bind(id).bind(now).bind(actor.value()).bind(&options.service_names).execute(&mut *tx).await.map_err(storage)?;
            let claim = StackOperationClaim {
                stack_id: id,
                release_id,
                platform_id,
                name: row.try_get("name").map_err(storage)?,
                project_name,
                platform_type,
                spec,
                row_version: row.try_get::<i64, _>("rowversion").map_err(storage)? + 1,
                actor_id: actor.value(),
                operation: operation.to_owned(),
                service_names: options.service_names,
            };
            tx.commit().await.map_err(storage)?;
            Ok(claim)
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn record_apply_source_impl<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        source: &'a StackReleaseSource,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            let changed=sqlx::query("UPDATE stackreleases r SET source=$4 FROM stacks s WHERE s.id=$1 AND s.currentstackreleaseid=$2 AND s.rowversion=$3 AND s.controlstate='Processing' AND r.id=$2 AND r.stackid=s.id AND r.status='Applying'")
                .bind(claim.stack_id).bind(claim.release_id).bind(claim.row_version).bind(source.to_storage_value()?).execute(&self.pool).await.map_err(storage)?.rows_affected();
            if changed != 1 {
                return Err(StackError::Conflict(
                    "The Stack changed before its source could be recorded.".into(),
                ));
            }
            Ok(())
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn complete_apply_impl<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackOperationClaim,
        result: &'a StackRuntimeResult,
        bindings: &'a [ResourceBindingSnapshot],
        source: Option<&'a StackReleaseSource>,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            let source = source.filter(|_| claim.service_names.is_empty());
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row=sqlx::query("SELECT s.name,s.description,s.stacksource,s.driftpolicy,s.stackupdatestate,r.spec,r.version FROM stacks s JOIN stackreleases r ON r.id=$2 AND r.stackid=s.id WHERE s.id=$1 AND s.currentstackreleaseid=$2 AND s.controlstate='Processing' AND s.rowversion=$3 FOR NO KEY UPDATE OF s,r")
                .bind(claim.stack_id).bind(claim.release_id).bind(claim.row_version).fetch_optional(&mut *tx).await.map_err(storage)?;
            let Some(row) = row else {
                // A committed completion can be retried after its acknowledgement
                // is lost. Accept only the same version and persisted outcome.
                let completed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1 AND r.id=$2 AND s.rowversion=$3 AND s.controlstate='Idle' AND r.status=$4 AND r.spec=$5::jsonb AND r.resourcebindings::jsonb=$6::jsonb AND ($7::jsonb IS NULL OR r.source::jsonb=$7::jsonb))")
                    .bind(claim.stack_id).bind(claim.release_id).bind(claim.row_version + 1)
                    .bind(result.status.as_str()).bind(claim.spec.to_storage_value()?)
                    .bind(ResourceBindingSnapshot::list_to_storage_value(bindings)?)
                    .bind(source.map(StackReleaseSource::to_storage_value).transpose()?)
                    .fetch_one(&mut *tx).await.map_err(storage)?;
                return if completed {
                    Ok(())
                } else {
                    Err(StackError::Conflict(
                        "The Stack operation was superseded.".into(),
                    ))
                };
            };
            let update_state = if let Some(source) =
                source.filter(|source| source.source_type == StackSource::Git)
            {
                let mut state = StackUpdateState::from_storage_value(
                    row.try_get("stackupdatestate").map_err(storage)?,
                )?;
                state
                    .record_applied_commit(&source.resolved_commit_sha, Utc::now())
                    .then(|| state.to_storage_value())
                    .transpose()?
            } else {
                None
            };
            let source = source
                .map(StackReleaseSource::to_storage_value)
                .transpose()?;
            let changed=sqlx::query("UPDATE stackreleases SET status=$3,resourcebindings=$4,source=COALESCE($5::json,source),spec=$6 WHERE id=$1 AND stackid=$2 AND status='Applying'")
                .bind(claim.release_id).bind(claim.stack_id).bind(result.status.as_str()).bind(ResourceBindingSnapshot::list_to_storage_value(bindings)?).bind(source).bind(claim.spec.to_storage_value()?)
                .execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed != 1 {
                return Err(StackError::Conflict(
                    "The Stack operation was superseded.".to_owned(),
                ));
            }
            sqlx::query("UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,applyservices='{}',stackupdatestate=COALESCE($3::json,stackupdatestate),rowversion=rowversion+1 WHERE id=$1 AND currentstackreleaseid=$2 AND controlstate='Processing'")
                .bind(claim.stack_id).bind(claim.release_id).bind(update_state).execute(&mut *tx).await.map_err(storage)?;
            let snapshot = stack_snapshot(
                claim.stack_id,
                row.try_get("name").map_err(storage)?,
                row.try_get("description").map_err(storage)?,
                parse_stack_source(row.try_get("stacksource").map_err(storage)?)?,
                StackDriftPolicy::from_storage_value(row.try_get("driftpolicy").map_err(storage)?)?,
                claim.platform_id,
                &claim.spec,
                actor,
                row.try_get("version").map_err(storage)?,
            );
            let info = if claim.operation == "Rollback" {
                ActivityEventInfo::StackRollback {
                    old_stack: None,
                    new_stack: Some(snapshot),
                    result: StackResultActivitySnapshot {
                        container_ids: None,
                        message: Some("Stack rollback completed.".to_owned()),
                        resource_bindings: serde_json::to_value(bindings).ok(),
                    },
                }
            } else {
                ActivityEventInfo::StackApplied {
                    stack: Some(snapshot),
                    result: StackResultActivitySnapshot {
                        container_ids: None,
                        message: Some("Stack deployment completed.".to_owned()),
                        resource_bindings: serde_json::to_value(bindings).ok(),
                    },
                }
            };
            insert_stack_activity(
                &mut tx,
                claim.stack_id,
                &claim.name,
                claim.platform_id,
                actor,
                info,
                ActivityStatus::Success,
            )
            .await?;
            webhooks::settle(&mut tx, claim, None).await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn fail_apply_impl<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackOperationClaim,
        message: &'a str,
        unknown: bool,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            let status = if unknown { "Applying" } else { "Failed" };
            let control = if unknown { "Processing" } else { "Idle" };
            let mut tx = self.pool.begin().await.map_err(storage)?;
            if sqlx::query_scalar::<_, Uuid>("SELECT id FROM stacks WHERE id=$1 AND currentstackreleaseid=$2 AND rowversion=$3 AND controlstate='Processing' FOR NO KEY UPDATE")
                .bind(claim.stack_id).bind(claim.release_id).bind(claim.row_version).fetch_optional(&mut *tx).await.map_err(storage)?.is_none() {
                return Err(StackError::Conflict("The Stack operation was superseded.".into()));
            }
            let release_changed=sqlx::query("UPDATE stackreleases SET status=$3 WHERE id=$1 AND stackid=$2 AND status='Applying'").bind(claim.release_id).bind(claim.stack_id).bind(status).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            let stack_changed=sqlx::query("UPDATE stacks SET controlstate=$3,controlstartedat=CASE WHEN $4 THEN controlstartedat ELSE NULL END,controltriggeredby=CASE WHEN $4 THEN controltriggeredby ELSE NULL END,applyservices=CASE WHEN $4 THEN applyservices ELSE '{}'::text[] END,rowversion=rowversion+1 WHERE id=$1 AND currentstackreleaseid=$2 AND controlstate='Processing'")
                .bind(claim.stack_id).bind(claim.release_id).bind(control).bind(unknown).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if release_changed != 1 || stack_changed != 1 {
                return Err(StackError::Conflict(
                    "The Stack operation was superseded.".to_owned(),
                ));
            }
            if !unknown {
                webhooks::settle(&mut tx, claim, Some("Stack webhook Apply failed.")).await?;
                let info = ActivityEventInfo::StackApplied {
                    stack: None,
                    result: StackResultActivitySnapshot {
                        container_ids: None,
                        message: Some(message.to_owned()),
                        resource_bindings: None,
                    },
                };
                insert_stack_activity(
                    &mut tx,
                    claim.stack_id,
                    &claim.name,
                    claim.platform_id,
                    actor,
                    info,
                    ActivityStatus::Failure,
                )
                .await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn stale_apply_claims_impl(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackOperationClaim)>, StackError>> {
        Box::pin(async move {
            sqlx::query("SELECT s.id,s.name,s.rowversion,s.applyservices,s.controltriggeredby,s.currentstackreleaseid,r.platformid,r.spec,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.controlstate='Processing' AND s.containeroperationid IS NULL AND s.controlstartedat <= $1 AND r.status='Applying' AND s.controltriggeredby IS NOT NULL ORDER BY s.controlstartedat,s.id LIMIT $2")
                .bind(started_before).bind(limit).fetch_all(&self.pool).await.map_err(storage)?.into_iter().map(|row| {
                    let id:Uuid=row.try_get("id").map_err(storage)?;
                    let spec=StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                    let descriptor:Value=row.try_get("platformdescriptor").map_err(storage)?;
                    let platform_type=crate::persistence::postgres::platforms::classification::platform_kind(descriptor.get("$type").and_then(Value::as_str).unwrap_or("Docker")).map_err(storage)?;
                    let name:String=row.try_get("name").map_err(storage)?;
                    let actor=ActorId::new(row.try_get("controltriggeredby").map_err(storage)?);
                    Ok((actor,StackOperationClaim { stack_id:id,release_id:row.try_get("currentstackreleaseid").map_err(storage)?,platform_id:row.try_get("platformid").map_err(storage)?,project_name:spec.common().project_name.clone().unwrap_or_else(|| normalize_project_name(&name,id)),platform_type,spec,row_version:row.try_get("rowversion").map_err(storage)?,actor_id:actor.value(),name,operation:"Apply".to_owned(),service_names:row.try_get("applyservices").map_err(storage)? }))
                }).collect()
        })
    }
}
