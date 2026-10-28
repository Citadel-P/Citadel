use super::*;
use citadel_builds::{BuildCleanupAttempt, BuildCleanupStore, BuildRetainedReservation};

impl BuildCleanupStore for PostgresBuildRepository {
    fn retained_reservation(
        &self,
        run_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<BuildRetainedReservation>, BuildError>> {
        Box::pin(async move {
            sqlx::query("SELECT reservation.*, run.platformsnapshot FROM buildexecutorreservations reservation JOIN buildruns run ON run.id=reservation.runid AND run.attemptid=reservation.attemptid WHERE run.id=$1 AND run.status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted')")
                .bind(run_id).fetch_optional(&self.pool).await.map_err(storage)?
                .map(|row| Ok(BuildRetainedReservation {
                    run_id,
                    attempt_id: row.try_get("attemptid").map_err(storage)?,
                    target: serde_json::from_value(row.try_get("platformsnapshot").map_err(storage)?).map_err(storage)?,
                    daemon_id: row.try_get("daemonid").map_err(storage)?,
                    authority_id: row.try_get("authorityid").map_err(storage)?,
                })).transpose()
        })
    }

    fn resolve_reservation<'a>(
        &'a self,
        reservation: &'a BuildRetainedReservation,
        actor: ActorId,
        reason: &'a str,
        replacement_authority: Uuid,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        Box::pin(async move {
            if replacement_authority.is_nil() || replacement_authority == reservation.authority_id {
                return Err(BuildError::Conflict(
                    "Restart the original Agent before resolving cleanup.".into(),
                ));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            crate::persistence::postgres::identity::users::repository::lock_identity_mutations(
                &mut tx,
            )
            .await
            .map_err(storage)?;
            let authorized: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users u JOIN actors a ON a.id=u.actorid WHERE a.id=$1 AND a.isenabled AND a.type='User' AND EXISTS(SELECT 1 FROM actorroles ar WHERE ar.roleid=$2 AND (ar.actorid=a.id OR EXISTS(SELECT 1 FROM actorteammemberships m JOIN teams t ON t.id=m.teamid JOIN actors ta ON ta.id=t.actorid WHERE m.memberactorid=a.id AND t.actorid=ar.actorid AND ta.isenabled))))")
                .bind(actor.value()).bind(citadel_identity::ADMIN_ROLE_ID).fetch_one(&mut *tx).await.map_err(storage)?;
            if !authorized {
                return Err(BuildError::Forbidden);
            }
            let row = sqlx::query("SELECT buildprojectid, projectnamesnapshot, errormessage FROM buildruns WHERE id=$1 AND attemptid=$2 AND status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted') FOR UPDATE")
                .bind(reservation.run_id).bind(reservation.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            let Some(row) = row else {
                return Ok(false);
            };
            let removed = sqlx::query("DELETE FROM buildexecutorreservations WHERE runid=$1 AND attemptid=$2 AND daemonid=$3 AND authorityid=$4")
                .bind(reservation.run_id).bind(reservation.attempt_id).bind(&reservation.daemon_id).bind(reservation.authority_id).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if removed == 0 {
                return Ok(false);
            }
            // Operator acknowledgment is not authenticated Agent cleanup evidence: never insert a receipt.
            let activity = citadel_activities::ActivityEvent::new_build_event(
                row.try_get("buildprojectid").map_err(storage)?,
                row.try_get("projectnamesnapshot").map_err(storage)?,
                actor,
                citadel_activities::ActivityEventInfo::BuildRunCleanupResolved {
                    run_id: reservation.run_id,
                    attempt_id: reservation.attempt_id,
                    daemon_id: reservation.daemon_id.clone(),
                    previous_authority_id: reservation.authority_id,
                    replacement_authority_id: replacement_authority,
                    reason: reason.to_owned(),
                },
                Utc::now(),
            )
            .map_err(storage)?;
            crate::persistence::postgres::activities::store::insert_activity(&mut tx, &activity)
                .await
                .map_err(storage)?;
            let original_error: Option<String> = row.try_get("errormessage").map_err(storage)?;
            let message = format!(
                "Administrator {} acknowledged that attempt {} and its pushes stopped, and resolved retained capacity after Agent restart ({} -> {}) on daemon {}. Reason: {}. The run outcome is unchanged; execution was not retried.{}",
                actor.value(),
                reservation.attempt_id,
                reservation.authority_id,
                replacement_authority,
                reservation.daemon_id,
                reason,
                original_error
                    .map(|error| format!(" Original result: {error}"))
                    .unwrap_or_default()
            );
            sqlx::query("INSERT INTO buildrunlogs(id,buildrunid,createdat,stream,message) VALUES($1,$2,CURRENT_TIMESTAMP,'system',$3)")
                .bind(Uuid::now_v7()).bind(reservation.run_id).bind(message).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("UPDATE buildruns SET errorcode='build.cleanup_resolved',errormessage='An administrator resolved retained capacity; execution was not retried. See run logs for the original failure.' WHERE id=$1 AND errorcode='build.cleanup_pending'")
                .bind(reservation.run_id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn pending_cleanup(
        &self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BuildCleanupAttempt>, BuildError>> {
        Box::pin(async move {
            sqlx::query("SELECT reservation.*, run.platformsnapshot, EXISTS(SELECT 1 FROM buildexecutorcleanupreceipts receipt WHERE receipt.attemptid=reservation.attemptid AND receipt.runid=reservation.runid AND receipt.daemonid=reservation.daemonid AND receipt.authorityid=reservation.authorityid AND receipt.admissiongeneration=reservation.admissiongeneration) AS receipt_recorded FROM buildexecutorreservations reservation JOIN buildruns run ON run.id=reservation.runid AND run.attemptid=reservation.attemptid WHERE run.status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted') AND reservation.admissiongeneration IS NOT NULL AND ($1::uuid IS NULL OR reservation.attemptid>$1) ORDER BY reservation.attemptid LIMIT $2")
                .bind(after).bind(limit.clamp(1, 16) as i64).fetch_all(&self.pool).await.map_err(storage)?
                .into_iter().map(|row| Ok(BuildCleanupAttempt {
                    run_id: row.try_get("runid").map_err(storage)?,
                    attempt_id: row.try_get("attemptid").map_err(storage)?,
                    target: serde_json::from_value(row.try_get("platformsnapshot").map_err(storage)?).map_err(storage)?,
                    daemon_id: row.try_get("daemonid").map_err(storage)?,
                    authority_id: row.try_get("authorityid").map_err(storage)?,
                    admission_generation: row.try_get::<i64,_>("admissiongeneration").map_err(storage)? as u64,
                    receipt_recorded: row.try_get("receipt_recorded").map_err(storage)?,
                })).collect()
        })
    }

    fn finish_cleanup<'a>(
        &'a self,
        attempt: &'a BuildCleanupAttempt,
    ) -> BoxFuture<'a, Result<bool, BuildError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            // Same lock order as completion/recovery: run first, then reservation.
            let run: Option<(Option<String>,)> = sqlx::query_as("SELECT errormessage FROM buildruns WHERE id=$1 AND attemptid=$2 AND status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted') FOR UPDATE")
                .bind(attempt.run_id).bind(attempt.attempt_id).fetch_optional(&mut *tx).await.map_err(storage)?;
            let Some((original_error,)) = run else {
                return Ok(false);
            };
            let removed = sqlx::query("DELETE FROM buildexecutorreservations WHERE runid=$1 AND attemptid=$2 AND daemonid=$3 AND authorityid=$4 AND admissiongeneration=$5")
                .bind(attempt.run_id).bind(attempt.attempt_id).bind(&attempt.daemon_id).bind(attempt.authority_id).bind(attempt.admission_generation as i64).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if removed == 0 {
                return Ok(false);
            }
            sqlx::query("INSERT INTO buildexecutorcleanupreceipts(attemptid,runid,daemonid,authorityid,admissiongeneration) VALUES($1,$2,$3,$4,$5) ON CONFLICT(attemptid) DO NOTHING")
                .bind(attempt.attempt_id).bind(attempt.run_id).bind(&attempt.daemon_id).bind(attempt.authority_id).bind(attempt.admission_generation as i64).execute(&mut *tx).await.map_err(storage)?;
            let evidence: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM buildexecutorcleanupreceipts WHERE runid=$1 AND attemptid=$2 AND daemonid=$3 AND authorityid=$4 AND admissiongeneration=$5)")
                .bind(attempt.run_id).bind(attempt.attempt_id).bind(&attempt.daemon_id).bind(attempt.authority_id).bind(attempt.admission_generation as i64).fetch_one(&mut *tx).await.map_err(storage)?;
            if !evidence {
                return Err(BuildError::Conflict(
                    "Build cleanup evidence does not match the reservation.".into(),
                ));
            }
            let message = format!(
                "Executor cleanup verified for attempt {}. Capacity released; the run outcome is unchanged and execution was not retried.{}",
                attempt.attempt_id,
                original_error
                    .map(|error| format!(" Original result: {error}"))
                    .unwrap_or_default()
            );
            sqlx::query("INSERT INTO buildrunlogs(id,buildrunid,createdat,stream,message) VALUES($1,$2,CURRENT_TIMESTAMP,'system',$3)")
                .bind(Uuid::now_v7()).bind(attempt.run_id).bind(message).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("UPDATE buildruns SET errorcode='build.cleanup_confirmed',errormessage='Executor cleanup verified; execution was not retried. See run logs for the original failure.' WHERE id=$1 AND errorcode='build.cleanup_pending'")
                .bind(attempt.run_id).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(true)
        })
    }
}
