use super::*;
impl PostgresStackRepository {
    pub(super) fn update_check_candidates_impl(
        &self,
        after: Uuid,
        images: bool,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, StackError>> {
        Box::pin(update_checks::candidates(&self.pool, after, images, limit))
    }
}
impl PostgresStackRepository {
    pub(super) fn save_update_check_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a StackDetails,
        state: &'a StackUpdateState,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(update_checks::save(
            self,
            actor,
            administrator,
            expected,
            state,
        ))
    }
}
impl PostgresStackRepository {
    pub(super) fn enqueue_webhook_impl<'a>(
        &'a self,
        expected: &'a StackDetails,
        commit: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(self.enqueue_stack_webhook(expected, commit))
    }
}
impl PostgresStackRepository {
    pub(super) fn ready_webhooks_impl(
        &self,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<citadel_stacks::StackWebhookJob>, StackError>> {
        webhooks::ready(&self.pool, limit)
    }
}
impl PostgresStackRepository {
    pub(super) fn discard_webhook_impl(&self, id: Uuid) -> BoxFuture<'_, Result<(), StackError>> {
        Box::pin(async move {
            sqlx::query("DELETE FROM stackwebhookdeployqueue WHERE id=$1 AND status='Queued'")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(storage)?;
            Ok(())
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn record_drift_impl<'a>(
        &'a self,
        expected: &'a StackDetails,
        status: StackReleaseStatus,
        info: ActivityEventInfo,
    ) -> BoxFuture<'a, Result<bool, StackError>> {
        Box::pin(drift::record(&self.pool, expected, status, info))
    }
}
impl PostgresStackRepository {
    pub(super) fn drift_monitor_candidates_impl(
        &self,
        after: Option<Uuid>,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<StackDetails>, StackError>> {
        Box::pin(async move {
            let sql = format!(
                r#"{AUTHORIZED_CTES}{PROJECTION}
WHERE s.controlstate='Idle'
  AND r.status IN ('Healthy','Degraded')
  AND COALESCE(s.driftpolicy->>'Mode','Disabled') <> 'Disabled'
  AND ($3::uuid IS NULL OR s.id > $3)
ORDER BY s.id
LIMIT $4"#
            );
            sqlx::query(AssertSqlSafe(sql))
                .bind(Uuid::nil())
                .bind(true)
                .bind(after)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_stack)
                .collect()
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn create_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        input: &'a CreateStack,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_platform(&mut tx, actor, administrator, input.platform_id).await?;
            validate_references(&mut tx, &input.spec).await?;
            lock_name(&mut tx, &input.name).await?;
            ensure_name_available(&mut tx, &input.name, None).await?;
            validate_tags(&mut tx, &input.tag_ids).await?;
            let id = Uuid::now_v7();
            let release_id = Uuid::now_v7();
            let now = Utc::now();
            let spec = input.spec.to_storage_value()?;
            let drift = input
                .drift_policy
                .clone()
                .unwrap_or_default()
                .to_storage_value()?;
            let update = StackUpdateState::new(&input.spec).to_storage_value()?;
            sqlx::query("INSERT INTO stacks(id,currentstackreleaseid,name,description,stacksource,stackupdatestate,driftpolicy,createdat,createdbyactorid,controlstate,rowversion) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,'Idle',0)")
                .bind(id).bind(release_id).bind(&input.name).bind(&input.description)
                .bind(input.stack_source.as_str()).bind(update).bind(drift).bind(now).bind(actor.value())
                .execute(&mut *tx).await.map_err(database_error)?;
            sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,status,version,spec,createdat,createdbyactorid) VALUES($1,$2,$3,'Created','1',$4,$5,$6)")
                .bind(release_id).bind(id).bind(input.platform_id).bind(&spec).bind(now).bind(actor.value())
                .execute(&mut *tx).await.map_err(database_error)?;
            insert_tags(&mut tx, id, actor, &input.tag_ids).await?;
            if let Some(source) = input.duplicate_source.as_ref() {
                copy_duplicate(&mut tx, actor, administrator, id, source).await?;
            }
            let snapshot = stack_snapshot(
                id,
                &input.name,
                input.description.clone(),
                input.stack_source,
                input.drift_policy.clone().unwrap_or_default(),
                input.platform_id,
                &input.spec,
                actor,
                "1",
            );
            let info = duplicate_activity_info(input, snapshot)?;
            insert_stack_activity(
                &mut tx,
                id,
                &input.name,
                input.platform_id,
                actor,
                info,
                ActivityStatus::Information,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor, administrator, id).await
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn update_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        expected_row_version: i64,
        input: &'a UpdateStack,
        spec: &'a StackSpec,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor,
                administrator,
                id,
                policy::WriteStack::REQUIREMENT,
            )
            .await?;
            let row=sqlx::query("SELECT s.*,r.platformid,r.status release_status,r.version,r.spec release_spec FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1 FOR UPDATE OF s,r")
                .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StackError::NotFound)?;
            ensure_idle(&row)?;
            if row.try_get::<i64, _>("rowversion").map_err(storage)? != expected_row_version {
                return Err(StackError::Conflict(
                    "The Stack configuration changed while it was being updated.".to_owned(),
                ));
            }
            let old_platform: Uuid = row.try_get("platformid").map_err(storage)?;
            let new_platform = input.platform_id.unwrap_or(old_platform);
            let old_status =
                StackReleaseStatus::parse(row.try_get("release_status").map_err(storage)?)?;
            if new_platform != old_platform
                && !matches!(
                    old_status,
                    StackReleaseStatus::Created | StackReleaseStatus::Failed
                )
            {
                return Err(StackError::Validation(
                    "A deployed Stack cannot be moved to another Platform.".to_owned(),
                ));
            }
            ensure_same_platform_type(&mut tx, actor, administrator, old_platform, new_platform)
                .await?;
            validate_references(&mut tx, spec).await?;
            let old_name: String = row.try_get("name").map_err(storage)?;
            let next_name = input.name.as_deref().unwrap_or(&old_name);
            if !next_name.eq_ignore_ascii_case(&old_name) {
                lock_name(&mut tx, next_name).await?;
                ensure_name_available(&mut tx, next_name, Some(id)).await?;
            }
            let old_description: Option<String> = row.try_get("description").map_err(storage)?;
            let next_description = input
                .description
                .clone()
                .unwrap_or_else(|| old_description.clone());
            let old_spec_value: Value = row.try_get("release_spec").map_err(storage)?;
            let old_spec = StackSpec::from_storage_value(old_spec_value)?;
            let old_drift =
                StackDriftPolicy::from_storage_value(row.try_get("driftpolicy").map_err(storage)?)?;
            let new_drift = input
                .drift_policy
                .clone()
                .unwrap_or_else(|| old_drift.clone())
                .normalized();
            let release_id: Uuid = row.try_get("currentstackreleaseid").map_err(storage)?;
            let version: String = row.try_get("version").map_err(storage)?;
            let changed_definition = old_platform != new_platform || old_spec != *spec;
            if changed_definition && old_status == StackReleaseStatus::Healthy {
                sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,status,version,spec,source,resourcebindings,createdat,createdbyactorid) SELECT $1,r.stackid,r.platformid,r.status,r.version,r.spec,r.source,r.resourcebindings,r.createdat,r.createdbyactorid FROM stackreleases r WHERE r.id=$2 AND NOT EXISTS(SELECT 1 FROM stackreleases snapshot WHERE snapshot.stackid=r.stackid AND snapshot.id<>r.id AND snapshot.version=r.version AND snapshot.status='Healthy')")
                    .bind(Uuid::now_v7()).bind(release_id).execute(&mut *tx).await.map_err(storage)?;
            }
            let next_spec = spec.to_storage_value()?;
            sqlx::query("UPDATE stackreleases SET platformid=$2,spec=$3 WHERE id=$1")
                .bind(release_id)
                .bind(new_platform)
                .bind(&next_spec)
                .execute(&mut *tx)
                .await
                .map_err(database_error)?;
            sqlx::query("UPDATE stacks SET name=$2,description=$3,driftpolicy=$4,rowversion=rowversion+1 WHERE id=$1")
                .bind(id).bind(next_name).bind(&next_description).bind(new_drift.to_storage_value()?)
                .execute(&mut *tx).await.map_err(database_error)?;
            let source = parse_stack_source(row.try_get("stacksource").map_err(storage)?)?;
            let old_snapshot = stack_snapshot(
                id,
                &old_name,
                old_description,
                source,
                old_drift,
                old_platform,
                &old_spec,
                ActorId::new(row.try_get("createdbyactorid").map_err(storage)?),
                &version,
            );
            let new_snapshot = stack_snapshot(
                id,
                next_name,
                next_description,
                source,
                new_drift,
                new_platform,
                spec,
                actor,
                &version,
            );
            insert_stack_activity(
                &mut tx,
                id,
                next_name,
                new_platform,
                actor,
                ActivityEventInfo::StackUpdated {
                    old_stack: old_snapshot,
                    new_stack: new_snapshot,
                },
                ActivityStatus::Success,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor, administrator, id).await
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn update_metadata_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor,
                administrator,
                id,
                policy::WriteStack::REQUIREMENT,
            )
            .await?;
            let row=sqlx::query("SELECT s.name,s.description,s.stacksource,s.driftpolicy,s.createdbyactorid,s.controlstate,r.platformid,r.spec,r.version FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1 FOR UPDATE OF s")
                .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StackError::NotFound)?;
            ensure_idle(&row)?;
            let name: String = row.try_get("name").map_err(storage)?;
            let old_description: Option<String> = row.try_get("description").map_err(storage)?;
            let platform_id: Uuid = row.try_get("platformid").map_err(storage)?;
            let source = parse_stack_source(row.try_get("stacksource").map_err(storage)?)?;
            let drift =
                StackDriftPolicy::from_storage_value(row.try_get("driftpolicy").map_err(storage)?)?;
            let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
            let version: String = row.try_get("version").map_err(storage)?;
            let created_by = ActorId::new(row.try_get("createdbyactorid").map_err(storage)?);
            sqlx::query("UPDATE stacks SET description=$2,rowversion=rowversion+1 WHERE id=$1")
                .bind(id)
                .bind(description)
                .execute(&mut *tx)
                .await
                .map_err(database_error)?;
            insert_stack_activity(
                &mut tx,
                id,
                &name,
                platform_id,
                actor,
                ActivityEventInfo::StackUpdated {
                    old_stack: stack_snapshot(
                        id,
                        &name,
                        old_description,
                        source,
                        drift.clone(),
                        platform_id,
                        &spec,
                        created_by,
                        &version,
                    ),
                    new_stack: stack_snapshot(
                        id,
                        &name,
                        description.map(str::to_owned),
                        source,
                        drift,
                        platform_id,
                        &spec,
                        actor,
                        &version,
                    ),
                },
                ActivityStatus::Success,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor, administrator, id).await
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn rename_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        name: &'a str,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor,
                administrator,
                id,
                policy::WriteStack::REQUIREMENT,
            )
            .await?;
            let row=sqlx::query("SELECT s.name,s.controlstate,r.platformid,r.spec FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1 FOR UPDATE OF s,r")
                .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StackError::NotFound)?;
            ensure_idle(&row)?;
            let old_name: String = row.try_get("name").map_err(storage)?;
            lock_name(&mut tx, name).await?;
            ensure_name_available(&mut tx, name, Some(id)).await?;
            let mut spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
            if spec.common().project_name.is_none() {
                spec.common_mut().project_name = Some(normalize_project_name(&old_name, id));
                sqlx::query("UPDATE stackreleases SET spec=$2 WHERE id=(SELECT currentstackreleaseid FROM stacks WHERE id=$1)")
                    .bind(id).bind(spec.to_storage_value()?).execute(&mut *tx).await.map_err(storage)?;
            }
            sqlx::query("UPDATE stacks SET name=$2,rowversion=rowversion+1 WHERE id=$1")
                .bind(id)
                .bind(name)
                .execute(&mut *tx)
                .await
                .map_err(database_error)?;
            let platform_id: Uuid = row.try_get("platformid").map_err(storage)?;
            insert_stack_activity(
                &mut tx,
                id,
                name,
                platform_id,
                actor,
                ActivityEventInfo::StackRenamed {
                    old_name,
                    new_name: name.to_owned(),
                },
                ActivityStatus::Success,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor, administrator, id).await
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn import_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        input: &'a ImportComposeProject,
        claim: &'a StackImportClaim,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_platform(&mut tx, actor, administrator, input.platform_id).await?;
            validate_references(&mut tx, &input.spec).await?;
            lock_name(&mut tx, &input.name).await?;
            ensure_name_available(&mut tx, &input.name, None).await?;
            validate_tags(&mut tx, &input.tag_ids).await?;
            let id = Uuid::now_v7();
            let release_id = Uuid::now_v7();
            let now = Utc::now();
            let drift_policy = StackDriftPolicy::default();
            sqlx::query("INSERT INTO stacks(id,currentstackreleaseid,name,description,stacksource,stackupdatestate,driftpolicy,createdat,createdbyactorid,controlstate,rowversion) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,'Idle',0)")
                .bind(id).bind(release_id).bind(&input.name).bind(&input.description)
                .bind(input.spec.source().as_str()).bind(StackUpdateState::new(&input.spec).to_storage_value()?)
                .bind(drift_policy.to_storage_value()?).bind(now).bind(actor.value())
                .execute(&mut *tx).await.map_err(database_error)?;
            sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,status,version,spec,createdat,createdbyactorid) VALUES($1,$2,$3,'Created','1',$4,$5,$6)")
                .bind(release_id).bind(id).bind(input.platform_id).bind(input.spec.to_storage_value()?)
                .bind(now).bind(actor.value()).execute(&mut *tx).await.map_err(database_error)?;
            insert_tags(&mut tx, id, actor, &input.tag_ids).await?;
            if claim.import_kind == citadel_stacks::StackImportKind::SwarmStack {
                let changed=sqlx::query("UPDATE swarmserviceprojections SET stackid=$1,ownership='CitadelStack',ownershipdiagnostic=NULL WHERE platformid=$2 AND dockerstacknamespace=$3 AND stackid IS NULL AND NOT isstale")
                    .bind(id).bind(input.platform_id).bind(&claim.project_name).execute(&mut *tx).await.map_err(storage)?.rows_affected();
                if changed as usize != claim.service_names.len() {
                    return Err(StackError::Conflict(
                        "The Docker Stack changed while it was being imported.".to_owned(),
                    ));
                }
                sqlx::query("INSERT INTO stackswarmnamespacereservations(stackid,platformid,namespace,createdat) VALUES($1,$2,$3,$4)")
                    .bind(id).bind(input.platform_id).bind(&claim.project_name).bind(now).execute(&mut *tx).await.map_err(database_error)?;
            } else {
                let changed=sqlx::query("UPDATE containers SET stackid=$1 WHERE platformid=$2 AND stackid IS NULL AND stack=$3 AND NOT isswarmtask")
                    .bind(id).bind(input.platform_id).bind(&claim.project_name).execute(&mut *tx).await.map_err(storage)?.rows_affected();
                if changed as usize != claim.container_ids.len() {
                    return Err(StackError::Conflict(
                        "The Compose project changed while it was being imported.".to_owned(),
                    ));
                }
            }
            let snapshot = stack_snapshot(
                id,
                &input.name,
                input.description.clone(),
                input.spec.source(),
                drift_policy,
                input.platform_id,
                &input.spec,
                actor,
                "1",
            );
            insert_stack_activity(
                &mut tx,
                id,
                &input.name,
                input.platform_id,
                actor,
                ActivityEventInfo::StackImported {
                    stack: snapshot,
                    project_name: claim.project_name.clone(),
                },
                ActivityStatus::Information,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor, administrator, id).await
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn find_import_owner_impl(
        &self,
        platform_id: Uuid,
        project_name: &str,
    ) -> BoxFuture<'_, Result<Option<Uuid>, StackError>> {
        let project_name = project_name.to_owned();
        Box::pin(async move {
            sqlx::query_scalar("SELECT stackid FROM stackswarmnamespacereservations WHERE platformid=$1 AND namespace=$2 UNION SELECT stackid FROM swarmserviceprojections WHERE platformid=$1 AND dockerstacknamespace=$2 AND stackid IS NOT NULL UNION SELECT stackid FROM containers WHERE platformid=$1 AND stack=$2 AND NOT isswarmtask AND stackid IS NOT NULL LIMIT 1").bind(platform_id).bind(project_name).fetch_optional(&self.pool).await.map_err(storage)
        })
    }
}

