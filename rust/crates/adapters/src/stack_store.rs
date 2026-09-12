use chrono::Utc;
use citadel_domain::{
    ActivityEvent, ActivityEventInfo, ActivityResourceType, ActivitySourceResource, ActivityStatus,
    ActorId, StackActivitySnapshot, StackResultActivitySnapshot,
};
use citadel_stacks::{
    CreateStackInput, ImportComposeProjectInput, PatchStackInput, ResourceBindingSnapshot,
    StackDeletionClaim, StackDriftPolicy, StackError, StackFilter, StackImportClaim,
    StackOperationClaim, StackReleaseSource, StackReleaseStatus, StackReleaseView,
    StackRuntimeResult, StackSource, StackSpec, StackStateClaim, StackStore, StackUpdateState,
    StackView, TagSummary, normalize_project_name,
};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::insert_activity;
mod update_checks;
mod drift;
mod webhooks;

const READ: i32 = 1;
const WRITE: i32 = 2;
const EXECUTE: i32 = 4;
const LOGS: i32 = 1;
const INSPECT: i32 = 1 << 1;
const APPLY: i32 = 1 << 2;
const PULL: i32 = 1 << 3;
const TERMINAL: i32 = 1 << 4;
const RELEASES: i32 = 1 << 6;

const AUTHORIZED_CTES: &str = r#"
WITH actor_scope AS (
    SELECT actor.id actorid FROM actors actor WHERE actor.id=$1 AND actor.isenabled
    UNION
    SELECT team.actorid FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors team_actor ON team_actor.id=team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid=$1
), global_permission AS (
    SELECT COALESCE(MAX(permission.permissionlevel),0)::integer level_mask,
           COALESCE(bit_or(permission.specificpermissions),0)::integer specific_mask
    FROM actor_scope scope JOIN actorroles assignment ON assignment.actorid=scope.actorid
    JOIN permissions permission ON permission.roleid=assignment.roleid
    WHERE permission.resourcetype=2
), resource_permissions AS (
    SELECT access.resourceid,COALESCE(MAX(access.permissionlevel),0)::integer level_mask,
           COALESCE(bit_or(access.specificpermissions),0)::integer specific_mask
    FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=2 GROUP BY access.resourceid
)
"#;

const PROJECTION: &str = r#"
SELECT s.id,s.name,s.description,s.stacksource,s.stackupdatestate,s.driftpolicy,
       s.createdat,s.createdbyactorid,s.controlstate,s.currentstackreleaseid,s.rowversion,
       r.platformid,r.status release_status,r.version,r.spec,r.source,r.resourcebindings,
       p.status platform_status,p.name platform_name,p.platformdescriptor,
       COALESCE(tags.value,'[]'::jsonb) tags,
       activity.value latest_activity,
       CASE WHEN $2 THEN 7 ELSE GREATEST(COALESCE(g.level_mask,0),COALESCE(rp.level_mask,0)) END permission_level,
       CASE WHEN $2 THEN 127 ELSE (COALESCE(g.specific_mask,0)|COALESCE(rp.specific_mask,0)) END permission_specific
FROM stacks s
JOIN stackreleases r ON r.id=s.currentstackreleaseid
JOIN platforms p ON p.id=r.platformid
LEFT JOIN global_permission g ON TRUE
LEFT JOIN resource_permissions rp ON rp.resourceid=s.id
LEFT JOIN LATERAL (
  SELECT jsonb_agg(jsonb_build_object('id',t.id,'name',t.name,'color',t.color) ORDER BY t.name,t.id) value
  FROM resourcetags rt JOIN tags t ON t.id=rt.tagid
  WHERE rt.resourcetype='Stack' AND rt.resourceid=s.id
) tags ON TRUE
LEFT JOIN LATERAL (
  SELECT jsonb_build_object('id',a.id,'resourceType',a.resourcetype,'eventType',a.eventtype,
    'status',a.status,'info',a.info::jsonb,'createdAt',a.createdat) value
  FROM activityevents a WHERE a.resourceid=s.id AND a.resourcetype='Stack'
    AND a.eventtype NOT IN ('StackUpdated','StackRenamed')
  ORDER BY a.createdat DESC,a.id DESC LIMIT 1
) activity ON TRUE
"#;

#[derive(Clone)]
pub struct PostgresStackStore {
    pool: PgPool,
}

