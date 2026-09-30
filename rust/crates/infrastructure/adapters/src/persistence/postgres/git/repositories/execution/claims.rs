use super::*;
use citadel_primitives::WebhookConfig;

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn enqueue_sync_impl<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        self.enqueue(actor_id, id, branch, None, "Manual")
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn enqueue_webhook_impl<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: &'a str,
        expected: &'a WebhookConfig,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        self.enqueue(actor_id, id, Some(branch), Some(expected), "Webhook")
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn enqueue_due_impl<'a>(
        &'a self,
        limit: usize,
    ) -> BoxFuture<'a, Result<usize, GitRepositoryExecutionError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let rows = sqlx::query(
                r#"
SELECT repository.id, branches.branch, repository.createdbyactorid
FROM gitrepositories repository
CROSS JOIN LATERAL (
    SELECT repository.defaultbranch AS branch
    UNION
    SELECT COALESCE(NULLIF(release.spec->>'Branch',''),repository.defaultbranch)
    FROM stacks stack JOIN stackreleases release ON release.id=stack.currentstackreleaseid
    WHERE stack.stacksource='Git' AND release.spec->>'GitRepoId'=repository.id::text
      AND COALESCE(release.spec->>'CommitSha','')=''
) branches
LEFT JOIN gitrepositoryrefs reference
  ON reference.gitrepositoryid=repository.id
 AND reference.branch=branches.branch
WHERE branches.branch<>'' AND repository.syncmode='PullInterval'
  AND repository.controlstate='Idle'
  AND (
    reference.id IS NULL OR
    reference.lastsyncedat <= CURRENT_TIMESTAMP - make_interval(mins => COALESCE(repository.syncintervalminutes,5))
  )
ORDER BY COALESCE(reference.lastsyncedat, repository.createdat), repository.id
FOR NO KEY UPDATE OF repository SKIP LOCKED
LIMIT $1
"#,
            )
            .bind(i64::try_from(limit).unwrap_or(100))
            .fetch_all(&mut *transaction)
            .await
            .map_err(storage)?;
            for row in &rows {
                let id: Uuid = row.try_get("id").map_err(storage)?;
                let branch: String = row.try_get("branch").map_err(storage)?;
                let actor_id: Uuid = row.try_get("createdbyactorid").map_err(storage)?;
                upsert_pending_ref(&mut transaction, id, &branch, "Poll").await?;
                sqlx::query(
                    "UPDATE gitrepositories SET status='Pending', controlstate='Queued', controltriggeredby=$2, controlstartedat=NULL, rowversion=rowversion+1 WHERE id=$1",
                )
                .bind(id)
                .bind(actor_id)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            }
            if !rows.is_empty() {
                sqlx::query("SELECT pg_notify('citadel_git_work','')")
                    .execute(&mut *transaction)
                    .await
                    .map_err(storage)?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(rows.len())
        })
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn claim_next_impl<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<GitSyncClaim>, GitRepositoryExecutionError>> {
        Box::pin(async move {
            self.recover_stale(stale_before).await?;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query(
                r#"
SELECT reference.id AS referenceid, reference.branch, reference.synctrigger, reference.resolvedcommitsha, reference.lasterror,
       repository.id, repository.name, repository.url, repository.defaultbranch,
       repository.gitaccountid, repository.syncmode, repository.syncintervalminutes,
       COALESCE(repository.controltriggeredby, repository.createdbyactorid) AS actorid
FROM gitrepositoryrefs reference
JOIN gitrepositories repository ON repository.id=reference.gitrepositoryid
WHERE reference.status='Pending' AND repository.controlstate='Queued'
  AND NOT EXISTS (
    SELECT 1 FROM gitrepositoryrefs active
    WHERE active.gitrepositoryid=repository.id AND active.status='Syncing'
  )
ORDER BY reference.lastsyncedat, reference.id
FOR NO KEY UPDATE OF repository, reference SKIP LOCKED
LIMIT 1
"#,
            )
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage)?;
            let Some(row) = row else {
                transaction.commit().await.map_err(storage)?;
                return Ok(None);
            };
            let reference_id: Uuid = row.try_get("referenceid").map_err(storage)?;
            let repository_id: Uuid = row.try_get("id").map_err(storage)?;
            let started_at = Utc::now();
            sqlx::query(
                "UPDATE gitrepositoryrefs SET status='Syncing', lastsyncedat=$2 WHERE id=$1",
            )
            .bind(reference_id)
            .bind(started_at)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            sqlx::query(
                "UPDATE gitrepositories SET controlstate='Processing', status='Pending', controlstartedat=$2, rowversion=rowversion+1 WHERE id=$1",
            )
            .bind(repository_id)
            .bind(started_at.timestamp())
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            let claim = GitSyncClaim {
                trigger: row.try_get("synctrigger").map_err(storage)?,
                previous_commit: row.try_get("resolvedcommitsha").map_err(storage)?,
                previous_error: row.try_get("lasterror").map_err(storage)?,
                repository: map_source(&row)?,
                reference_id,
                branch: row.try_get("branch").map_err(storage)?,
                actor_id: ActorId::new(row.try_get("actorid").map_err(storage)?),
            };
            transaction.commit().await.map_err(storage)?;
            Ok(Some(claim))
        })
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn complete_impl<'a>(
        &'a self,
        claim: &'a GitSyncClaim,
        result: &'a SyncResult,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        Box::pin(async move { finish_sync(&self.pool, claim, Some(result), None).await })
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn fail_impl<'a>(
        &'a self,
        claim: &'a GitSyncClaim,
        message: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        Box::pin(async move { finish_sync(&self.pool, claim, None, Some(message)).await })
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    // Recovery is bounded independently of claim throughput. A failed recovery
    // is retried, and clones of this repository share the same clock.
    async fn recover_stale(
        &self,
        stale_before: DateTime<Utc>,
    ) -> Result<(), GitRepositoryExecutionError> {
        let mut last = self.last_recovery.lock().await;
        if last.is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(60)) {
            return Ok(());
        }
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        sqlx::query(
                r#"
WITH candidates AS MATERIALIZED (
  SELECT repository.id FROM gitrepositories repository
  WHERE EXISTS (SELECT 1 FROM gitrepositoryrefs reference
                WHERE reference.gitrepositoryid=repository.id AND reference.status='Syncing' AND reference.lastsyncedat<$1)
  ORDER BY repository.id FOR NO KEY UPDATE SKIP LOCKED LIMIT 100
), stale AS (
  UPDATE gitrepositoryrefs reference
  SET status='Pending', lasterror='Previous synchronization was interrupted.'
  FROM candidates WHERE reference.gitrepositoryid=candidates.id
    AND reference.status='Syncing' AND reference.lastsyncedat<$1
  RETURNING reference.gitrepositoryid
)
UPDATE gitrepositories
SET controlstate='Queued', status='Pending', controlstartedat=NULL, rowversion=rowversion+1
WHERE id IN (SELECT gitrepositoryid FROM stale)
"#,
            )
            .bind(stale_before)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
        transaction.commit().await.map_err(storage)?;
        *last = Some(tokio::time::Instant::now());
        Ok(())
    }
}
