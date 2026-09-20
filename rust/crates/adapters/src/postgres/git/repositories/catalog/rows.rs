use super::*;

pub(super) fn map_git_repository(row: PgRow) -> Result<GitRepository, GitRepositoryError> {
    Ok(GitRepository {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        url: row.try_get("url").map_err(storage)?,
        default_branch: row.try_get("defaultbranch").map_err(storage)?,
        git_account_id: row.try_get("gitaccountid").map_err(storage)?,
        sync_mode: parse_git_sync_mode(&row.try_get::<String, _>("syncmode").map_err(storage)?)?,
        sync_interval_minutes: row.try_get("syncintervalminutes").map_err(storage)?,
        webhook: row.try_get("webhook").map_err(storage)?,
        on_clone: deserialize_optional(row.try_get("onclone").map_err(storage)?)?,
        on_pull: deserialize_optional(row.try_get("onpull").map_err(storage)?)?,
        status: row.try_get("status").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        control_state: row
            .try_get::<Option<String>, _>("controlstate")
            .map_err(storage)?
            .unwrap_or_else(|| "Idle".to_owned()),
        latest_activity: row.try_get("latest_activity").map_err(storage)?,
        tags: serde_json::from_value(row.try_get("tags").map_err(storage)?).map_err(storage)?,
    })
}

pub(super) async fn get_git_repository(
    pool: &PgPool,
    id: Uuid,
) -> Result<GitRepository, GitRepositoryError> {
    let q = format!(
        "SELECT resource.*, {TAG_SUMMARIES}, {LATEST_GIT_ACTIVITY} FROM gitrepositories resource WHERE resource.id=$2"
    );
    sqlx::query(AssertSqlSafe(q.as_str()))
        .bind(TaggableResourceType::GitRepository.as_database_str())
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(GitRepositoryError::NotFound)
        .and_then(map_git_repository)
}

pub(super) async fn get_git_repository_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    lock: bool,
) -> Result<GitRepository, GitRepositoryError> {
    let lock_clause = if lock { " FOR UPDATE OF resource" } else { "" };
    let query = format!(
        "SELECT resource.*, {TAG_SUMMARIES}, {LATEST_GIT_ACTIVITY} FROM gitrepositories resource WHERE resource.id=$2{lock_clause}"
    );
    sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(TaggableResourceType::GitRepository.as_database_str())
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(GitRepositoryError::NotFound)
        .and_then(map_git_repository)
}

pub(super) async fn load_git_repositories_tx(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<Vec<GitRepository>, GitRepositoryError> {
    let q = format!(
        "SELECT resource.*, {TAG_SUMMARIES}, NULL::jsonb AS latest_activity FROM gitrepositories resource WHERE resource.id=ANY($2::uuid[]) ORDER BY resource.id"
    );
    sqlx::query(AssertSqlSafe(q.as_str()))
        .bind(TaggableResourceType::GitRepository.as_database_str())
        .bind(ids)
        .fetch_all(&mut **tx)
        .await
        .map_err(storage)?
        .into_iter()
        .map(map_git_repository)
        .collect()
}

pub(super) fn git_snapshot(
    value: &GitRepository,
) -> Result<GitRepositoryActivitySnapshot, GitRepositoryError> {
    Ok(GitRepositoryActivitySnapshot {
        id: value.id,
        name: value.name.clone(),
        description: value.description.clone(),
        url: value.url.clone(),
        default_branch: value.default_branch.clone(),
        git_account_id: value.git_account_id,
        sync_mode: value.sync_mode.as_database_str().to_owned(),
        sync_interval_minutes: value.sync_interval_minutes,
        webhook: value.webhook.as_ref().map(mask_webhook),
        on_clone: serialize_activity_value(value.on_clone.as_ref())?,
        on_pull: serialize_activity_value(value.on_pull.as_ref())?,
        resolved_commit_sha: None,
    })
}

pub(super) fn parse_git_sync_mode(v: &str) -> Result<GitRepositorySyncMode, GitRepositoryError> {
    match v {
        "Manual" => Ok(GitRepositorySyncMode::Manual),
        "PullInterval" => Ok(GitRepositorySyncMode::PullInterval),
        _ => Err(GitRepositoryError::Storage(format!(
            "Unknown Git repository sync mode '{v}'."
        ))),
    }
}