impl PostgresStackStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl StackStore for PostgresStackStore {
    fn update_check_candidates(
        &self,
        after: Uuid,
        images: bool,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, StackError>> {
        Box::pin(update_checks::candidates(&self.pool, after, images, limit))
    }
    fn save_update_check<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a StackView,
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
    fn enqueue_webhook<'a>(
        &'a self,
        expected: &'a StackView,
        commit: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(self.enqueue_stack_webhook(expected, commit))
    }
    fn ready_webhooks(
        &self,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<citadel_stacks::StackWebhookJob>, StackError>> {
        webhooks::ready(&self.pool, limit)
    }
    fn discard_webhook(&self, id: Uuid) -> BoxFuture<'_, Result<(), StackError>> {
        Box::pin(async move {
            sqlx::query("DELETE FROM stackwebhookdeployqueue WHERE id=$1 AND status='Queued'")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(storage)?;
            Ok(())
        })
    }
    fn record_drift<'a>(
        &'a self,
        expected: &'a StackView,
        status: StackReleaseStatus,
        info: ActivityEventInfo,
    ) -> BoxFuture<'a, Result<bool, StackError>> {
        Box::pin(drift::record(&self.pool, expected, status, info))
    }

    fn drift_monitor_candidates(
        &self,
        after: Option<Uuid>,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<StackView>, StackError>> {
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

    fn list_authorized<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        filter: &'a StackFilter,
    ) -> BoxFuture<'a, Result<Vec<StackView>, StackError>> {
        Box::pin(async move {
            let sql = format!(
                r#"{AUTHORIZED_CTES}{PROJECTION}
WHERE ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(rp.level_mask,0)) >= {READ})
  AND ($3::uuid IS NULL OR r.platformid=$3)
  AND NOT EXISTS (SELECT 1 FROM unnest($4::text[]) requested(name) WHERE NOT EXISTS (
    SELECT 1 FROM resourcetags link JOIN tags tag ON tag.id=link.tagid
    WHERE link.resourcetype='Stack' AND link.resourceid=s.id
      AND (lower(tag.name)=lower(requested.name) OR tag.id::text=requested.name)))
ORDER BY s.createdat DESC,s.name,s.id"#
            );
            sqlx::query(AssertSqlSafe(sql))
                .bind(actor.value())
                .bind(administrator)
                .bind(filter.platform_id)
                .bind(&filter.tags)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_stack)
                .collect()
        })
    }

    fn get_authorized(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<StackView, StackError>> {
        Box::pin(async move { get_authorized(&self.pool, actor, administrator, id).await })
    }

    fn create<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        input: &'a CreateStackInput,
    ) -> BoxFuture<'a, Result<StackView, StackError>> {
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

    fn update<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        expected_row_version: i64,
        input: &'a PatchStackInput,
        spec: &'a StackSpec,
    ) -> BoxFuture<'a, Result<StackView, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(&mut tx, actor, administrator, id, WRITE, 0).await?;
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

    fn update_metadata<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<StackView, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(&mut tx, actor, administrator, id, WRITE, 0).await?;
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

    fn rename<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        name: &'a str,
    ) -> BoxFuture<'a, Result<StackView, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(&mut tx, actor, administrator, id, WRITE, 0).await?;
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

    fn releases(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<StackReleaseView>, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(&mut tx, actor, administrator, id, READ, RELEASES).await?;
            let rows=sqlx::query("WITH history AS (SELECT DISTINCT ON (r.version) r.* FROM stackreleases r JOIN stacks s ON s.id=r.stackid JOIN stackreleases current ON current.id=s.currentstackreleaseid WHERE r.stackid=$1 AND r.id<>current.id AND r.version<>current.version AND r.status='Healthy' ORDER BY r.version,r.createdat,r.id) SELECT r.*,p.status platform_status,p.name platform_name,a.type actor_type,COALESCE(u.name,sa.name,t.name,'Unknown') actor_name FROM history r JOIN platforms p ON p.id=r.platformid JOIN actors a ON a.id=r.createdbyactorid LEFT JOIN users u ON u.actorid=a.id LEFT JOIN serviceaccounts sa ON sa.actorid=a.id LEFT JOIN teams t ON t.actorid=a.id ORDER BY r.createdat DESC,r.id DESC")
                .bind(id).fetch_all(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            rows.into_iter().map(map_release).collect()
        })
    }

    fn claim_apply(
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

    fn claim_apply_versioned(
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
            ensure_access(&mut tx, actor, administrator, id, EXECUTE, APPLY).await?;
            let row=sqlx::query("SELECT s.*,r.platformid,r.status release_status,r.version,r.spec,p.status platform_status,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1 FOR UPDATE OF s,r")
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
            let status = StackReleaseStatus::parse(row.try_get("release_status").map_err(storage)?)?;
            let create_next = has_snapshot
                && !matches!(status, StackReleaseStatus::Created | StackReleaseStatus::Failed);
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
                        .bind(current_release).execute(&mut *tx).await.map_err(storage)?;
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
            let platform_type = descriptor
                .get("$type")
                .or_else(|| descriptor.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("Docker")
                .to_owned();
            let project_name = spec.common().project_name.clone().unwrap_or_else(|| {
                normalize_project_name(
                    row.try_get::<String, _>("name")
                        .unwrap_or_default()
                        .as_str(),
                    id,
                )
            });
            if !options.service_names.is_empty() {
                if platform_type != "Docker"
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

    fn record_apply_source<'a>(
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
    fn complete_apply<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackOperationClaim,
        result: &'a StackRuntimeResult,
        bindings: &'a [ResourceBindingSnapshot],
        source: Option<&'a StackReleaseSource>,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row=sqlx::query("SELECT s.name,s.description,s.stacksource,s.driftpolicy,s.stackupdatestate,r.spec,r.version FROM stacks s JOIN stackreleases r ON r.id=$2 AND r.stackid=s.id WHERE s.id=$1 AND s.currentstackreleaseid=$2 AND s.controlstate='Processing' AND s.rowversion=$3 FOR UPDATE OF s,r")
                .bind(claim.stack_id).bind(claim.release_id).bind(claim.row_version).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or_else(|| StackError::Conflict("The Stack operation was superseded.".to_owned()))?;
            let source = source.filter(|_| claim.service_names.is_empty());
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

    fn fail_apply<'a>(
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
            if sqlx::query_scalar::<_, Uuid>("SELECT id FROM stacks WHERE id=$1 AND currentstackreleaseid=$2 AND rowversion=$3 AND controlstate='Processing' FOR UPDATE")
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

    fn stale_apply_claims(
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
                    let platform_type=descriptor.get("$type").and_then(Value::as_str).unwrap_or("Docker").to_owned();
                    let name:String=row.try_get("name").map_err(storage)?;
                    let actor=ActorId::new(row.try_get("controltriggeredby").map_err(storage)?);
                    Ok((actor,StackOperationClaim { stack_id:id,release_id:row.try_get("currentstackreleaseid").map_err(storage)?,platform_id:row.try_get("platformid").map_err(storage)?,project_name:spec.common().project_name.clone().unwrap_or_else(|| normalize_project_name(&name,id)),platform_type,spec,row_version:row.try_get("rowversion").map_err(storage)?,actor_id:actor.value(),name,operation:"Apply".to_owned(),service_names:row.try_get("applyservices").map_err(storage)? }))
                }).collect()
        })
    }

    fn stale_delete_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackDeletionClaim)>, StackError>> {
        Box::pin(async move {
            sqlx::query("SELECT s.id,s.name,s.controltriggeredby,r.platformid,r.spec,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.controlstate='Processing' AND s.containeroperationid IS NULL AND s.controlstartedat <= $1 AND r.status NOT IN ('Applying','Pending') AND s.controltriggeredby IS NOT NULL ORDER BY s.controlstartedat,s.id LIMIT $2")
                .bind(started_before).bind(limit).fetch_all(&self.pool).await.map_err(storage)?.into_iter().map(|row| {
                    let id:Uuid=row.try_get("id").map_err(storage)?;
                    let spec=StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                    let descriptor:Value=row.try_get("platformdescriptor").map_err(storage)?;
                    let name:String=row.try_get("name").map_err(storage)?;
                    Ok((ActorId::new(row.try_get("controltriggeredby").map_err(storage)?), StackDeletionClaim {
                        stack_id:id,
                        platform_id:row.try_get("platformid").map_err(storage)?,
                        project_name:spec.common().project_name.clone().unwrap_or_else(|| normalize_project_name(&name,id)),
                        platform_type:descriptor.get("$type").and_then(Value::as_str).unwrap_or("Docker").to_owned(),
                    }))
                }).collect()
        })
    }

    fn claim_delete<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StackDeletionClaim>, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let mut claims = Vec::with_capacity(ids.len());
            for id in ids {
                ensure_access(&mut tx, actor, administrator, *id, EXECUTE, 0).await?;
                let row=sqlx::query("SELECT s.name,s.controlstate,r.platformid,r.spec,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1 FOR UPDATE OF s")
                    .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StackError::NotFound)?;
                ensure_idle(&row)?;
                let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
                let name: String = row.try_get("name").map_err(storage)?;
                claims.push(StackDeletionClaim {
                    stack_id: *id,
                    platform_id: row.try_get("platformid").map_err(storage)?,
                    project_name: spec
                        .common()
                        .project_name
                        .clone()
                        .unwrap_or_else(|| normalize_project_name(&name, *id)),
                    platform_type: descriptor
                        .get("$type")
                        .and_then(Value::as_str)
                        .unwrap_or("Docker")
                        .to_owned(),
                });
            }
            for id in ids {
                sqlx::query("UPDATE stacks SET controlstate='Processing',controlstartedat=$2,controltriggeredby=$3,rowversion=rowversion+1 WHERE id=$1").bind(id).bind(Utc::now().timestamp()).bind(actor.value()).execute(&mut *tx).await.map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(claims)
        })
    }

    fn complete_delete<'a>(
        &'a self,
        actor: ActorId,
        claims: &'a [StackDeletionClaim],
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            for claim in claims {
                let row=sqlx::query("SELECT s.name,s.description,s.stacksource,s.driftpolicy,r.spec,r.version FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1 AND s.controlstate='Processing' FOR UPDATE OF s")
                    .bind(claim.stack_id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StackError::NotFound)?;
                let name: String = row.try_get("name").map_err(storage)?;
                let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                let snapshot = stack_snapshot(
                    claim.stack_id,
                    &name,
                    row.try_get("description").map_err(storage)?,
                    parse_stack_source(row.try_get("stacksource").map_err(storage)?)?,
                    StackDriftPolicy::from_storage_value(
                        row.try_get("driftpolicy").map_err(storage)?,
                    )?,
                    claim.platform_id,
                    &spec,
                    actor,
                    row.try_get("version").map_err(storage)?,
                );
                insert_stack_activity(
                    &mut tx,
                    claim.stack_id,
                    &name,
                    claim.platform_id,
                    actor,
                    ActivityEventInfo::StackDeleted { stack: snapshot },
                    ActivityStatus::Success,
                )
                .await?;
                sqlx::query("DELETE FROM stacks WHERE id=$1 AND controlstate='Processing'")
                    .bind(claim.stack_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn release_delete<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            sqlx::query("UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE id=ANY($1::uuid[]) AND controlstate='Processing'").bind(ids).execute(&self.pool).await.map_err(storage)?;
            Ok(())
        })
    }

    fn claim_state<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StackStateClaim>, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let mut claims = Vec::with_capacity(ids.len());
            for id in ids {
                ensure_access(&mut tx, actor, administrator, *id, EXECUTE, 0).await?;
                let row = sqlx::query("SELECT s.name,s.controlstate,s.currentstackreleaseid,r.platformid,r.status,r.spec,p.status platform_status,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1 FOR UPDATE OF s,r")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(storage)?
                    .ok_or(StackError::NotFound)?;
                ensure_idle(&row)?;
                if row
                    .try_get::<String, _>("platform_status")
                    .map_err(storage)?
                    != "Online"
                {
                    return Err(StackError::Conflict(
                        "The Stack Platform is offline.".to_owned(),
                    ));
                }
                let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
                let platform_type = descriptor
                    .get("$type")
                    .or_else(|| descriptor.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or("Docker")
                    .to_owned();
                if platform_type == "DockerSwarm" {
                    return Err(StackError::Validation(
                        "Container state actions are not available for Docker Swarm stacks."
                            .to_owned(),
                    ));
                }
                let name: String = row.try_get("name").map_err(storage)?;
                let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                claims.push(StackStateClaim {
                    stack_id: *id,
                    release_id: row.try_get("currentstackreleaseid").map_err(storage)?,
                    platform_id: row.try_get("platformid").map_err(storage)?,
                    name: name.clone(),
                    project_name: spec
                        .common()
                        .project_name
                        .clone()
                        .unwrap_or_else(|| normalize_project_name(&name, *id)),
                    platform_type,
                    previous_status: StackReleaseStatus::parse(
                        &row.try_get::<String, _>("status").map_err(storage)?,
                    )?,
                    actor_id: actor.value(),
                });
            }
            let now = Utc::now().timestamp();
            for claim in &claims {
                sqlx::query("UPDATE stackreleases SET status='Pending' WHERE id=$1 AND stackid=$2")
                    .bind(claim.release_id)
                    .bind(claim.stack_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
                sqlx::query("UPDATE stacks SET controlstate='Processing',controlstartedat=$2,controltriggeredby=$3,rowversion=rowversion+1 WHERE id=$1")
                    .bind(claim.stack_id)
                    .bind(now)
                    .bind(actor.value())
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(claims)
        })
    }

    fn complete_state<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackStateClaim,
        status: StackReleaseStatus,
        container_ids: &'a [String],
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            let info = match status {
                StackReleaseStatus::Healthy => ActivityEventInfo::StackStarted {
                    container_ids: container_ids.to_vec(),
                },
                StackReleaseStatus::Stopped => ActivityEventInfo::StackStopped {
                    container_ids: container_ids.to_vec(),
                },
                StackReleaseStatus::Paused => ActivityEventInfo::StackPaused {
                    container_ids: container_ids.to_vec(),
                },
                _ => {
                    return Err(StackError::Validation(
                        "A Stack state operation did not reach a stable state.".to_owned(),
                    ));
                }
            };
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let locked = sqlx::query_scalar::<_, String>("SELECT name FROM stacks WHERE id=$1 AND currentstackreleaseid=$2 AND controlstate='Processing' AND controltriggeredby=$3 FOR UPDATE")
                .bind(claim.stack_id)
                .bind(claim.release_id)
                .bind(claim.actor_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or_else(|| StackError::Conflict("The Stack state operation was superseded.".to_owned()))?;
            let release_changed = sqlx::query("UPDATE stackreleases SET status=$3 WHERE id=$1 AND stackid=$2 AND status='Pending'")
                .bind(claim.release_id)
                .bind(claim.stack_id)
                .bind(status.as_str())
                .execute(&mut *tx)
                .await
                .map_err(storage)?
                .rows_affected();
            let stack_changed = sqlx::query("UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentstackreleaseid=$2 AND controlstate='Processing' AND controltriggeredby=$3")
                .bind(claim.stack_id)
                .bind(claim.release_id)
                .bind(claim.actor_id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?
                .rows_affected();
            if release_changed != 1 || stack_changed != 1 {
                return Err(StackError::Conflict(
                    "The Stack state operation was superseded.".to_owned(),
                ));
            }
            insert_stack_activity(
                &mut tx,
                claim.stack_id,
                &locked,
                claim.platform_id,
                actor,
                info,
                ActivityStatus::Success,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn release_state<'a>(
        &'a self,
        claims: &'a [StackStateClaim],
    ) -> BoxFuture<'a, Result<(), StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            for claim in claims {
                let locked = sqlx::query_scalar::<_, Uuid>("SELECT currentstackreleaseid FROM stacks WHERE id=$1 AND currentstackreleaseid=$2 AND controlstate='Processing' AND controltriggeredby=$3 FOR UPDATE")
                    .bind(claim.stack_id)
                    .bind(claim.release_id)
                    .bind(claim.actor_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(storage)?;
                if locked.is_none() {
                    continue;
                }
                sqlx::query("UPDATE stackreleases SET status=$3 WHERE id=$1 AND stackid=$2 AND status='Pending'")
                    .bind(claim.release_id)
                    .bind(claim.stack_id)
                    .bind(claim.previous_status.as_str())
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
                sqlx::query("UPDATE stacks SET controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 WHERE id=$1 AND currentstackreleaseid=$2 AND controlstate='Processing' AND controltriggeredby=$3")
                    .bind(claim.stack_id)
                    .bind(claim.release_id)
                    .bind(claim.actor_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn stale_state_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackStateClaim)>, StackError>> {
        Box::pin(async move {
            sqlx::query("SELECT s.id,s.name,s.controltriggeredby,s.currentstackreleaseid,r.platformid,r.spec,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.controlstate='Processing' AND s.containeroperationid IS NULL AND s.controlstartedat <= $1 AND r.status='Pending' AND s.controltriggeredby IS NOT NULL ORDER BY s.controlstartedat,s.id LIMIT $2")
                .bind(started_before)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(|row| {
                    let id: Uuid = row.try_get("id").map_err(storage)?;
                    let actor = ActorId::new(row.try_get("controltriggeredby").map_err(storage)?);
                    let name: String = row.try_get("name").map_err(storage)?;
                    let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
                    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
                    Ok((actor, StackStateClaim {
                        stack_id: id,
                        release_id: row.try_get("currentstackreleaseid").map_err(storage)?,
                        platform_id: row.try_get("platformid").map_err(storage)?,
                        project_name: spec.common().project_name.clone().unwrap_or_else(|| normalize_project_name(&name, id)),
                        platform_type: descriptor.get("$type").or_else(|| descriptor.get("type")).and_then(Value::as_str).unwrap_or("Docker").to_owned(),
                        previous_status: StackReleaseStatus::Unknown,
                        actor_id: actor.value(),
                        name,
                    }))
                })
                .collect()
        })
    }

    fn import<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        input: &'a ImportComposeProjectInput,
        claim: &'a StackImportClaim,
    ) -> BoxFuture<'a, Result<StackView, StackError>> {
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

    fn find_import_owner(
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

async fn get_authorized(
    pool: &PgPool,
    actor: ActorId,
    administrator: bool,
    id: Uuid,
) -> Result<StackView, StackError> {
    let sql = format!(
        r#"{AUTHORIZED_CTES}{PROJECTION} WHERE s.id=$3 AND ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(rp.level_mask,0)) >= {READ})"#
    );
    sqlx::query(AssertSqlSafe(sql))
        .bind(actor.value())
        .bind(administrator)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .map(map_stack)
        .transpose()?
        .ok_or(StackError::NotFound)
}

fn map_stack(row: PgRow) -> Result<StackView, StackError> {
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    let level: i32 = row.try_get("permission_level").map_err(storage)?;
    let specific: i32 = row.try_get("permission_specific").map_err(storage)?;
    Ok(StackView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        stack_source: parse_stack_source(row.try_get("stacksource").map_err(storage)?)?,
        stack_update_state: StackUpdateState::from_storage_value(
            row.try_get("stackupdatestate").map_err(storage)?,
        )?,
        drift_policy: StackDriftPolicy::from_storage_value(
            row.try_get("driftpolicy").map_err(storage)?,
        )?,
        status: StackReleaseStatus::parse(row.try_get("release_status").map_err(storage)?)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        control_state: row.try_get("controlstate").map_err(storage)?,
        current_stack_release_id: row.try_get("currentstackreleaseid").map_err(storage)?,
        platform_type: descriptor
            .get("$type")
            .or_else(|| descriptor.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("Docker")
            .to_owned(),
        platform_id: Some(row.try_get("platformid").map_err(storage)?),
        version: Some(row.try_get("version").map_err(storage)?),
        spec: Some(StackSpec::from_storage_value(
            row.try_get("spec").map_err(storage)?,
        )?),
        source: row
            .try_get::<Option<Value>, _>("source")
            .map_err(storage)?
            .map(StackReleaseSource::from_storage_value)
            .transpose()?,
        resource_bindings: row
            .try_get::<Option<Value>, _>("resourcebindings")
            .map_err(storage)?
            .map(ResourceBindingSnapshot::list_from_storage_value)
            .transpose()?,
        platform_status: row.try_get("platform_status").map_err(storage)?,
        platform_name: Some(row.try_get("platform_name").map_err(storage)?),
        tags: serde_json::from_value::<Vec<TagSummary>>(row.try_get("tags").map_err(storage)?)
            .map_err(|error| StackError::Storage(error.to_string()))?,
        latest_activity_view: drift::public_activity(row.try_get("latest_activity").map_err(storage)?),
        capabilities: Some(citadel_stacks::StackCapabilities {
            can_read: level >= READ,
            can_write: level >= WRITE,
            can_execute: level & EXECUTE != 0,
            can_delete: level & EXECUTE != 0,
            can_view_logs: level >= READ && specific & LOGS != 0,
            can_inspect: level >= READ && specific & INSPECT != 0,
            can_open_terminal: level >= READ && specific & TERMINAL != 0,
            can_pull: level >= READ && specific & PULL != 0,
            can_apply: level & EXECUTE != 0 && specific & APPLY != 0,
            can_view_resource_bindings: level >= READ && specific & (1 << 5) != 0,
            can_view_releases: level >= READ && specific & RELEASES != 0,
        }),
        row_version: row.try_get("rowversion").map_err(storage)?,
    })
}

fn map_release(row: PgRow) -> Result<StackReleaseView, StackError> {
    Ok(StackReleaseView {
        id: row.try_get("id").map_err(storage)?,
        stack_id: row.try_get("stackid").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        status: StackReleaseStatus::parse(row.try_get("status").map_err(storage)?)?,
        version: row.try_get("version").map_err(storage)?,
        spec: StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?,
        source: row
            .try_get::<Option<Value>, _>("source")
            .map_err(storage)?
            .map(StackReleaseSource::from_storage_value)
            .transpose()?,
        resource_bindings: row
            .try_get::<Option<Value>, _>("resourcebindings")
            .map_err(storage)?
            .map(ResourceBindingSnapshot::list_from_storage_value)
            .transpose()?,
        created_at: row.try_get("createdat").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        actor_name: row.try_get("actor_name").map_err(storage)?,
        actor_type: row.try_get("actor_type").map_err(storage)?,
        platform_status: row.try_get("platform_status").map_err(storage)?,
        platform_name: Some(row.try_get("platform_name").map_err(storage)?),
    })
}

async fn ensure_access(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    id: Uuid,
    level: i32,
    specific: i32,
) -> Result<(), StackError> {
    if administrator {
        return Ok(());
    }
    let row=sqlx::query(r#"WITH actor_scope AS (SELECT id actorid FROM actors WHERE id=$1 AND isenabled UNION SELECT t.actorid FROM actorteammemberships m JOIN teams t ON t.id=m.teamid JOIN actors a ON a.id=t.actorid AND a.isenabled WHERE m.memberactorid=$1), effective AS (SELECT COALESCE(MAX(p.permissionlevel),0)::integer level_mask,COALESCE(bit_or(p.specificpermissions),0)::integer specific_mask FROM actor_scope s JOIN actorroles ar ON ar.actorid=s.actorid JOIN permissions p ON p.roleid=ar.roleid WHERE p.resourcetype=2 UNION ALL SELECT COALESCE(MAX(ra.permissionlevel),0)::integer,COALESCE(bit_or(ra.specificpermissions),0)::integer FROM actor_scope s JOIN resourceaccesses ra ON ra.actorid=s.actorid WHERE ra.resourcetype=2 AND ra.resourceid=$2) SELECT COALESCE(MAX(level_mask),0)::integer level_mask,COALESCE(bit_or(specific_mask),0)::integer specific_mask FROM effective"#).bind(actor.value()).bind(id).fetch_one(&mut **tx).await.map_err(storage)?;
    let actual: i32 = row.try_get("level_mask").map_err(storage)?;
    let specs: i32 = row.try_get("specific_mask").map_err(storage)?;
    if actual < level || (specific != 0 && specs & specific != specific) {
        return Err(StackError::Forbidden);
    }
    Ok(())
}

async fn ensure_platform(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    id: Uuid,
) -> Result<(), StackError> {
    let row = sqlx::query("SELECT platformdescriptor FROM platforms WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(StackError::NotFound)?;
    if !administrator {
        let allowed:bool=sqlx::query_scalar(r#"WITH actor_scope AS (
            SELECT id actorid FROM actors WHERE id=$1 AND isenabled
            UNION
            SELECT team.actorid FROM actorteammemberships membership
            JOIN teams team ON team.id=membership.teamid
            JOIN actors team_actor ON team_actor.id=team.actorid AND team_actor.isenabled
            WHERE membership.memberactorid=$1)
            SELECT EXISTS(
                SELECT 1 FROM actor_scope scope
                JOIN actorroles assignment ON assignment.actorid=scope.actorid
                JOIN permissions permission ON permission.roleid=assignment.roleid
                WHERE permission.resourcetype=0 AND permission.permissionlevel>=1
                UNION ALL
                SELECT 1 FROM actor_scope scope
                JOIN resourceaccesses access ON access.actorid=scope.actorid
                WHERE access.resourcetype=0 AND access.resourceid=$2 AND access.permissionlevel>=1)"#)
            .bind(actor.value()).bind(id).fetch_one(&mut **tx).await.map_err(storage)?;
        if !allowed {
            return Err(StackError::NotFound);
        }
    }
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    let kind = descriptor
        .get("$type")
        .and_then(Value::as_str)
        .unwrap_or("Docker");
    if !matches!(kind, "Docker" | "DockerSwarm") {
        return Err(StackError::Validation(
            "Stacks require a Docker or Docker Swarm Platform.".to_owned(),
        ));
    }
    Ok(())
}

async fn ensure_same_platform_type(
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
async fn validate_references(
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
async fn validate_tags(tx: &mut Transaction<'_, Postgres>, ids: &[Uuid]) -> Result<(), StackError> {
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
async fn insert_tags(
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
async fn lock_name(tx: &mut Transaction<'_, Postgres>, name: &str) -> Result<(), StackError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(lower($1),2))")
        .bind(name)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    Ok(())
}
async fn ensure_name_available(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    exclude: Option<Uuid>,
) -> Result<(), StackError> {
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stacks WHERE lower(name)=lower($1) AND ($2::uuid IS NULL OR id<>$2))").bind(name).bind(exclude).fetch_one(&mut **tx).await.map_err(storage)?;
    if exists {
        Err(StackError::Conflict(
            "A Stack with this name already exists.".to_owned(),
        ))
    } else {
        Ok(())
    }
}
fn ensure_idle(row: &PgRow) -> Result<(), StackError> {
    if row
        .try_get::<Option<String>, _>("controlstate")
        .map_err(storage)?
        .as_deref()
        != Some("Idle")
    {
        return Err(StackError::Conflict(
            "The Stack has an operation in progress.".to_owned(),
        ));
    }
    Ok(())
}
fn parse_stack_source(value: &str) -> Result<StackSource, StackError> {
    match value {
        "WebEditor" => Ok(StackSource::WebEditor),
        "Git" => Ok(StackSource::Git),
        _ => Err(StackError::Storage(format!(
            "invalid Stack source '{value}'"
        ))),
    }
}
fn next_version(value: &str) -> String {
    value
        .parse::<u64>()
        .map_or_else(|_| format!("{value}.1"), |v| (v + 1).to_string())
}
#[allow(clippy::too_many_arguments)]
fn stack_snapshot(
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
fn duplicate_activity_info(
    input: &CreateStackInput,
    snapshot: StackActivitySnapshot,
) -> Result<ActivityEventInfo, StackError> {
    let Some(source) = input.duplicate_source.as_ref() else {
        return Ok(ActivityEventInfo::StackCreated { stack: snapshot });
    };
    let resource_id = source
        .get("resourceId")
        .or_else(|| source.get("ResourceId"))
        .and_then(Value::as_str)
        .and_then(|v| Uuid::parse_str(v).ok())
        .ok_or_else(|| StackError::Validation("Duplicate source is invalid.".to_owned()))?;
    let resource_name = source
        .get("resourceName")
        .or_else(|| source.get("ResourceName"))
        .and_then(Value::as_str)
        .unwrap_or("Stack")
        .to_owned();
    Ok(ActivityEventInfo::StackDuplicated {
        stack: snapshot,
        source: ActivitySourceResource {
            resource_type: ActivityResourceType::Stack,
            resource_id,
            resource_name,
        },
    })
}
async fn copy_duplicate(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    administrator: bool,
    target: Uuid,
    source: &Value,
) -> Result<(), StackError> {
    let id = source
        .get("resourceId")
        .or_else(|| source.get("ResourceId"))
        .and_then(Value::as_str)
        .and_then(|v| Uuid::parse_str(v).ok())
        .ok_or_else(|| StackError::Validation("Duplicate source is invalid.".to_owned()))?;
    ensure_access(tx, actor, administrator, id, READ, 1 << 5).await?;
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
async fn insert_stack_activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: &str,
    platform: Uuid,
    actor: ActorId,
    info: ActivityEventInfo,
    status: ActivityStatus,
) -> Result<(), StackError> {
    let event = ActivityEvent::new_stack_event(
        id,
        name.to_owned(),
        platform,
        actor,
        info,
        status,
        Utc::now(),
    )
    .map_err(|e| StackError::Storage(e.to_string()))?;
    insert_activity(tx, &event)
        .await
        .map_err(|e| StackError::Storage(e.to_string()))
}
fn storage(error: impl std::fmt::Display) -> StackError {
    StackError::Storage(error.to_string())
}
fn database_error(error: sqlx::Error) -> StackError {
    match &error {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            StackError::Conflict("A Stack with the same identity already exists.".to_owned())
        }
        sqlx::Error::Database(db) if db.is_foreign_key_violation() => {
            StackError::Validation("A referenced Stack resource does not exist.".to_owned())
        }
        _ => storage(error),
    }
}
