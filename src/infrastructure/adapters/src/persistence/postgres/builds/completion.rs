use citadel_builds::{
    BuildCompletionRepository, BuildConsumerClaim, BuildConsumerType, BuildError,
};
use citadel_deployments::{DeploymentImageInfo, DeploymentSpec};
use citadel_stacks::StackSpec;
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

pub struct PostgresBuildCompletionRepository {
    pool: PgPool,
}
impl PostgresBuildCompletionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Same transaction as Build success: a crash cannot lose consumer propagation.
pub(crate) async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    project: Uuid,
    run: Uuid,
) -> Result<(), BuildError> {
    sqlx::query("INSERT INTO buildcompletionqueue(id,buildrunid,buildprojectid,resourceid,resourcetype) SELECT gen_random_uuid(),$2,$1,id,'Deployment' FROM deployments WHERE spec->'Image'->>'$type'='Build' AND spec->'Image'->>'BuildProjectId'=$1::text ON CONFLICT (resourceid,resourcetype,buildprojectid) WHERE status='Queued' DO UPDATE SET buildrunid=EXCLUDED.buildrunid,queuedat=now()")
        .bind(project).bind(run).execute(&mut **tx).await.map_err(storage)?;
    sqlx::query("INSERT INTO buildcompletionqueue(id,buildrunid,buildprojectid,resourceid,resourcetype) SELECT gen_random_uuid(),$2,$1,s.id,'Stack' FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE EXISTS(SELECT 1 FROM jsonb_array_elements(COALESCE(r.spec::jsonb->'BuildImageBindings','[]'::jsonb)) b WHERE b->>'BuildProjectId'=$1::text) ON CONFLICT (resourceid,resourcetype,buildprojectid) WHERE status='Queued' DO UPDATE SET buildrunid=EXCLUDED.buildrunid,queuedat=now()")
        .bind(project).bind(run).execute(&mut **tx).await.map_err(storage)?;
    sqlx::query("SELECT pg_notify('citadel_build_completion','')")
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    Ok(())
}

impl BuildCompletionRepository for PostgresBuildCompletionRepository {
    fn claim_next(&self) -> BoxFuture<'_, Result<Option<BuildConsumerClaim>, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            // A short transaction lock prevents overlapping claimers; it is never
            // held while pulling images or applying a resource.
            if !sqlx::query_scalar::<_, bool>(
                "SELECT pg_try_advisory_xact_lock(hashtext('citadel-build-completion'))",
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?
            {
                return Ok(None);
            }
            let Some(row)=sqlx::query("SELECT q.*,b.buildprojectid,b.imagereferences,b.imagedigest FROM buildcompletionqueue q JOIN buildruns b ON b.id=q.buildrunid LEFT JOIN deployments d ON q.resourcetype='Deployment' AND d.id=q.resourceid LEFT JOIN stacks s ON q.resourcetype='Stack' AND s.id=q.resourceid WHERE q.status='Queued' AND COALESCE(d.controlstate,s.controlstate,'Idle')='Idle' AND NOT EXISTS(SELECT 1 FROM buildcompletionqueue active WHERE active.resourceid=q.resourceid AND active.resourcetype=q.resourcetype AND active.status='Processing') ORDER BY q.queuedat,q.id LIMIT 1")
                .fetch_optional(&mut *tx).await.map_err(storage)? else { return Ok(None); };
            let id: Uuid = row.try_get("id").map_err(storage)?;
            let run: Uuid = row.try_get("buildrunid").map_err(storage)?;
            let resource: Uuid = row.try_get("resourceid").map_err(storage)?;
            let project: Uuid = row.try_get("buildprojectid").map_err(storage)?;
            let references: Value = row.try_get("imagereferences").map_err(storage)?;
            let reference = references
                .as_array()
                .and_then(|values| values.first())
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    BuildError::Storage("Completed Build has no image reference.".into())
                })?;
            let digest: Option<String> = row.try_get("imagedigest").map_err(storage)?;
            let kind = match row.try_get::<&str, _>("resourcetype").map_err(storage)? {
                "Deployment" => BuildConsumerType::Deployment,
                "Stack" => BuildConsumerType::Stack,
                _ => return Err(BuildError::Storage("Invalid Build consumer type.".into())),
            };
            // Lock the resource before the queue row, matching Build completion's
            // enqueue order; an Apply never has its row version changed underneath it.
            let update = match kind {
                BuildConsumerType::Deployment => {
                    update_deployment(
                        &mut tx,
                        resource,
                        project,
                        run,
                        reference,
                        digest.as_deref(),
                    )
                    .await?
                }
                BuildConsumerType::Stack => {
                    update_stack(
                        &mut tx,
                        resource,
                        project,
                        run,
                        reference,
                        digest.as_deref(),
                    )
                    .await?
                }
            };
            if sqlx::query("UPDATE buildcompletionqueue SET status='Processing',startedat=now() WHERE id=$1 AND buildrunid=$2 AND status='Queued'")
                .bind(id).bind(run).execute(&mut *tx).await.map_err(storage)?.rows_affected()!=1 { return Ok(None); }
            tx.commit().await.map_err(storage)?;
            Ok(Some(BuildConsumerClaim {
                id,
                build_run_id: run,
                resource_id: resource,
                resource_type: kind,
                expected_version: update.as_ref().map(|(version, _, _)| *version),
                redeploy: update.as_ref().is_some_and(|(_, redeploy, _)| *redeploy),
                service_names: update.map(|(_, _, services)| services).unwrap_or_default(),
            }))
        })
    }
    fn complete<'a>(
        &'a self,
        claim: &'a BuildConsumerClaim,
        message: &'a str,
    ) -> BoxFuture<'a, Result<Option<citadel_builds::BuildLogEntry>, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let removed=sqlx::query("DELETE FROM buildcompletionqueue WHERE id=$1 AND buildrunid=$2 AND status='Processing'")
                .bind(claim.id).bind(claim.build_run_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            let entry = if removed == 1 {
                let entry = citadel_builds::BuildLogEntry {
                    id: Uuid::now_v7(),
                    build_run_id: claim.build_run_id,
                    created_at: chrono::Utc::now(),
                    stream: "stdout".into(),
                    message: format!(
                        "{} {}: {message}",
                        claim.resource_type.as_str(),
                        claim.resource_id
                    ),
                };
                sqlx::query("INSERT INTO buildrunlogs(id,buildrunid,createdat,message,stream) VALUES($1,$2,$4,$3,'stdout')")
                    .bind(entry.id).bind(claim.build_run_id)
                    .bind(&entry.message)
                    .bind(entry.created_at)
                    .execute(&mut *tx).await.map_err(storage)?;
                Some(entry)
            } else {
                None
            };
            tx.commit().await.map_err(storage)?;
            Ok(entry)
        })
    }
    fn recover(&self) -> BoxFuture<'_, Result<(), BuildError>> {
        Box::pin(async move {
            // Apply reconciliation owns ambiguous Docker outcomes. Never replay an
            // interrupted consumer mutation. Keep a durable explanation in Build logs.
            sqlx::query("WITH stale AS (SELECT id FROM buildcompletionqueue WHERE status='Processing' AND startedat<now()-interval '30 minutes' ORDER BY startedat,id LIMIT 25 FOR UPDATE SKIP LOCKED), removed AS (DELETE FROM buildcompletionqueue q USING stale WHERE q.id=stale.id RETURNING q.buildrunid,q.resourceid,q.resourcetype) INSERT INTO buildrunlogs(id,buildrunid,createdat,message,stream) SELECT gen_random_uuid(),buildrunid,now(),resourcetype||' '||resourceid::text||': Build consumer processing was interrupted. Review resource state before applying again.','stderr' FROM removed")
                .execute(&self.pool).await.map_err(storage)?;
            Ok(())
        })
    }
}

