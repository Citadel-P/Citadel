use super::*;
use citadel_builds::{
    BuildAdmission, BuildAdmissionStore, BuildExecutorCapacity, BuildExecutorOccupancy,
};

impl BuildAdmissionStore for PostgresBuildRepository {
    fn executor_occupancy<'a>(
        &'a self,
        daemon_id: &'a str,
    ) -> BoxFuture<'a, Result<BuildExecutorOccupancy, BuildError>> {
        Box::pin(async move {
            let row: Option<(Uuid, i64, i64)> = sqlx::query_as("SELECT executor.authorityid,count(reservation.attemptid),count(reservation.attemptid) FILTER(WHERE reservation.uncertain) FROM buildexecutors executor LEFT JOIN buildexecutorreservations reservation ON reservation.daemonid=executor.daemonid WHERE executor.daemonid=$1 GROUP BY executor.daemonid")
                .bind(daemon_id).fetch_optional(&self.pool).await.map_err(storage)?;
            Ok(row
                .map(|(authority, reserved, uncertain)| BuildExecutorOccupancy {
                    authority_id: Some(authority),
                    reserved: reserved as u64,
                    uncertain: uncertain as u64,
                })
                .unwrap_or_default())
        })
    }

    fn record_executor_cleanup<'a>(
        &'a self,
        claim: &'a BuildClaim,
        executor: &'a BuildExecutorCapacity,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let owned: Option<Uuid> = sqlx::query_scalar("SELECT id FROM buildruns WHERE id=$1 AND attemptid=$2 AND status IN ('Preparing','Running') AND EXISTS(SELECT 1 FROM buildprojects p WHERE p.id=buildruns.buildprojectid AND p.currentrunid=buildruns.id) FOR UPDATE")
                .bind(claim.run.id).bind(claim.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            if owned.is_none() {
                return Ok(false);
            }
            let inserted = sqlx::query("INSERT INTO buildexecutorcleanupreceipts(attemptid,runid,daemonid,authorityid,admissiongeneration) SELECT attemptid,runid,daemonid,authorityid,admissiongeneration FROM buildexecutorreservations WHERE runid=$1 AND attemptid=$2 AND daemonid=$3 AND authorityid=$4 AND admissiongeneration=$5 ON CONFLICT(attemptid) DO NOTHING")
                .bind(claim.run.id).bind(claim.attempt_id).bind(&executor.daemon_id).bind(executor.authority_id).bind(executor.admission_generation as i64).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            let confirmed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM buildexecutorcleanupreceipts WHERE runid=$1 AND attemptid=$2 AND daemonid=$3 AND authorityid=$4 AND admissiongeneration=$5)")
                .bind(claim.run.id).bind(claim.attempt_id).bind(&executor.daemon_id).bind(executor.authority_id).bind(executor.admission_generation as i64).fetch_one(&mut *tx).await.map_err(storage)?;
            if inserted > 0 {
                sqlx::query("INSERT INTO buildrunlogs(id,buildrunid,createdat,stream,message) VALUES($1,$2,CURRENT_TIMESTAMP,'system',$3)")
                    .bind(Uuid::now_v7()).bind(claim.run.id).bind(format!("Executor confirmed cleanup for attempt {}. Capacity will be released when the run result is persisted.", claim.attempt_id)).execute(&mut *tx).await.map_err(storage)?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(confirmed)
        })
    }

    fn reserve_executor<'a>(
        &'a self,
        claim: &'a BuildClaim,
        executor: &'a BuildExecutorCapacity,
        commit: &'a str,
    ) -> BoxFuture<'a, Result<BuildAdmission, BuildError>> {
        self.reserve_host(claim, executor, Some(commit), true)
    }

    fn reserve_preparation<'a>(
        &'a self,
        claim: &'a BuildClaim,
        executor: &'a BuildExecutorCapacity,
    ) -> BoxFuture<'a, Result<BuildAdmission, BuildError>> {
        self.reserve_host(
            claim,
            executor,
            claim.run.resolved_commit_sha.as_deref(),
            false,
        )
    }

    fn dispatch_executor<'a>(
        &'a self,
        claim: &'a BuildClaim,
        commit: &'a str,
    ) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let owned = sqlx::query("UPDATE buildruns SET resolvedcommitsha=$3 WHERE id=$1 AND attemptid=$2 AND status='Preparing' AND (resolvedcommitsha IS NULL OR resolvedcommitsha=$3) AND EXISTS(SELECT 1 FROM buildprojects p WHERE p.id=buildruns.buildprojectid AND p.currentrunid=buildruns.id)")
                .bind(claim.run.id).bind(claim.attempt_id).bind(commit).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if owned != 1 || commit.is_empty() {
                return Err(BuildError::Conflict(
                    "Build attempt no longer owns preparation.".into(),
                ));
            }
            let changed = sqlx::query("UPDATE buildexecutorreservations SET dispatchstarted=true WHERE runid=$1 AND attemptid=$2 AND NOT dispatchstarted AND NOT uncertain")
                .bind(claim.run.id).bind(claim.attempt_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed != 1 {
                return Err(BuildError::Conflict(
                    "Build dispatch already started or reservation lost.".into(),
                ));
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}

impl PostgresBuildRepository {
    fn reserve_host<'a>(
        &'a self,
        claim: &'a BuildClaim,
        executor: &'a BuildExecutorCapacity,
        commit: Option<&'a str>,
        dispatched: bool,
    ) -> BoxFuture<'a, Result<BuildAdmission, BuildError>> {
        Box::pin(async move {
            if executor.daemon_id.trim().is_empty()
                || executor.daemon_id.len() > 256
                || executor.authority_id.is_nil()
                || executor.admission_generation == 0
                || executor.admission_generation > i64::MAX as u64
                || !(1..=64).contains(&executor.build_slots)
                || !(1..=64).contains(&executor.heavy_slots)
                || commit.is_some_and(str::is_empty)
            {
                return Err(BuildError::Validation(
                    "Invalid executor capacity identity.".into(),
                ));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let owned: Option<Uuid> = sqlx::query_scalar("SELECT id FROM buildruns WHERE id=$1 AND attemptid=$2 AND status='Preparing' AND (resolvedcommitsha IS NULL OR resolvedcommitsha=$3) AND EXISTS(SELECT 1 FROM buildprojects p WHERE p.id=buildruns.buildprojectid AND p.currentrunid=buildruns.id) FOR UPDATE")
                .bind(claim.run.id).bind(claim.attempt_id).bind(commit).fetch_optional(&mut *tx).await.map_err(storage)?;
            if owned.is_none() {
                return Err(BuildError::Conflict(
                    "Build attempt no longer owns execution.".into(),
                ));
            }
            sqlx::query("INSERT INTO buildexecutors(daemonid,authorityid,buildcapacity) VALUES($1,$2,$3) ON CONFLICT(daemonid) DO NOTHING")
                .bind(&executor.daemon_id).bind(executor.authority_id).bind(executor.build_slots as i32).execute(&mut *tx).await.map_err(storage)?;
            let authority: Uuid = sqlx::query_scalar(
                "SELECT authorityid FROM buildexecutors WHERE daemonid=$1 FOR UPDATE",
            )
            .bind(&executor.daemon_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
            let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM buildexecutorreservations WHERE runid=$1 OR attemptid=$2)")
                .bind(claim.run.id).bind(claim.attempt_id).fetch_one(&mut *tx).await.map_err(storage)?;
            if duplicate {
                return Err(BuildError::Conflict(
                    "Build already has an executor reservation; dispatch cannot be repeated."
                        .into(),
                ));
            }
            let project_pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM buildexecutorreservations reservation JOIN buildruns run ON run.id=reservation.runid WHERE run.buildprojectid=$1)")
                .bind(claim.project.id).fetch_one(&mut *tx).await.map_err(storage)?;
            if project_pending {
                return Ok(BuildAdmission::CleanupPending);
            }
            let (occupied, uncertain): (i64, i64) = sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE uncertain) FROM buildexecutorreservations WHERE daemonid=$1")
                .bind(&executor.daemon_id).fetch_one(&mut *tx).await.map_err(storage)?;
            // Only one process authority may own a daemon while any attempt is
            // reserved. Reconnects retain authority; restarts never erase holds.
            let data: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM dataexecutorreservations WHERE daemonid=$1",
            )
            .bind(&executor.daemon_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
            let automation: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM automationexecutorreservations WHERE daemonid=$1",
            )
            .bind(&executor.daemon_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
            if occupied + data + automation > 0 && authority != executor.authority_id {
                return Ok(BuildAdmission::CleanupPending);
            }
            sqlx::query(
                "UPDATE buildexecutors SET authorityid=$2,buildcapacity=$3,heavycapacity=$4 WHERE daemonid=$1",
            )
            .bind(&executor.daemon_id)
            .bind(executor.authority_id)
            .bind(executor.build_slots as i32)
            .bind(executor.heavy_slots as i32)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
            if occupied >= i64::from(executor.build_slots)
                || occupied + data + automation >= i64::from(executor.heavy_slots)
            {
                tx.commit().await.map_err(storage)?;
                return Ok(if uncertain > 0 {
                    BuildAdmission::CleanupPending
                } else {
                    BuildAdmission::CapacityBusy
                });
            }
            sqlx::query("UPDATE buildruns SET resolvedcommitsha=$3 WHERE id=$1 AND attemptid=$2")
                .bind(claim.run.id)
                .bind(claim.attempt_id)
                .bind(commit)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            sqlx::query("INSERT INTO buildexecutorreservations(attemptid,runid,daemonid,authorityid,admissiongeneration,dispatchstarted) VALUES($1,$2,$3,$4,$5,$6)")
                .bind(claim.attempt_id).bind(claim.run.id).bind(&executor.daemon_id).bind(executor.authority_id).bind(executor.admission_generation as i64).bind(dispatched).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(BuildAdmission::Reserved)
        })
    }
}

