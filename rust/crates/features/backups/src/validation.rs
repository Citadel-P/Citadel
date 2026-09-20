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

pub(crate) fn discriminator(value: &Value) -> Result<&str, BackupError> {
    value.get("$type").and_then(Value::as_str).ok_or_else(|| {
        BackupError::Validation("Polymorphic Backup input requires a $type discriminator.".into())
    })
}

pub(crate) fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, BackupError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| BackupError::Validation(format!("Backup field '{key}' is required.")))
}

pub(crate) fn required_uuid(value: &Value, key: &str) -> Result<Uuid, BackupError> {
    required_string(value, key).and_then(|v| {
        Uuid::parse_str(v)
            .map_err(|_| BackupError::Validation(format!("Backup field '{key}' must be a UUID.")))
    })
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
