use super::*;
pub(super) async fn validate_git_account(
    transaction: &mut Transaction<'_, Postgres>,
    id: Option<Uuid>,
    repository_url: &str,
) -> Result<(), GitRepositoryError> {
    let Some(id) = id else {
        return validate_direct_git_url(repository_url);
    };
    let domain = sqlx::query_scalar::<_, String>("SELECT domain FROM gitaccounts WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?
        .ok_or(GitRepositoryError::NotFound)?;
    if is_account_relative_git_path(repository_url) {
        return Ok(());
    }
    let repository_domain = extract_git_domain(repository_url)?;
    if repository_domain.eq_ignore_ascii_case(domain.trim().trim_end_matches('/')) {
        Ok(())
    } else {
        Err(GitRepositoryError::Validation(format!(
            "Repository URL domain '{repository_domain}' does not match linked GitAccount domain '{}'.",
            domain.trim().trim_end_matches('/')
        )))
    }
}

pub(super) fn is_account_relative_git_path(repository_url: &str) -> bool {
    let value = repository_url.trim();
    !value.is_empty()
        && !value.starts_with('-')
        && !value.contains(['\r', '\n', '\0'])
        && !value.starts_with('/')
        && !value.contains("..")
        && !value.contains("://")
        && !value.starts_with("git@")
}

pub(super) fn validate_direct_git_url(repository_url: &str) -> Result<(), GitRepositoryError> {
    let value = repository_url.trim();
    let valid = url::Url::parse(value).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https" | "file")
            && (url.scheme() == "file" || url.host_str().is_some())
    }) || value.strip_prefix("git@").is_some_and(|value| {
        value
            .split_once(':')
            .is_some_and(|(host, path)| !host.is_empty() && !path.is_empty())
    });
    if valid {
        Ok(())
    } else {
        Err(GitRepositoryError::Validation(
            "Enter a complete repository URL when no Git account is selected.".to_owned(),
        ))
    }
}

pub(super) fn extract_git_domain(repository_url: &str) -> Result<String, GitRepositoryError> {
    let value = repository_url.trim();
    if let Ok(url) = url::Url::parse(value)
        && let Some(host) = url.host_str()
    {
        return Ok(host.to_owned());
    }
    if let Some(value) = value.strip_prefix("git@")
        && let Some((host, _)) = value.split_once(':')
    {
        return Ok(host.to_owned());
    }
    let authority = value.split_once('/').map_or(value, |(host, _)| host);
    let host = authority
        .split_once(':')
        .map_or(authority, |(host, _)| host)
        .trim();
    if host.is_empty() {
        Err(GitRepositoryError::Validation(
            "Repository URL must include a host.".to_owned(),
        ))
    } else {
        Ok(host.to_owned())
    }
}