/// Called inside the guarded terminal/deferral transaction, never from Drop.
pub(super) async fn settle_reservation(
    tx: &mut Transaction<'_, Postgres>,
    claim: &BuildClaim,
    safe_to_release: bool,
) -> Result<(), BuildError> {
    let confirmed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM buildexecutorcleanupreceipts receipt JOIN buildexecutorreservations reservation USING(attemptid,runid,daemonid,authorityid,admissiongeneration) WHERE receipt.runid=$1 AND receipt.attemptid=$2)")
        .bind(claim.run.id).bind(claim.attempt_id).fetch_one(&mut **tx).await.map_err(storage)?;
    let not_dispatched: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM buildexecutorreservations WHERE runid=$1 AND attemptid=$2 AND NOT dispatchstarted)")
        .bind(claim.run.id).bind(claim.attempt_id).fetch_one(&mut **tx).await.map_err(storage)?;
    if safe_to_release || confirmed || not_dispatched {
        sqlx::query("DELETE FROM buildexecutorreservations WHERE runid=$1 AND attemptid=$2")
            .bind(claim.run.id)
            .bind(claim.attempt_id)
            .execute(&mut **tx)
            .await
            .map_err(storage)?;
    } else {
        let held = sqlx::query(
            "UPDATE buildexecutorreservations SET uncertain=true WHERE runid=$1 AND attemptid=$2",
        )
        .bind(claim.run.id)
        .bind(claim.attempt_id)
        .execute(&mut **tx)
        .await
        .map_err(storage)?
        .rows_affected();
        if held > 0 {
            sqlx::query("UPDATE buildruns SET errorcode='build.cleanup_pending',errormessage=concat_ws(' ',errormessage,'Executor capacity remains reserved until cleanup is verified.') WHERE id=$1 AND attemptid=$2")
                .bind(claim.run.id).bind(claim.attempt_id).execute(&mut **tx).await.map_err(storage)?;
        }
    }
    Ok(())
}
