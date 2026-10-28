#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    #[error("Automated operations require an active license entitlement.")]
    LicenseRequired,
    #[error("{0}")]
    Validation(String),
    #[error("Script was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("storage failed: {0}")]
    Storage(String),
    #[error("external operation failed: {0}")]
    External(String),
}
