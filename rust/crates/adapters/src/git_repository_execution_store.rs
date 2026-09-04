use chrono::{DateTime, Utc};
use citadel_domain::{
    ActivityEvent, ActivityEventInfo, ActorId, GitRepositoryActivitySnapshot,
    GitRepositorySyncActivitySnapshot,
};
use citadel_git::{
    GitRepositoryExecutionError, GitRepositoryExecutionStore, GitRepositoryRefView,
    GitRepositorySource, GitRepositorySyncMode, GitRepositoryWebhook, GitSyncClaim, SyncResult,
};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::insert_activity;

#[derive(Clone)]
pub struct PostgresGitRepositoryExecutionStore {
    pool: PgPool,
}

impl PostgresGitRepositoryExecutionStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl GitRepositoryExecutionStore for PostgresGitRepositoryExecutionStore {
    fn get_source<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<GitRepositorySource, GitRepositoryExecutionError>> {
        Box::pin(async move { get_source(&self.pool, id).await })
    }

    fn enqueue_sync<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let row =
                sqlx::query("SELECT defaultbranch FROM gitrepositories WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(storage)?
                    .ok_or(GitRepositoryExecutionError::NotFound)?;
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
                sqlx::query(
                    "UPDATE gitrepositories SET status='Pending', controlstate='Queued', controltriggeredby=$2, rowversion=rowversion+1 WHERE id=$1",
                )
                .bind(id)
                .bind(actor_id.value())
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            } else {
                upsert_pending_ref(&mut transaction, id, branch).await?;
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

    fn enqueue_due<'a>(
        &'a self,
        limit: usize,
    ) -> BoxFuture<'a, Result<usize, GitRepositoryExecutionError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let rows = sqlx::query(
                r#"
SELECT repository.id, repository.defaultbranch, repository.createdbyactorid
FROM gitrepositories repository
LEFT JOIN gitrepositoryrefs reference
  ON reference.gitrepositoryid=repository.id
 AND reference.branch=repository.defaultbranch
WHERE repository.syncmode='PullInterval'
  AND repository.controlstate='Idle'
  AND (
    reference.id IS NULL OR
    reference.lastsyncedat <= CURRENT_TIMESTAMP - make_interval(mins => repository.syncintervalminutes)
  )
ORDER BY COALESCE(reference.lastsyncedat, repository.createdat), repository.id
FOR UPDATE OF repository SKIP LOCKED
LIMIT $1
"#,
            )
            .bind(i64::try_from(limit).unwrap_or(100))
            .fetch_all(&mut *transaction)
            .await
            .map_err(storage)?;
            for row in &rows {
                let id: Uuid = row.try_get("id").map_err(storage)?;
                let branch: String = row.try_get("defaultbranch").map_err(storage)?;
                let actor_id: Uuid = row.try_get("createdbyactorid").map_err(storage)?;
                upsert_pending_ref(&mut transaction, id, &branch).await?;
                sqlx::query(
                    "UPDATE gitrepositories SET status='Pending', controlstate='Queued', controltriggeredby=$2, controlstartedat=NULL, rowversion=rowversion+1 WHERE id=$1",
                )
                .bind(id)
                .bind(actor_id)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(rows.len())
        })
    }

    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<GitSyncClaim>, GitRepositoryExecutionError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            sqlx::query(
                r#"
WITH stale AS (
  UPDATE gitrepositoryrefs
  SET status='Pending', lasterror='Previous synchronization was interrupted.'
  WHERE status='Syncing' AND lastsyncedat < $1
  RETURNING gitrepositoryid
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
            let row = sqlx::query(
                r#"
SELECT reference.id AS referenceid, reference.branch,
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
FOR UPDATE OF reference, repository SKIP LOCKED
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
                "UPDATE gitrepositoryrefs SET status='Syncing', lasterror=NULL, lastsyncedat=$2 WHERE id=$1",
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
                repository: map_source(&row)?,
                reference_id,
                branch: row.try_get("branch").map_err(storage)?,
                actor_id: ActorId::new(row.try_get("actorid").map_err(storage)?),
            };
            transaction.commit().await.map_err(storage)?;
            Ok(Some(claim))
        })
    }

    fn complete<'a>(
        &'a self,
        claim: &'a GitSyncClaim,
        result: &'a SyncResult,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        Box::pin(async move { finish_sync(&self.pool, claim, Some(result), None).await })
    }

    fn fail<'a>(
        &'a self,
        claim: &'a GitSyncClaim,
        message: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        Box::pin(async move { finish_sync(&self.pool, claim, None, Some(message)).await })
    }

    fn list_refs<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<GitRepositoryRefView>, GitRepositoryExecutionError>> {
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

    fn resolve_reference<'a>(
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

    fn get_webhook<'a>(
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

fn map_webhook(value: &Value) -> Result<GitRepositoryWebhook, GitRepositoryExecutionError> {
    let field = |name: &str| {
        value.as_object().and_then(|object| {
            object
                .iter()
                .find_map(|(key, value)| key.eq_ignore_ascii_case(name).then_some(value))
        })
    };
    let enabled = field("enabled").and_then(Value::as_bool).unwrap_or(false);
    let provider = field("provider")
        .and_then(Value::as_str)
        .unwrap_or("GitHub")
        .to_owned();
    let auth_scheme = field("authScheme")
        .and_then(Value::as_str)
        .unwrap_or("GitHubHmacSha256")
        .to_owned();
    Ok(GitRepositoryWebhook {
        enabled,
        provider,
        auth_scheme,
        secret: field("secret").and_then(Value::as_str).map(str::to_owned),
        branch_filter: field("branchFilter")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

async fn get_source(
    pool: &PgPool,
    id: Uuid,
) -> Result<GitRepositorySource, GitRepositoryExecutionError> {
    sqlx::query(
        "SELECT id,name,url,defaultbranch,gitaccountid,syncmode,syncintervalminutes FROM gitrepositories WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?
    .ok_or(GitRepositoryExecutionError::NotFound)
    .and_then(|row| map_source(&row))
}

fn map_source(
    row: &sqlx::postgres::PgRow,
) -> Result<GitRepositorySource, GitRepositoryExecutionError> {
    let sync_mode: String = row.try_get("syncmode").map_err(storage)?;
    Ok(GitRepositorySource {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        url: row.try_get("url").map_err(storage)?,
        default_branch: row.try_get("defaultbranch").map_err(storage)?,
        git_account_id: row.try_get("gitaccountid").map_err(storage)?,
        sync_mode: match sync_mode.as_str() {
            "Manual" => GitRepositorySyncMode::Manual,
            "PullInterval" => GitRepositorySyncMode::PullInterval,
            _ => {
                return Err(GitRepositoryExecutionError::Storage(format!(
                    "unknown Git synchronization mode '{sync_mode}'"
                )));
            }
        },
        sync_interval_minutes: row.try_get("syncintervalminutes").map_err(storage)?,
    })
}

fn map_ref(
    row: sqlx::postgres::PgRow,
) -> Result<GitRepositoryRefView, GitRepositoryExecutionError> {
    Ok(GitRepositoryRefView {
        id: row.try_get("id").map_err(storage)?,
        git_repository_id: row.try_get("gitrepositoryid").map_err(storage)?,
        branch: row.try_get("branch").map_err(storage)?,
        resolved_commit_sha: row.try_get("resolvedcommitsha").map_err(storage)?,
        status: row.try_get("status").map_err(storage)?,
        last_error: row.try_get("lasterror").map_err(storage)?,
        last_synced_at: row.try_get("lastsyncedat").map_err(storage)?,
    })
}

async fn upsert_pending_ref(
    transaction: &mut Transaction<'_, Postgres>,
    repository_id: Uuid,
    branch: &str,
) -> Result<(), GitRepositoryExecutionError> {
    sqlx::query(
        r#"
INSERT INTO gitrepositoryrefs(id,gitrepositoryid,branch,resolvedcommitsha,status,lasterror,lastsyncedat)
VALUES($1,$2,$3,NULL,'Pending',NULL,CURRENT_TIMESTAMP)
ON CONFLICT(gitrepositoryid,branch) DO UPDATE
SET status='Pending', lasterror=NULL
"#,
    )
    .bind(Uuid::now_v7())
    .bind(repository_id)
    .bind(branch)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

async fn finish_sync(
    pool: &PgPool,
    claim: &GitSyncClaim,
    result: Option<&SyncResult>,
    error: Option<&str>,
) -> Result<(), GitRepositoryExecutionError> {
    let mut transaction = pool.begin().await.map_err(storage)?;
    let (status, commit) = if let Some(result) = result {
        ("Healthy", Some(result.commit.as_str()))
    } else {
        ("Degraded", None)
    };
    let affected = sqlx::query(
        "UPDATE gitrepositoryrefs SET status=$2,resolvedcommitsha=COALESCE($3,resolvedcommitsha),lasterror=$4,lastsyncedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Syncing'",
    )
    .bind(claim.reference_id)
    .bind(status)
    .bind(commit)
    .bind(error)
    .execute(&mut *transaction)
    .await
    .map_err(storage)?
    .rows_affected();
    if affected != 1 {
        return Err(GitRepositoryExecutionError::Conflict);
    }
    let rerun_requested = sqlx::query_scalar::<_, bool>(
        "SELECT controlstate='Queued' FROM gitrepositories WHERE id=$1 FOR UPDATE",
    )
    .bind(claim.repository.id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(storage)?;
    if rerun_requested {
        sqlx::query("UPDATE gitrepositoryrefs SET status='Pending',lasterror=NULL WHERE id=$1")
            .bind(claim.reference_id)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
    }
    let pending = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM gitrepositoryrefs WHERE gitrepositoryid=$1 AND status='Pending')",
    )
    .bind(claim.repository.id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(storage)?;
    sqlx::query(
        "UPDATE gitrepositories SET status=$2,controlstate=$3,controlstartedat=NULL,controltriggeredby=CASE WHEN $4 THEN controltriggeredby ELSE NULL END,rowversion=rowversion+1 WHERE id=$1",
    )
    .bind(claim.repository.id)
    .bind(if pending { "Pending" } else { status })
    .bind(if pending { "Queued" } else { "Idle" })
    .bind(pending)
    .execute(&mut *transaction)
    .await
    .map_err(storage)?;
    let activity_result = GitRepositorySyncActivitySnapshot {
        commit_sha: commit.map(str::to_owned),
        message: error.map(str::to_owned),
    };
    let snapshot = repository_snapshot(&mut transaction, &claim.repository, commit).await?;
    let info = ActivityEventInfo::git_repo_synchronized(
        snapshot,
        activity_result,
        result.is_some_and(|result| result.cloned),
    );
    let activity = ActivityEvent::new_git_repository_sync_event(
        claim.repository.id,
        claim.repository.name.clone(),
        claim.actor_id,
        info,
        result.is_some(),
        Utc::now(),
    )
    .map_err(|error| GitRepositoryExecutionError::Storage(error.to_string()))?;
    insert_activity(&mut transaction, &activity)
        .await
        .map_err(|error| GitRepositoryExecutionError::Storage(error.to_string()))?;
    transaction.commit().await.map_err(storage)
}

async fn repository_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    source: &GitRepositorySource,
    commit: Option<&str>,
) -> Result<GitRepositoryActivitySnapshot, GitRepositoryExecutionError> {
    let row =
        sqlx::query("SELECT description,webhook,onclone,onpull FROM gitrepositories WHERE id=$1")
            .bind(source.id)
            .fetch_one(&mut **transaction)
            .await
            .map_err(storage)?;
    Ok(GitRepositoryActivitySnapshot {
        id: source.id,
        name: source.name.clone(),
        description: row.try_get("description").map_err(storage)?,
        url: source.url.clone(),
        default_branch: source.default_branch.clone(),
        git_account_id: source.git_account_id,
        sync_mode: match source.sync_mode {
            GitRepositorySyncMode::Manual => "Manual",
            GitRepositorySyncMode::PullInterval => "PullInterval",
        }
        .to_owned(),
        sync_interval_minutes: source.sync_interval_minutes,
        webhook: mask_webhook(row.try_get("webhook").map_err(storage)?),
        on_clone: parse_json_column(row.try_get("onclone").map_err(storage)?)?,
        on_pull: parse_json_column(row.try_get("onpull").map_err(storage)?)?,
        resolved_commit_sha: commit.map(str::to_owned),
    })
}

fn mask_webhook(value: Option<Value>) -> Option<Value> {
    value.map(|mut webhook| {
        if let Some(object) = webhook.as_object_mut()
            && object.get("secret").is_some_and(|value| !value.is_null())
        {
            object.insert("secret".to_owned(), Value::String("********".to_owned()));
        }
        webhook
    })
}

fn parse_json_column(value: Option<String>) -> Result<Option<Value>, GitRepositoryExecutionError> {
    value
        .map(|value| {
            serde_json::from_str(&value)
                .map_err(|error| GitRepositoryExecutionError::Storage(error.to_string()))
        })
        .transpose()
}

async fn repository_exists(pool: &PgPool, id: Uuid) -> Result<bool, GitRepositoryExecutionError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM gitrepositories WHERE id=$1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(storage)
}

fn is_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn storage(error: sqlx::Error) -> GitRepositoryExecutionError {
    GitRepositoryExecutionError::Storage(error.to_string())
}
