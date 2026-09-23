#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("one or more validation errors occurred")]
    FieldValidation(std::collections::BTreeMap<String, Vec<String>>),
    #[error("Cooldown must be between 10s and 24h. (Parameter 'cooldownSeconds')")]
    InvalidCooldown,
    #[error("The provided alert rule does not exist")]
    RuleNotFound,
    #[error("Advanced alerting requires a license")]
    LicenseRequired,
    #[error("{0}")]
    Validation(String),
    #[error("Alert resource was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("alert storage failed: {0}")]
    Storage(String),
    #[error("alert delivery failed: {0}")]
    Delivery(String),
}
