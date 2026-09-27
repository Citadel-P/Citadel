use super::*;
use citadel_deployments::{DeploymentUpdateCheck, checkable_deployment_image};

pub(super) async fn begin(
    store: &PostgresDeploymentRepository,
    actor: ActorId,
    administrator: bool,
    expected: &DeploymentDetails,
) -> Result<DeploymentUpdateCheck, DeploymentError> {
    checkable_deployment_image(expected)?;
    let mut tx = store.pool.begin().await.map_err(storage)?;
    ensure_access(
        &mut tx,
        actor,
        administrator,
        expected.id,
        WriteDeployment::REQUIREMENT,
    )
    .await?;
    let lease_id = Uuid::now_v7();
    let changed = sqlx::query("UPDATE deployments SET updatecheckid=$3,controlstate='Processing',controlstartedat=$4,controltriggeredby=$5,rowversion=rowversion+1 WHERE id=$1 AND rowversion=$2 AND controlstate='Idle' AND updatecheckid IS NULL")
        .bind(expected.id).bind(expected.row_version).bind(lease_id).bind(Utc::now().timestamp()).bind(actor.value())
        .execute(&mut *tx).await.map_err(storage)?.rows_affected();
    if changed != 1 {
        return Err(DeploymentError::Conflict(
            "The Deployment changed before the update check.".into(),
        ));
    }
    sqlx::query("SELECT pg_notify($1, '')")
        .bind(citadel_runtime::RuntimeSignal::DeploymentRecovery.channel())
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    tx.commit().await.map_err(storage)?;
    Ok(DeploymentUpdateCheck {
        lease_id,
        deployment: expected.clone(),
    })
}

pub(super) async fn finish(
    store: &PostgresDeploymentRepository,
    claim: &DeploymentUpdateCheck,
    update: Option<&AutoUpdateState>,
) -> Result<(), DeploymentError> {
    let changed = sqlx::query("UPDATE deployments SET updatecheckid=NULL,controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1,autoupdatestate_lastcheckedat=COALESCE($3,autoupdatestate_lastcheckedat),autoupdatestate_status=COALESCE($4,autoupdatestate_status),autoupdatestate_currentdigest=CASE WHEN $4::text IS NULL THEN autoupdatestate_currentdigest ELSE $5 END,autoupdatestate_remotedigest=CASE WHEN $4::text IS NULL THEN autoupdatestate_remotedigest ELSE $6 END,autoupdatestate_lasterror=CASE WHEN $4::text IS NULL THEN autoupdatestate_lasterror ELSE $7 END WHERE id=$1 AND updatecheckid=$2 AND controlstate='Processing'")
        .bind(claim.deployment.id).bind(claim.lease_id).bind(update.map(|v|v.last_checked_at)).bind(update.map(|v|v.status.as_str()))
        .bind(update.and_then(|v|v.current_digest.as_deref())).bind(update.and_then(|v|v.remote_digest.as_deref())).bind(update.and_then(|v|v.last_error.as_deref()))
        .execute(&store.pool).await.map_err(storage)?.rows_affected();
    if changed != 1 {
        return Err(DeploymentError::Conflict(
            "The Deployment update check lease changed.".into(),
        ));
    }
    Ok(())
}

pub(super) async fn recover(
    store: &PostgresDeploymentRepository,
    started_before: i64,
    limit: i64,
) -> Result<Vec<Uuid>, DeploymentError> {
    sqlx::query_scalar("WITH expired AS (SELECT id FROM deployments WHERE updatecheckid IS NOT NULL AND controlstate='Processing' AND controlstartedat<$1 ORDER BY controlstartedat,id LIMIT $2 FOR UPDATE SKIP LOCKED) UPDATE deployments d SET updatecheckid=NULL,controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 FROM expired WHERE d.id=expired.id RETURNING d.id")
        .bind(started_before).bind(limit.clamp(1,100)).fetch_all(&store.pool).await.map_err(storage)
}

pub(super) async fn candidates(
    store: &PostgresDeploymentRepository,
    after: Uuid,
    limit: i64,
) -> Result<Vec<Uuid>, DeploymentError> {
    sqlx::query_scalar("SELECT d.id FROM deployments d JOIN platforms p ON p.id=d.platformid WHERE d.id>$1 AND d.controlstate='Idle' AND p.status='Online' AND p.platformdescriptor->>'$type'='Docker' AND d.spec->>'UpdateBehavior' IN ('Notify','AutoDeploy') AND d.spec->'Image'->>'$type'='External' AND COALESCE(d.spec->'Image'->>'ResolvedDigest','')<>'' AND position('@' in COALESCE(d.spec->'Image'->>'ImageTag',''))=0 ORDER BY d.id LIMIT $2")
        .bind(after).bind(limit.clamp(1,100)).fetch_all(&store.pool).await.map_err(storage)
}

impl PostgresDeploymentRepository {
    pub(super) fn begin_update_check_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a DeploymentDetails,
    ) -> BoxFuture<'a, Result<citadel_deployments::DeploymentUpdateCheck, DeploymentError>> {
        Box::pin(updates::begin(self, actor, administrator, expected))
    }
    pub(super) fn complete_update_check_impl<'a>(
        &'a self,
        claim: &'a citadel_deployments::DeploymentUpdateCheck,
        state: Option<&'a AutoUpdateState>,
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        Box::pin(updates::finish(self, claim, state))
    }
    pub(super) fn recover_update_checks_impl(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, DeploymentError>> {
        Box::pin(updates::recover(self, started_before, limit))
    }
    pub(super) fn scheduled_update_candidates_impl(
        &self,
        after: Uuid,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, DeploymentError>> {
        Box::pin(updates::candidates(self, after, limit))
    }
}
