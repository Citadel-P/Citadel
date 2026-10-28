use super::*;

impl PostgresBuildRepository {
    /// Only pre-dispatch contention or a structured refusal of the first Agent
    /// phase can enter this path. Neither permits replay after execution starts.
    pub(super) async fn defer_impl(
        &self,
        claim: &BuildClaim,
        result: &BuildExecutionResult,
    ) -> Result<bool, BuildError> {
        // Before preparation there may be no commit yet. Once pinned, every
        // retry must preserve it; the guarded update rejects a changed commit.
        let commit = result
            .resolved_commit_sha
            .as_deref()
            .or(claim.run.resolved_commit_sha.as_deref());
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let delay = 5 + (claim.attempt_id.as_u128() % 11) as i32;
        let state: Option<String> = sqlx::query_scalar(
            "UPDATE buildruns SET status=CASE WHEN queuedat+INTERVAL '24 hours'<=CURRENT_TIMESTAMP THEN 'Failed' ELSE 'Queued' END, completedat=CASE WHEN queuedat+INTERVAL '24 hours'<=CURRENT_TIMESTAMP THEN CURRENT_TIMESTAMP ELSE NULL END, nextattemptat=CURRENT_TIMESTAMP+make_interval(secs=>$4::integer), resolvedcommitsha=$3, imagereferences=$5, errorcode=CASE WHEN queuedat+INTERVAL '24 hours'<=CURRENT_TIMESTAMP THEN 'build.queue_timeout' ELSE 'build.capacity_wait' END, errormessage=CASE WHEN queuedat+INTERVAL '24 hours'<=CURRENT_TIMESTAMP THEN 'Build waited for executor capacity for 24 hours.' ELSE $6 END, startedat=NULL WHERE id=$1 AND attemptid=$2 AND status='Preparing' AND (resolvedcommitsha IS NULL OR resolvedcommitsha=$3) AND EXISTS(SELECT 1 FROM buildprojects p WHERE p.id=buildruns.buildprojectid AND p.currentrunid=buildruns.id) RETURNING status")
            .bind(claim.run.id).bind(claim.attempt_id).bind(commit).bind(delay)
            .bind(serde_json::to_value(&result.image_references).map_err(storage)?)
            .bind(result.error_message.as_deref().unwrap_or("Waiting for executor build capacity."))
            .fetch_optional(&mut *tx).await.map_err(storage)?;
        let Some(state) = state else { return Ok(false) };
        admission::settle_reservation(&mut tx, claim, true).await?;
        let queued = state == "Queued";
        sqlx::query("UPDATE buildprojects SET controlstate=CASE WHEN $3 THEN 'Queued' ELSE 'Idle' END,currentrunid=CASE WHEN $3 THEN $2 ELSE NULL END,controlstartedat=NULL,rowversion=rowversion+1,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND currentrunid=$2")
            .bind(claim.project.id).bind(claim.run.id).bind(queued).execute(&mut *tx).await.map_err(storage)?;
        // Do not emit a new Queued activity on every busy response.
        if !queued {
            record_run_activity(&mut tx, claim.run.id).await?;
        }
        tx.commit().await.map_err(storage)?;
        Ok(true)
    }
}
