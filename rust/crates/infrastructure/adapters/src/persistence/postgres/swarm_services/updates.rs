use super::*;
use citadel_swarm_services::{ServiceUpdateCheck, checkable_image};

impl PostgresSwarmServiceRepository {
    pub(super) async fn claim_image_check(
        &self,
        actor: ActorId,
        administrator: bool,
        expected: &SwarmServiceDetails,
    ) -> Result<ServiceUpdateCheck, SwarmServiceError> {
        checkable_image(expected)?;
        let mut tx = self.pool.begin().await.map_err(storage)?;
        ensure_access(
            &mut tx,
            actor,
            administrator,
            expected.id,
            policy::WriteSwarmService::REQUIREMENT,
        )
        .await?;
        let lease_id = Uuid::now_v7();
        let changed = sqlx::query("UPDATE swarmservices SET updatecheckid=$3,controlstate='Processing',controlstartedat=$4,controltriggeredby=$5,rowversion=rowversion+1 WHERE id=$1 AND rowversion=$2 AND controlstate='Idle' AND updatecheckid IS NULL")
            .bind(expected.id).bind(expected.row_version).bind(lease_id).bind(Utc::now().timestamp()).bind(actor.value())
            .execute(&mut *tx).await.map_err(storage)?.rows_affected();
        if changed != 1 {
            return Err(SwarmServiceError::Conflict(
                "The Service changed before the update check could start.".into(),
            ));
        }
        sqlx::query("SELECT pg_notify($1, '')")
            .bind(citadel_runtime::RuntimeSignal::SwarmServiceRecovery.channel())
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        Ok(ServiceUpdateCheck {
            lease_id,
            service: expected.clone(),
        })
    }

    pub(super) async fn finish_image_check(
        &self,
        claim: &ServiceUpdateCheck,
        update: Option<&AutoUpdateState>,
    ) -> Result<(), SwarmServiceError> {
        let changed = sqlx::query("UPDATE swarmservices SET updatecheckid=NULL,controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1,autoupdatestate_lastcheckedat=COALESCE($3,autoupdatestate_lastcheckedat),autoupdatestate_status=COALESCE($4,autoupdatestate_status),autoupdatestate_currentdigest=CASE WHEN $4::text IS NULL THEN autoupdatestate_currentdigest ELSE $5 END,autoupdatestate_remotedigest=CASE WHEN $4::text IS NULL THEN autoupdatestate_remotedigest ELSE $6 END,autoupdatestate_lasterror=CASE WHEN $4::text IS NULL THEN autoupdatestate_lasterror ELSE $7 END WHERE id=$1 AND updatecheckid=$2 AND controlstate='Processing'")
            .bind(claim.service.id).bind(claim.lease_id).bind(update.map(|v|v.last_checked_at)).bind(update.map(|v|v.status.as_str()))
            .bind(update.and_then(|v|v.current_digest.as_deref())).bind(update.and_then(|v|v.remote_digest.as_deref())).bind(update.and_then(|v|v.last_error.as_deref()))
            .execute(&self.pool).await.map_err(storage)?.rows_affected();
        if changed != 1 {
            return Err(SwarmServiceError::Conflict(
                "The update check lease has changed.".into(),
            ));
        }
        Ok(())
    }

    pub(super) async fn recover_image_checks(
        &self,
        started_before: i64,
        limit: i64,
    ) -> Result<Vec<Uuid>, SwarmServiceError> {
        // Check execution is bounded to 30 seconds. Recovery only releases old
        // leases; it neither retries a Docker mutation nor rewrites its history.
        sqlx::query_scalar("WITH expired AS (SELECT id FROM swarmservices WHERE updatecheckid IS NOT NULL AND controlstate='Processing' AND controlstartedat<$1 ORDER BY controlstartedat,id LIMIT $2 FOR UPDATE SKIP LOCKED) UPDATE swarmservices s SET updatecheckid=NULL,controlstate='Idle',controlstartedat=NULL,controltriggeredby=NULL,rowversion=rowversion+1 FROM expired WHERE s.id=expired.id RETURNING s.id")
            .bind(started_before).bind(limit.clamp(1,100)).fetch_all(&self.pool).await.map_err(storage)
    }
}
