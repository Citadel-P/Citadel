use super::*;

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn enqueue<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: Option<&'a str>,
        expected: Option<&'a GitRepositoryWebhook>,
        trigger: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query(
                "SELECT defaultbranch,webhook FROM gitrepositories WHERE id=$1 FOR NO KEY UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage)?
            .ok_or(GitRepositoryExecutionError::NotFound)?;
            if let Some(expected) = expected {
                let value: Option<Value> = row.try_get("webhook").map_err(storage)?;
                if value.as_ref().map(map_webhook).transpose()?.as_ref() != Some(expected) {
                    return Err(GitRepositoryExecutionError::Conflict);
                }
            }
            let branch = branch
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(row.try_get("defaultbranch").map_err(storage)?);
            let currently_syncing = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM gitrepositoryrefs WHERE gitrepositoryid=$1 AND branch=$2 AND status='Syncing')",
            )
            .bind(id)
            .bind(branch)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            if currently_syncing {
                sqlx::query("UPDATE gitrepositoryrefs SET synctrigger=$3 WHERE gitrepositoryid=$1 AND branch=$2")
                    .bind(id).bind(branch).bind(trigger)
                    .execute(&mut *transaction).await.map_err(storage)?;
                sqlx::query(
                    "UPDATE gitrepositories SET status='Pending', controlstate='Queued', controltriggeredby=$2, rowversion=rowversion+1 WHERE id=$1",
                )
                .bind(id)
                .bind(actor_id.value())
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            } else {
                upsert_pending_ref(&mut transaction, id, branch, trigger).await?;
                sqlx::query(
                    "UPDATE gitrepositories SET status='Pending', controlstate='Queued', controltriggeredby=$2, controlstartedat=NULL, rowversion=rowversion+1 WHERE id=$1",
                )
                .bind(id)
                .bind(actor_id.value())
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            }
            transaction.commit().await.map_err(storage)
        })
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn get_source_impl<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<GitRepositorySource, GitRepositoryExecutionError>> {
        Box::pin(async move { get_source(&self.pool, id).await })
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn enqueue_apply_impl<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        self.enqueue(actor_id, id, Some(branch), None, "Apply")
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn get_ref_impl<'a>(
        &'a self,
        id: Uuid,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Option<GitRepositoryRef>, GitRepositoryExecutionError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM gitrepositoryrefs WHERE gitrepositoryid=$1 AND branch=$2")
                .bind(id)
                .bind(branch)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .map(map_ref)
                .transpose()
        })
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn list_refs_impl<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<GitRepositoryRef>, GitRepositoryExecutionError>> {
        Box::pin(async move {
            if !repository_exists(&self.pool, id).await? {
                return Err(GitRepositoryExecutionError::NotFound);
            }
            sqlx::query(
                "SELECT * FROM gitrepositoryrefs WHERE gitrepositoryid=$1 ORDER BY branch, id",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(map_ref)
            .collect()
        })
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn resolve_reference_impl<'a>(
        &'a self,
        id: Uuid,
        revision: Option<&'a str>,
    ) -> BoxFuture<'a, Result<String, GitRepositoryExecutionError>> {
        Box::pin(async move {
            let source = get_source(&self.pool, id).await?;
            if let Some(revision) = revision.filter(|value| !value.trim().is_empty())
                && is_object_id(revision)
            {
                return Ok(revision.to_ascii_lowercase());
            }
            let branch = revision.unwrap_or(&source.default_branch);
            sqlx::query_scalar::<_, Option<String>>(
                "SELECT resolvedcommitsha FROM gitrepositoryrefs WHERE gitrepositoryid=$1 AND branch=$2 AND status='Healthy'",
            )
            .bind(id)
            .bind(branch)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .flatten()
            .ok_or(GitRepositoryExecutionError::NotSynchronized)
        })
    }
}

impl PostgresGitRepositoryExecutionPersistence {
    pub(super) fn get_webhook_impl<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<GitRepositoryWebhook>, GitRepositoryExecutionError>> {
        Box::pin(async move {
            let value = sqlx::query_scalar::<_, Option<Value>>(
                "SELECT webhook FROM gitrepositories WHERE id=$1",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .ok_or(GitRepositoryExecutionError::NotFound)?;
            value.as_ref().map(map_webhook).transpose()
        })
    }
}
