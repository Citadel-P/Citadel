use super::*;
use citadel_git::{GitRepositoryRefStatus, GitRepositoryStatus};

pub(super) async fn get_source(
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

pub(super) fn map_source(
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

pub(super) fn map_ref(
    row: sqlx::postgres::PgRow,
) -> Result<GitRepositoryRef, GitRepositoryExecutionError> {
    Ok(GitRepositoryRef {
        id: row.try_get("id").map_err(storage)?,
        git_repository_id: row.try_get("gitrepositoryid").map_err(storage)?,
        branch: row.try_get("branch").map_err(storage)?,
        resolved_commit_sha: row.try_get("resolvedcommitsha").map_err(storage)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        last_error: row.try_get("lasterror").map_err(storage)?,
        last_synced_at: row.try_get("lastsyncedat").map_err(storage)?,
    })
}

pub(super) async fn upsert_pending_ref(
    transaction: &mut Transaction<'_, Postgres>,
    repository_id: Uuid,
    branch: &str,
    trigger: &str,
) -> Result<(), GitRepositoryExecutionError> {
    sqlx::query(
        r#"
INSERT INTO gitrepositoryrefs(id,gitrepositoryid,branch,resolvedcommitsha,status,lasterror,lastsyncedat,synctrigger)
VALUES($1,$2,$3,NULL,'Pending',NULL,CURRENT_TIMESTAMP,$4)
ON CONFLICT(gitrepositoryid,branch) DO UPDATE
SET status='Pending', synctrigger=EXCLUDED.synctrigger
"#,
    )
    .bind(Uuid::now_v7())
    .bind(repository_id)
    .bind(branch)
    .bind(trigger)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

pub(super) async fn finish_sync(
    pool: &PgPool,
    claim: &GitSyncClaim,
    result: Option<&SyncResult>,
    error: Option<&str>,
) -> Result<(), GitRepositoryExecutionError> {
    let mut transaction = pool.begin().await.map_err(storage)?;
    let rerun_requested = sqlx::query_scalar::<_, bool>(
        "SELECT controlstate='Queued' FROM gitrepositories WHERE id=$1 FOR NO KEY UPDATE",
    )
    .bind(claim.repository.id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(storage)?;
    let (status, commit) = if let Some(result) = result {
        (
            GitRepositoryRefStatus::Healthy,
            Some(result.commit.as_str()),
        )
    } else {
        (GitRepositoryRefStatus::Degraded, None)
    };
    let affected = sqlx::query(
        "UPDATE gitrepositoryrefs SET status=$2,resolvedcommitsha=COALESCE($3,resolvedcommitsha),lasterror=$4,lastsyncedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Syncing'",
    )
    .bind(claim.reference_id)
    .bind(status.as_str())
    .bind(commit)
    .bind(error)
    .execute(&mut *transaction)
    .await
    .map_err(storage)?
    .rows_affected();
    if affected != 1 {
        return Err(GitRepositoryExecutionError::Conflict);
    }
    if rerun_requested {
        sqlx::query("UPDATE gitrepositoryrefs SET status='Pending' WHERE id=$1")
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
    let primary = claim.trigger != "Poll"
        || claim
            .branch
            .eq_ignore_ascii_case(&claim.repository.default_branch);
    let repository_status = if !primary && !pending {
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM gitrepositoryrefs WHERE gitrepositoryid=$1 AND branch=$2",
        )
        .bind(claim.repository.id)
        .bind(&claim.repository.default_branch)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(storage)?
        .map(|value| value.parse::<GitRepositoryRefStatus>())
        .transpose()
        .map_err(storage)?
        .map(GitRepositoryStatus::from)
        .unwrap_or(GitRepositoryStatus::Unknown)
    } else {
        GitRepositoryStatus::from(status)
    };
    sqlx::query(
        "UPDATE gitrepositories SET status=$2,controlstate=$3,controlstartedat=NULL,controltriggeredby=CASE WHEN $4 THEN controltriggeredby ELSE NULL END,rowversion=rowversion+1 WHERE id=$1",
    )
    .bind(claim.repository.id)
    .bind(if pending { GitRepositoryStatus::Pending } else { repository_status }.as_str())
    .bind(if pending { "Queued" } else { "Idle" })
    .bind(pending)
    .execute(&mut *transaction)
    .await
    .map_err(storage)?;
    let record_activity = if claim.trigger != "Poll" {
        true
    } else if let Some(result) = result {
        result.cloned
            // Recovery is meaningful even when the remote commit did not change.
            || claim.previous_error.is_some()
            || !claim
                .previous_commit
                .as_deref()
                .is_some_and(|previous| previous.eq_ignore_ascii_case(&result.commit))
    } else {
        claim.previous_error.as_deref() != error
    };
    if record_activity {
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
    }
    if claim.trigger == "Webhook"
        && let Some(error) = error
    {
        let reason = error
            .chars()
            .filter(|c| !c.is_control())
            .take(256)
            .collect::<String>();
        let branch = claim
            .branch
            .chars()
            .filter(|c| !c.is_control())
            .take(100)
            .collect::<String>();
        let observation = citadel_alerts::AlertObservation {
            alert_type: "WebhookGitRepoSyncFailed".into(),
            info: serde_json::json!({"GitRepositoryName":claim.repository.name,"Reason":reason,"Branch":branch,"HumanMessage":format!("Webhook-triggered sync failed for Git repository '{}' on branch '{}': {}",claim.repository.name,branch,reason)}),
            resource_id: claim.repository.id,
            resource_name: claim.repository.name.clone(),
            resource_type: "Webhook".into(),
            deduplication_component: format!("{branch}:{reason}"),
            observed_at: Utc::now(),
            value: None,
            matched: true,
        };
        crate::persistence::postgres::alerts::observations::enqueue(&mut transaction, &observation)
            .await
            .map_err(storage)?;
    }
    transaction.commit().await.map_err(storage)
}

pub(super) fn parse_json_column(
    value: Option<String>,
) -> Result<Option<Value>, GitRepositoryExecutionError> {
    value
        .map(|value| {
            serde_json::from_str(&value)
                .map_err(|error| GitRepositoryExecutionError::Storage(error.to_string()))
        })
        .transpose()
}

pub(super) async fn repository_exists(
    pool: &PgPool,
    id: Uuid,
) -> Result<bool, GitRepositoryExecutionError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM gitrepositories WHERE id=$1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(storage)
}

pub(super) fn is_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn storage(error: impl std::fmt::Display) -> GitRepositoryExecutionError {
    GitRepositoryExecutionError::Storage(error.to_string())
}
