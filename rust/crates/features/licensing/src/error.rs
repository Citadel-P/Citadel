#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("resource conflict: {0}")]
    Conflict(String),
    #[error("The submitted license does not replace the currently installed license.")]
    ReplacementMismatch,
    #[error("A future-dated license cannot replace a currently active license.")]
    ReplacementNotYetEffective,
    #[error("license storage failed: {0}")]
    Storage(String),
    #[error("license verification key is invalid")]
    InvalidKey,
}
