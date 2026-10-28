#[derive(Debug, Clone, thiserror::Error)]
pub enum BuildError {
    #[error("This Build operation requires license capability {0:?}.")]
    LicenseRequired(citadel_licensing::LicenseCapability),
    #[error("{0}")]
    Validation(String),
    #[error("Administrator access is required.")]
    Forbidden,
    #[error("Build resource was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("storage failed: {0}")]
    Storage(String),
}
