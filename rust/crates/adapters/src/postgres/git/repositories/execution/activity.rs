use super::*;

pub(super) async fn repository_snapshot(
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
