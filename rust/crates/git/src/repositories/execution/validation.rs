use super::*;
pub(super) fn validate_repository_path(
    path: &str,
    require_value: bool,
) -> Result<String, GitRepositoryExecutionError> {
    let path = path.trim();
    if path.chars().count() > 4_096 || path.len() > 16 * 1_024 || path.contains(['\0', '\\']) {
        return Err(GitRepositoryExecutionError::Validation(
            "Repository path is invalid or too long.".to_owned(),
        ));
    }
    let normalized = path.trim_start_matches("./").trim_matches('/');
    if (require_value && normalized.is_empty())
        || path.starts_with('/')
        || (!normalized.is_empty()
            && (normalized
                .split('/')
                .any(|component| component.is_empty() || matches!(component, "." | ".."))
                || normalized
                    .split('/')
                    .next()
                    .is_some_and(|component| component.eq_ignore_ascii_case(".git"))))
    {
        return Err(GitRepositoryExecutionError::Validation(
            "Repository path must be a relative path inside the selected Git tree.".to_owned(),
        ));
    }
    Ok(normalized.to_owned())
}

pub(super) fn is_full_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn storage_error(error: std::io::Error) -> GitRepositoryExecutionError {
    GitRepositoryExecutionError::Storage(error.to_string())
}

pub(super) fn validate_branch_input(branch: &str) -> Result<(), GitRepositoryExecutionError> {
    let branch = branch.trim();
    if branch.is_empty()
        || branch.len() > 255
        || branch.starts_with('-')
        || branch.contains(['\r', '\n'])
    {
        Err(GitRepositoryExecutionError::Validation(
            "Git branch name is invalid.".to_owned(),
        ))
    } else {
        Ok(())
    }
}

pub(super) fn bounded_error(message: &str) -> String {
    const MAX_ERROR_BYTES: usize = 4096;
    if message.len() <= MAX_ERROR_BYTES {
        return message.to_owned();
    }
    let mut end = MAX_ERROR_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message[..end].to_owned()
}

pub(super) fn map_webhook_error(error: WebhookError) -> GitRepositoryExecutionError {
    match error {
        WebhookError::Authentication => GitRepositoryExecutionError::Authentication,
        WebhookError::Validation(message) => GitRepositoryExecutionError::Validation(message),
    }
}
