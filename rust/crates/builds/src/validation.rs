use crate::*;

pub(crate) fn normalize_required(
    value: Option<String>,
    fallback: &str,
) -> Result<String, BuildError> {
    let value = value
        .unwrap_or_else(|| fallback.to_owned())
        .trim()
        .to_owned();
    if value.is_empty() || value.len() > 512 {
        Err(BuildError::Validation(
            "Build path or branch is invalid.".to_owned(),
        ))
    } else {
        Ok(value)
    }
}

pub(crate) fn valid_git_branch(branch: &str) -> bool {
    !branch.is_empty()
        && branch.len() <= 255
        && !branch.starts_with('-')
        && !branch.starts_with('.')
        && !branch.ends_with('.')
        && !branch.ends_with('/')
        && !branch.contains("..")
        && !branch.contains("@{")
        && !branch.contains([' ', '~', '^', ':', '?', '*', '[', '\\', '\r', '\n'])
}

pub(crate) fn normalize_path(value: Option<String>, fallback: &str) -> Result<String, BuildError> {
    let value = normalize_required(value, fallback)?.replace('\\', "/");
    if value.starts_with('/') || value.as_bytes().get(1) == Some(&b':') {
        return Err(BuildError::Validation(
            "Build paths must be relative to the repository.".into(),
        ));
    }
    if value.split('/').any(|part| matches!(part, ".." | ".git")) {
        return Err(BuildError::Validation(
            "Build paths cannot traverse or access Git metadata.".to_owned(),
        ));
    }
    Ok(value)
}

pub(crate) fn validate_range(
    value: i32,
    min: i32,
    max: i32,
    label: &str,
) -> Result<(), BuildError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(BuildError::Validation(format!(
            "Build Agent Pool {label} is out of range."
        )))
    }
}