pub(super) async fn ensure_same_platform_type(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    old: Uuid,
    new: Uuid,
) -> Result<(), StackError> {
    ensure_platform(tx, actor, administrator, new).await?;
    let rows = sqlx::query("SELECT id,platformdescriptor FROM platforms WHERE id=ANY($1::uuid[])")
        .bind(vec![old, new])
        .fetch_all(&mut **tx)
        .await
        .map_err(storage)?;
    let kinds = rows
        .into_iter()
        .filter_map(|r| r.try_get::<Value, _>("platformdescriptor").ok())
        .filter_map(|v| v.get("$type").and_then(Value::as_str).map(str::to_owned))
        .collect::<std::collections::BTreeSet<_>>();
    if kinds.len() != 1 {
        return Err(StackError::Validation(
            "A Stack cannot move between Docker Standalone and Docker Swarm Platforms.".to_owned(),
        ));
    }
    Ok(())
}
pub(super) async fn validate_references(
    tx: &mut Transaction<'_, Postgres>,
    spec: &StackSpec,
) -> Result<(), StackError> {
    if let Some(id) = spec.common().registry_id {
        let exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM registries WHERE id=$1)")
                .bind(id)
                .fetch_one(&mut **tx)
                .await
                .map_err(storage)?;
        if !exists {
            return Err(StackError::Validation(
                "The selected Registry does not exist.".to_owned(),
            ));
        }
    }
    if let StackSpec::Git { git_repo_id, .. } = spec {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM gitrepositories WHERE id=$1)",
        )
        .bind(git_repo_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
        if !exists {
            return Err(StackError::Validation(
                "The selected Git repository does not exist.".to_owned(),
            ));
        }
    }
    for binding in &spec.common().build_image_bindings {
        if !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM buildprojects WHERE id=$1 AND archivedat IS NULL)",
        )
        .bind(binding.build_project_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?
        {
            return Err(StackError::Validation(
                "A selected build project does not exist.".to_owned(),
            ));
        }
    }
    Ok(())
}
pub(super) async fn validate_tags(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<(), StackError> {
    if ids.is_empty() {
        return Ok(());
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE id=ANY($1::uuid[])")
        .bind(ids)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    if count as usize != ids.len() {
        return Err(StackError::Validation(
            "One or more selected Tags do not exist.".to_owned(),
        ));
    }
    Ok(())
}
pub(super) async fn insert_tags(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    actor: ActorId,
    tags: &[Uuid],
) -> Result<(), StackError> {
    for tag in tags {
        sqlx::query("INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdat,createdbyactorid) VALUES('Stack',$1,$2,$3,$4)").bind(id).bind(tag).bind(Utc::now()).bind(actor.value()).execute(&mut **tx).await.map_err(storage)?;
    }
    Ok(())
}
pub(super) async fn lock_name(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
) -> Result<(), StackError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(lower($1),2))")
        .bind(name)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    Ok(())
}
pub(super) fn next_version(value: &str) -> String {
    value
        .parse::<u64>()
        .map_or_else(|_| format!("{value}.1"), |v| (v + 1).to_string())
}
#[allow(clippy::too_many_arguments)]
pub(super) fn stack_snapshot(
    id: Uuid,
    name: &str,
    description: Option<String>,
    source: StackSource,
    drift: citadel_stacks::StackDriftPolicy,
    platform_id: Uuid,
    spec: &StackSpec,
    actor: ActorId,
    version: &str,
) -> StackActivitySnapshot {
    StackActivitySnapshot {
        id,
        name: name.to_owned(),
        description,
        stack_source: source.as_str().to_owned(),
        drift_policy: drift.to_storage_value().unwrap_or_else(|_| json!({})),
        stack_release: Some(
            json!({"PlatformId":platform_id,"Spec":spec.to_storage_value().unwrap_or_else(|_|json!({})),"CreatedByActorId":actor.value(),"Version":version,"Source":null,"ResourceBindings":null}),
        ),
    }
}
pub(super) async fn copy_duplicate(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    target: Uuid,
    source: &citadel_stacks::DuplicateStackSource,
) -> Result<(), StackError> {
    let id = source.id;
    ensure_access(
        tx,
        actor,
        administrator,
        id,
        policy::ReadStackBindings::REQUIREMENT,
    )
    .await?;
    if !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM stacks WHERE id=$1)")
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?
    {
        return Err(StackError::NotFound);
    }
    sqlx::query("INSERT INTO resourcebindings(id,name,kind,scope,value,secretid,secretdeliverymode,targetpath,resourceid,createdat,updatedat) SELECT gen_random_uuid(),name,kind,scope,value,secretid,secretdeliverymode,targetpath,$2,$3,$3 FROM resourcebindings WHERE resourceid=$1").bind(id).bind(target).bind(Utc::now()).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}
