#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("Automated operations require an active license entitlement.")]
    LicenseRequired,
    #[error("{0}")]
    Validation(String),
    #[error("Backup resource was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("backup storage failed: {0}")]
    Storage(String),
    #[error("external Backup operation failed: {0}")]
    External(String),
}