async fn update_deployment(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    project: Uuid,
    run: Uuid,
    reference: &str,
    digest: Option<&str>,
) -> Result<Option<(i64, bool, Vec<String>)>, BuildError> {
    let Some(row) = sqlx::query(
        "SELECT spec,rowversion,controlstate FROM deployments WHERE id=$1 FOR NO KEY UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    else {
        return Ok(None);
    };
    if row.try_get::<&str, _>("controlstate").map_err(storage)? != "Idle" {
        return Err(BuildError::Conflict("Build consumer is busy.".into()));
    }
    let mut spec = DeploymentSpec::from_storage_value(row.try_get("spec").map_err(storage)?)
        .map_err(storage)?;
    let DeploymentImageInfo::Build {
        build_project_id,
        redeploy_on_build,
        resolved_image_reference,
        resolved_digest,
        resolved_build_run_id,
        ..
    } = &mut spec.image
    else {
        return Ok(None);
    };
    if *build_project_id != project {
        return Ok(None);
    }
    *resolved_image_reference = Some(reference.into());
    *resolved_digest = digest.map(str::to_owned);
    *resolved_build_run_id = Some(run);
    let redeploy = *redeploy_on_build;
    sqlx::query("UPDATE deployments SET spec=$2,rowversion=rowversion+1 WHERE id=$1")
        .bind(id)
        .bind(spec.to_storage_value().map_err(storage)?)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    Ok(Some((
        row.try_get::<i64, _>("rowversion").map_err(storage)? + 1,
        redeploy,
        Vec::new(),
    )))
}
async fn update_stack(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    project: Uuid,
    run: Uuid,
    reference: &str,
    digest: Option<&str>,
) -> Result<Option<(i64, bool, Vec<String>)>, BuildError> {
    let Some(row)=sqlx::query("SELECT r.spec,s.rowversion,s.controlstate,r.id releaseid FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1 FOR NO KEY UPDATE OF s,r")
        .bind(id).fetch_optional(&mut **tx).await.map_err(storage)? else { return Ok(None); };
    if row.try_get::<&str, _>("controlstate").map_err(storage)? != "Idle" {
        return Err(BuildError::Conflict("Build consumer is busy.".into()));
    }
    let mut spec =
        StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?).map_err(storage)?;
    let mut changed = false;
    let mut redeploy = false;
    let mut services = Vec::new();
    for binding in &mut spec.common_mut().build_image_bindings {
        if binding.build_project_id != project {
            continue;
        }
        changed = true;
        redeploy |= binding.redeploy_on_build;
        if binding.redeploy_on_build {
            services.push(binding.service_name.clone());
        }
        binding.resolved_image_reference = Some(reference.into());
        binding.resolved_digest = digest.map(str::to_owned);
        binding.resolved_build_run_id = Some(run);
    }
    if !changed {
        return Ok(None);
    }
    sqlx::query("UPDATE stackreleases SET spec=$2 WHERE id=$1")
        .bind(row.try_get::<Uuid, _>("releaseid").map_err(storage)?)
        .bind(spec.to_storage_value().map_err(storage)?)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    sqlx::query("UPDATE stacks SET rowversion=rowversion+1 WHERE id=$1")
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    Ok(Some((
        row.try_get::<i64, _>("rowversion").map_err(storage)? + 1,
        redeploy,
        services,
    )))
}
fn storage(error: impl std::fmt::Display) -> BuildError {
    BuildError::Storage(error.to_string())
}
