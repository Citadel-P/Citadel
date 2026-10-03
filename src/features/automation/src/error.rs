#[derive(Debug, thiserror::Error)]
pub enum AutomationError {
    #[error("Automated operations require an active license entitlement.")]
    LicenseRequired,
    #[error("{0}")]
    Validation(String),
    #[error("Automation Action was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("storage failed: {0}")]
    Storage(String),
    #[error("external operation failed: {0}")]
    External(String),
}
