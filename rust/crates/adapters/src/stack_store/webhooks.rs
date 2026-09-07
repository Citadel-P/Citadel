use super::*;
use citadel_stacks::{StackWebhookJob, can_queue_stack_webhook, stack_webhook_fingerprint};

impl PostgresStackStore {
    pub(super) async fn enqueue_stack_webhook(
        &self,
        expected: &StackView,
        commit: Option<&str>,
    ) -> Result<(), StackError> {
        if commit.is_some_and(|sha| {
            !matches!(sha.len(), 40 | 64) || !sha.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err(StackError::Validation(
                "Webhook commit must be a full commit ID.".into(),
            ));
        }
        let expected_spec = expected.spec.as_ref().ok_or(StackError::NotFound)?;
        if !can_queue_stack_webhook(expected_spec) {
            return Err(StackError::Conflict(
                "Stack webhook configuration changed.".into(),
            ));
        }
        let fingerprint = stack_webhook_fingerprint(expected_spec)?;
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let row = sqlx::query("SELECT s.currentstackreleaseid,r.spec FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1 FOR UPDATE OF s,r")
            .bind(expected.id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(StackError::NotFound)?;
        let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
        if row
            .try_get::<Uuid, _>("currentstackreleaseid")
            .map_err(storage)?
            != expected.current_stack_release_id
            || stack_webhook_fingerprint(&spec)? != fingerprint
        {
            return Err(StackError::Conflict(
                "Stack webhook configuration changed.".into(),
            ));
        }
        let StackSpec::Git {
            git_repo_id,
            branch,
            ..
        } = &spec
        else {
            return Err(StackError::NotFound);
        };
        let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stackwebhookdeployqueue WHERE stackid=$1 AND expectedspecfingerprint=$2 AND dispatchedcommitsha IS NOT DISTINCT FROM $3 AND status IN ('Queued','Processing'))")
            .bind(expected.id).bind(&fingerprint).bind(commit).fetch_one(&mut *tx).await.map_err(storage)?;
        if !duplicate {
            // Coalesce pending pushes for this Stack; never replace an executing claim.
            sqlx::query("DELETE FROM stackwebhookdeployqueue WHERE stackid=$1 AND status='Queued'")
                .bind(expected.id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            sqlx::query("INSERT INTO stackwebhookdeployqueue(id,stackid,gitrepositoryid,expectedstackreleaseid,expectedspecfingerprint,branch,dispatchedcommitsha,attempts,queuedat,availableat,status) VALUES($1,$2,$3,$4,$5,$6,$7,0,now(),now(),'Queued')")
                .bind(Uuid::now_v7()).bind(expected.id).bind(git_repo_id).bind(expected.current_stack_release_id)
                .bind(fingerprint).bind(branch).bind(commit).execute(&mut *tx).await.map_err(storage)?;
        }
        tx.commit().await.map_err(storage)
    }
}

pub(super) async fn claim(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    stack_id: Uuid,
    release_id: Uuid,
    spec: &StackSpec,
) -> Result<(), StackError> {
    let row = sqlx::query("SELECT * FROM stackwebhookdeployqueue WHERE id=$1 AND stackid=$2 AND status='Queued' AND availableat<=now() FOR UPDATE")
        .bind(id).bind(stack_id).fetch_optional(&mut **tx).await.map_err(storage)?.ok_or(StackError::Conflict("Webhook is no longer queued.".into()))?;
    if row
        .try_get::<Uuid, _>("expectedstackreleaseid")
        .map_err(storage)?
        != release_id
        || !can_queue_stack_webhook(spec)
        || row
            .try_get::<String, _>("expectedspecfingerprint")
            .map_err(storage)?
            != stack_webhook_fingerprint(spec)?
    {
        return Err(StackError::NotFound);
    }
    if let Some(commit) = row
        .try_get::<Option<String>, _>("dispatchedcommitsha")
        .map_err(storage)?
    {
        let applied: bool = sqlx::query_scalar("SELECT lower(COALESCE(source->>'ResolvedCommitSha',source->>'resolvedCommitSha',''))=lower($2) FROM stackreleases WHERE id=$1")
            .bind(release_id).bind(commit).fetch_one(&mut **tx).await.map_err(storage)?;
        if applied {
            return Err(StackError::NotFound);
        }
    }
    sqlx::query("UPDATE stackwebhookdeployqueue SET status='Processing',startedat=now(),attempts=attempts+1 WHERE id=$1")
        .bind(id).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}

pub(super) async fn settle(
    tx: &mut Transaction<'_, Postgres>,
    claim: &StackOperationClaim,
    failure: Option<&str>,
) -> Result<(), StackError> {
    if failure.is_some() {
        // Failed execution is already recorded as a Stack activity. Never blindly
        // retry an unknown outcome; reconciliation calls this once it is known.
        sqlx::query("UPDATE stackwebhookdeployqueue q SET status='Queued',startedat=NULL,availableat=now()+make_interval(secs => 5 * power(2,attempts-1)::integer),lasterror=$3 WHERE stackid=$1 AND expectedstackreleaseid=$2 AND status='Processing' AND attempts<3 AND NOT EXISTS(SELECT 1 FROM stackwebhookdeployqueue next WHERE next.stackid=q.stackid AND next.status='Queued')")
            .bind(claim.stack_id).bind(claim.release_id).bind(failure).execute(&mut **tx).await.map_err(storage)?;
    }
    sqlx::query("DELETE FROM stackwebhookdeployqueue WHERE stackid=$1 AND expectedstackreleaseid=$2 AND status='Processing'")
        .bind(claim.stack_id).bind(claim.release_id).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}

pub(super) fn ready(
    pool: &PgPool,
    limit: i64,
) -> BoxFuture<'_, Result<Vec<StackWebhookJob>, StackError>> {
    Box::pin(async move {
        sqlx::query("SELECT q.id,q.stackid FROM stackwebhookdeployqueue q JOIN stacks s ON s.id=q.stackid WHERE q.status='Queued' AND q.availableat<=now() AND s.controlstate='Idle' ORDER BY q.availableat,q.queuedat,q.id LIMIT $1")
            .bind(limit.clamp(1,100)).fetch_all(pool).await.map_err(storage)?.into_iter()
            .map(|row|Ok(StackWebhookJob{id:row.try_get("id").map_err(storage)?,stack_id:row.try_get("stackid").map_err(storage)?})).collect()
    })
}
