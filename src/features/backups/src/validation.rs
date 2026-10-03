use crate::*;

pub(crate) fn validate_name(value: &mut String, label: &str) -> Result<(), BackupError> {
    *value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > 128 {
        Err(BackupError::Validation(format!(
            "{label} name must contain between 1 and 128 characters."
        )))
    } else {
        Ok(())
    }
}

pub(crate) fn reqwest_url(value: &str) -> Result<url::Url, BackupError> {
    let url = url::Url::parse(value)
        .map_err(|_| BackupError::Validation("S3 endpoint is invalid.".into()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        Err(BackupError::Validation("S3 endpoint is invalid.".into()))
    } else {
        Ok(url)
    }
}
